use crate::handler::Handler;
use crate::http::headers::Line;
use crate::http::request::Request;
use crate::http::response::StatusCode;
use axum_core::response::IntoResponse;
use http::{HeaderMap, HeaderName, HeaderValue};
use std::fmt::{Display, Formatter};
use std::str::from_utf8;

type HttpStatusCode = http::StatusCode;
type AxumResponse = axum_core::response::Response;
type AxumRequest = axum_core::extract::Request;

impl Handler {
    pub fn handle_axum_request(&self, request: AxumRequest) -> AxumResponse {
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

impl Request<AxumResponse> for RequestAdapter {
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
        body: Option<&'a [u8]>,
    ) -> AxumResponse {
        let status_code: HttpStatusCode = HttpStatusCode::from_u16(code.into()).unwrap();
        let mut map = HeaderMap::new();
        headers.for_each(|ref line| {
            let line = line.as_ref();
            if let Ok(name) = HeaderName::from_bytes(line.key) {
                if let Ok(value) = HeaderValue::from_bytes(line.value.as_ref()) {
                    map.append(name, value);
                }
            }
        });
        if let Some(bytes) = body {
            (status_code, map, bytes.to_vec()).into_response()
        } else {
            (status_code, map).into_response()
        }
    }
}
