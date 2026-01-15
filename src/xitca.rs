use crate::handler::Handler;
use crate::http::headers::Line;
use crate::http::request::Request;
use crate::http::response::StatusCode;
use bytes::Bytes;
use std::str::from_utf8;
use xitca_http::body::ResponseBody;
use xitca_http::http;
use xitca_http::http::{HeaderName, HeaderValue, IntoResponse, Response};

type HttpStatusCode = http::StatusCode;
type XitcaRequest<E> = http::Request<http::RequestExt<E>>;

impl Handler {
    pub fn handle_xitca_request<E>(&self, req: XitcaRequest<E>) -> Response<ResponseBody> {
        self.handle(RequestAdapter { inner: req })
    }
}

struct RequestAdapter<E> {
    inner: XitcaRequest<E>,
}

impl<E> Request<Response<ResponseBody>> for RequestAdapter<E> {
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
        response_headers: impl Iterator<Item = &'a Line>,
        body: Option<Bytes>,
    ) -> Response<ResponseBody> {
        let mut res = self.inner.into_response(
            body.map(ResponseBody::bytes)
                .unwrap_or_else(ResponseBody::empty),
        );
        let headers = res.headers_mut();
        response_headers.for_each(|line| {
            headers.append(
                HeaderName::from_static(from_utf8(line.key).unwrap()),
                HeaderValue::from_bytes(line.value.as_ref()).unwrap(),
            );
        });
        *res.status_mut() = HttpStatusCode::from_u16(code.into()).unwrap();
        res
    }
}
