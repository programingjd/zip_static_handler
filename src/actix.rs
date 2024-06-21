use crate::handler::Handler;
use crate::http::headers::Line;
use crate::http::request::Request;
use crate::http::response::StatusCode;
use actix_web::body::EitherBody;
use actix_web::{HttpRequest, HttpResponse};
use bytes::Bytes;
use std::str::from_utf8;

impl Handler {
    pub fn handle_actix_request(
        &self,
        request: HttpRequest,
    ) -> HttpResponse<EitherBody<Bytes, ()>> {
        self.handle(RequestAdapter { inner: request })
    }
}

struct RequestAdapter {
    inner: HttpRequest,
}

impl Request<HttpResponse<EitherBody<Bytes, ()>>> for RequestAdapter {
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

    fn response<'a>(
        self,
        code: StatusCode,
        headers: impl Iterator<Item = &'a Line>,
        body: Option<Bytes>,
    ) -> HttpResponse<EitherBody<Bytes, ()>> {
        let code: u16 = code.into();
        let mut builder = HttpResponse::build(actix_web::http::StatusCode::from_u16(code).unwrap());
        headers.for_each(|line| {
            builder.append_header((line.key, line.value.as_ref()));
        });
        let body = body
            .map(|body| EitherBody::Left { body })
            .unwrap_or_else(|| EitherBody::Right { body: () });
        builder.message_body(body).unwrap()
    }
}
