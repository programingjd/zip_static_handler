use crate::handler::Handler;
use crate::http::OwnedOrStatic;
use crate::http::headers::Line;
use crate::http::request::Request;
use crate::http::response::StatusCode;
use bytes::Bytes;
use rocket::http::uri::Path;
use rocket::http::{Header, Status};
use rocket::route::Handler as RocketHandler;
use rocket::route::Outcome;
use rocket::{Data, Request as RocketRequest, Response};
use std::borrow::Cow;
use std::io::Cursor;
use std::str::from_utf8;
use std::sync::Arc;

#[derive(Clone)]
pub struct HandlerAdapter;

#[rocket::async_trait]
impl RocketHandler for HandlerAdapter {
    async fn handle<'r>(&self, request: &'r RocketRequest<'_>, _data: Data<'r>) -> Outcome<'r> {
        let path = request.uri().path();
        request
            .rocket()
            .state::<Arc<Handler>>()
            .unwrap()
            .handle(RequestAdapter {
                inner: request,
                path,
            })
    }
}

struct RequestAdapter<'r, 'o> {
    inner: &'r RocketRequest<'o>,
    path: Path<'r>,
}

impl<'r> Request<Outcome<'r>> for RequestAdapter<'r, '_> {
    fn method(&self) -> &[u8] {
        self.inner.method().as_str().as_bytes()
    }

    fn path(&self) -> &[u8] {
        self.path.as_bytes()
    }

    fn first_header_value(&self, key: &'static [u8]) -> Option<&[u8]> {
        from_utf8(key)
            .ok()
            .and_then(|key| self.inner.headers().get_one(key).map(|it| it.as_bytes()))
    }

    fn response<'a>(
        self,
        code: StatusCode,
        headers: impl Iterator<Item = &'a Line>,
        body: Option<Bytes>,
    ) -> Outcome<'r> {
        let code: u16 = code.into();
        let mut builder = Response::build();
        builder.status(Status::new(code));
        headers.for_each(|ref line| {
            let line = line.as_ref().clone();
            builder.header(Header::new(
                String::from_utf8_lossy(line.key),
                match line.value {
                    OwnedOrStatic::Owned(vec) => {
                        Cow::Owned(String::from_utf8_lossy(&vec).to_string())
                    }
                    OwnedOrStatic::Static(slice) => String::from_utf8_lossy(slice),
                },
            ));
        });
        if let Some(bytes) = body {
            let len = bytes.len();
            builder.sized_body(Some(len), Cursor::new(bytes));
        }
        Outcome::Success(builder.finalize())
    }
}
