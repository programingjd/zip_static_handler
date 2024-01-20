use crate::errors::{Error, Result};
use crate::handler::Handler;
use crate::http::headers::Line;
use crate::http::request::Request;
use crate::http::response::{Builder, StatusCode};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::http;
use hyper::http::{HeaderName, HeaderValue};
use std::fmt::{Display, Formatter};
use std::str::from_utf8;

type HyperResponse = hyper::Response<BoxBody<Bytes, hyper::Error>>;
type HyperRequest = hyper::Request<hyper::body::Incoming>;

impl Handler {
    pub fn handle_request(
        &self,
        request: HyperRequest,
    ) -> std::result::Result<HyperResponse, Error> {
        self.handle(RequestAdapter { inner: request })
    }
}

impl Error {
    pub fn boxed(self) -> Box<impl std::error::Error + Send + Sync> {
        Box::new(ErrorAdapter {
            message: self.to_string(),
        })
    }
}

#[derive(Debug)]
struct ErrorAdapter {
    message: String,
}

impl Display for ErrorAdapter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ErrorAdapter {}

struct RequestAdapter {
    inner: HyperRequest,
}

struct ResponseBuilderAdapter {
    inner: http::response::Builder,
}

impl Request<HyperResponse, ResponseBuilderAdapter> for RequestAdapter {
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

    fn response_builder_with_status(code: StatusCode) -> ResponseBuilderAdapter {
        let code: u16 = code.into();
        ResponseBuilderAdapter {
            inner: hyper::Response::builder().status(code),
        }
    }
}

impl ResponseBuilderAdapter {
    fn full(slice: &[u8]) -> BoxBody<Bytes, hyper::Error> {
        Full::new(Bytes::copy_from_slice(slice))
            .map_err(|never| match never {})
            .boxed()
    }
    fn empty() -> BoxBody<Bytes, hyper::Error> {
        Empty::<Bytes>::new()
            .map_err(|never| match never {})
            .boxed()
    }
}

impl Builder<HyperResponse> for ResponseBuilderAdapter {
    fn append_headers(self, headers: impl Iterator<Item = impl AsRef<Line>>) -> Self {
        let mut inner = self.inner;
        let map = inner.headers_mut().unwrap();
        headers.for_each(|ref line| {
            let line = line.as_ref();
            if let Ok(name) = HeaderName::from_bytes(line.key) {
                if let Ok(value) = HeaderValue::from_bytes(line.value.as_ref()) {
                    map.append(name, value);
                }
            }
        });
        Self { inner }
    }

    fn with_body(self, body: Option<&[u8]>) -> Result<HyperResponse> {
        let body = body.map(Self::full).unwrap_or_else(Self::empty);
        self.inner
            .body(body)
            .map_err(|it| Error::Wrapped(Box::new(it)))
    }
}
