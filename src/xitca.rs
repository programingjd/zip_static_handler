use crate::handler::Handler;
use crate::http::headers::Line;
use crate::http::request::Request;
use crate::http::response::StatusCode;
use http::{HeaderName, HeaderValue, Response};
use std::str::from_utf8;
use xitca_web::body::{BoxBody, RequestBody};
use xitca_web::bytes::Bytes;
use xitca_web::http::WebResponse;
use xitca_web::WebContext;

type HttpStatusCode = http::StatusCode;

impl Handler {
    pub fn handle_xitca_request<C>(&self, req: &WebContext<'_, C>) -> Response<BoxBody> {
        self.handle(RequestAdapter { inner: req })
    }
}

struct RequestAdapter<'a, 'r, C> {
    inner: &'a WebContext<'r, C>,
}

impl<'b, 'r, C> Request<Response<BoxBody>> for RequestAdapter<'b, 'r, C> {
    fn method(&self) -> &[u8] {
        self.inner.req().method().as_str().as_bytes()
    }

    fn path(&self) -> &[u8] {
        self.inner.req().uri().path().as_bytes()
    }

    fn first_header_value(&self, key: &'static [u8]) -> Option<&[u8]> {
        from_utf8(key)
            .ok()
            .and_then(|key| self.inner.req().headers().get(key).map(|it| it.as_bytes()))
    }

    fn response<'a>(
        self,
        code: StatusCode,
        response_headers: impl Iterator<Item = &'a Line>,
        body: Option<&'a [u8]>,
    ) -> Response<BoxBody> {
        let mut res = WebResponse::new(BoxBody::new(RequestBody::from(Bytes::copy_from_slice(
            body.unwrap_or(&[]),
        ))));
        let headers = res.headers_mut();
        headers.clear();
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
