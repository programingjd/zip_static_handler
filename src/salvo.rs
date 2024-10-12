use crate::handler::Handler;
use crate::http::headers::Line;
use crate::http::request::Request;
use crate::http::response::StatusCode;
use bytes::Bytes;
use salvo::async_trait;
use salvo::http::{HeaderMap, HeaderName, HeaderValue};
use salvo::{Depot, FlowCtrl};
use std::str::from_utf8;

type HttpStatusCode = salvo::http::StatusCode;
type SalvoResponse = salvo::Response;
type SalvoRequest = salvo::Request;

struct RequestAdapter<'a> {
    request: &'a mut SalvoRequest,
    response: &'a mut SalvoResponse,
}

impl Request<()> for RequestAdapter<'_> {
    fn method(&self) -> &[u8] {
        self.request.method().as_str().as_bytes()
    }

    fn path(&self) -> &[u8] {
        self.request.uri().path().as_bytes()
    }

    fn first_header_value(&self, key: &'static [u8]) -> Option<&[u8]> {
        from_utf8(key)
            .ok()
            .and_then(|key| self.request.headers().get(key).map(|it| it.as_bytes()))
    }

    fn response<'a>(
        self,
        code: StatusCode,
        headers: impl Iterator<Item = &'a Line>,
        body: Option<Bytes>,
    ) {
        let status_code: HttpStatusCode = HttpStatusCode::from_u16(code.into()).unwrap();
        let mut map = HeaderMap::new();
        headers.for_each(|line| {
            if let Ok(name) = HeaderName::from_bytes(line.key) {
                if let Ok(value) = HeaderValue::from_bytes(line.value.as_ref()) {
                    map.append(name, value);
                }
            }
        });
        if let Some(bytes) = body {
            self.response.status_code = Some(status_code);
            self.response.set_headers(map);
            self.response.body(bytes);
        } else {
            self.response.status_code = Some(status_code);
            self.response.set_headers(map);
        }
    }
}

#[async_trait]
impl salvo::Handler for Handler {
    async fn handle(
        &self,
        request: &mut SalvoRequest,
        _depot: &mut Depot,
        response: &mut SalvoResponse,
        _ctrl: &mut FlowCtrl,
    ) {
        self.handle(RequestAdapter { request, response });
    }
}
