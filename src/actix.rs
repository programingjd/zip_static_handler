use crate::errors::Result;
use crate::handler::Handler;
use crate::http::headers::Line;
use crate::http::request::Request;
use crate::http::response::StatusCode;
use actix_web::body::BoxBody;
use actix_web::{HttpRequest, HttpResponse};
use std::str::from_utf8;

impl Handler {
    pub fn handle_actix_request(&self, request: HttpRequest) -> Result<HttpResponse<BoxBody>> {
        self.handle(RequestAdapter { inner: request })
    }
}

struct RequestAdapter {
    inner: HttpRequest,
}

impl Request<HttpResponse<BoxBody>> for RequestAdapter {
    fn method(&self) -> &[u8] {
        self.inner.method().as_str().as_bytes()
    }

    fn path(&self) -> &[u8] {
        self.inner.uri().path().as_bytes()
    }

    fn first_header_value(&self, key: &'static [u8]) -> Option<&[u8]> {
        from_utf8(key)
            .ok()
            .and_then(|key| self.inner.headers().get(key).map(|it| it.as_bytes()))
    }

    fn response(
        self,
        code: StatusCode,
        headers: impl Iterator<Item = impl AsRef<Line>>,
        body: Option<impl AsRef<[u8]> + Send>,
    ) -> Result<HttpResponse<BoxBody>> {
        let code: u16 = code.into();
        let mut builder = HttpResponse::build(actix_web::http::StatusCode::from_u16(code).unwrap());
        headers.for_each(|ref line| {
            let line = line.as_ref();
            builder.append_header((line.key, line.value.as_ref()));
        });
        let body = body.map(Self::full).unwrap_or_else(Self::empty);
        Ok(builder.body(body))
    }
}

impl RequestAdapter {
    fn full(slice: impl AsRef<[u8]> + Send) -> BoxBody {
        BoxBody::new(slice.as_ref().to_vec())
    }
    fn empty() -> BoxBody {
        BoxBody::new(())
    }
}
