use crate::errors::Result;
use crate::handler::Handler;
use crate::http::headers::Line;
use crate::http::request::Request;
use crate::http::response::{Builder, StatusCode};
use axum_core::response::IntoResponse;
use http::{HeaderMap, HeaderName, HeaderValue};
use std::fmt::{Display, Formatter};
use std::str::from_utf8;

type HttpStatusCode = http::StatusCode;
type AxumResponse = axum_core::response::Response;
type AxumRequest = axum_core::extract::Request;

impl Handler {
    pub fn handle_request(&self, request: AxumRequest) -> Result<AxumResponse> {
        self.handle(RequestAdapter { inner: request })
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
    inner: AxumRequest,
}

struct AxumResponseBuilder {
    status_code: HttpStatusCode,
    headers: HeaderMap,
}

impl Request<AxumResponse, AxumResponseBuilder> for RequestAdapter {
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

    fn response_builder_with_status(code: StatusCode) -> AxumResponseBuilder {
        let status_code: HttpStatusCode = HttpStatusCode::from_u16(code.into()).unwrap();
        let headers = HeaderMap::new();
        AxumResponseBuilder {
            status_code,
            headers,
        }
    }
}

impl Builder<AxumResponse> for AxumResponseBuilder {
    fn append_headers(self, headers: impl Iterator<Item = impl AsRef<Line>>) -> Self {
        let status_code = self.status_code;
        let mut map = self.headers;
        headers.for_each(|ref line| {
            let line = line.as_ref();
            if let Ok(name) = HeaderName::from_bytes(line.key) {
                if let Ok(value) = HeaderValue::from_bytes(line.value.as_ref()) {
                    map.append(name, value);
                }
            }
        });
        Self {
            status_code,
            headers: map,
        }
    }

    fn with_body(self, body: Option<&[u8]>) -> Result<AxumResponse> {
        Ok(if let Some(bytes) = body {
            (self.status_code, self.headers, bytes.to_vec()).into_response()
        } else {
            (self.status_code, self.headers).into_response()
        })
    }
}
