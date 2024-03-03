use crate::errors::Result;
use crate::handler::Handler;
use crate::http::headers::Line;
use crate::http::request::Request;
use crate::http::response::{Builder, StatusCode};
use actix_web::body::BoxBody;
use actix_web::{HttpRequest, HttpResponse, HttpResponseBuilder};
use std::str::from_utf8;

impl Handler {
    pub fn handle_actix_request(&self, request: HttpRequest) -> Result<HttpResponse<BoxBody>> {
        self.handle(RequestAdapter { inner: request })
    }
}

struct RequestAdapter {
    inner: HttpRequest,
}

struct ResponseBuilderAdapter {
    inner: HttpResponseBuilder,
}

impl Request<HttpResponse<BoxBody>, ResponseBuilderAdapter> for RequestAdapter {
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

    fn response_builder_with_status(self, code: StatusCode) -> ResponseBuilderAdapter {
        let code: u16 = code.into();
        ResponseBuilderAdapter {
            inner: HttpResponse::build(actix_web::http::StatusCode::from_u16(code).unwrap()),
        }
    }
}

impl ResponseBuilderAdapter {
    fn full(slice: impl AsRef<[u8]> + Send) -> BoxBody {
        BoxBody::new(slice.as_ref().to_vec())
    }
    fn empty() -> BoxBody {
        BoxBody::new(())
    }
}

impl Builder<HttpResponse<BoxBody>> for ResponseBuilderAdapter {
    fn build(
        self,
        headers: impl Iterator<Item = impl AsRef<Line>>,
        body: Option<impl AsRef<[u8]> + Send>,
    ) -> Result<HttpResponse<BoxBody>> {
        let mut inner = self.inner;
        headers.for_each(|ref line| {
            let line = line.as_ref();
            inner.append_header((line.key, line.value.as_ref()));
        });
        let body = body.map(Self::full).unwrap_or_else(Self::empty);
        Ok(inner.body(body))
    }
}
