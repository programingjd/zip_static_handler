use crate::handler::Handler;
use crate::http::headers::Line;
use crate::http::request::Request;
use crate::http::response::StatusCode;
use http_body_util::{Either, Empty, Full};
use hyper::body::Bytes;
use hyper::http::{HeaderName, HeaderValue};
use std::str::from_utf8;

type HyperResponse = hyper::Response<Either<Full<Bytes>, Empty<Bytes>>>;
type HyperRequest = hyper::Request<hyper::body::Incoming>;

impl Handler {
    pub fn handle_hyper_request(&self, request: HyperRequest) -> HyperResponse {
        self.handle(RequestAdapter { inner: request })
    }
}

struct RequestAdapter {
    inner: HyperRequest,
}

impl Request<HyperResponse> for RequestAdapter {
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
    ) -> HyperResponse {
        let code: u16 = code.into();
        let mut builder = hyper::Response::builder().status(code);
        let map = builder.headers_mut().unwrap();
        headers.for_each(|line| {
            if let Ok(name) = HeaderName::from_bytes(line.key) {
                if let Ok(value) = HeaderValue::from_bytes(line.value.as_ref()) {
                    map.append(name, value);
                }
            }
        });
        let body = body
            .map(|b| Either::Left(Full::new(b)))
            .unwrap_or_else(|| Either::Right(Empty::new()));
        builder.body(body).unwrap()
    }
}
