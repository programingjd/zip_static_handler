use crate::handler::Handler;
use crate::http::headers::Line;
use crate::http::request::Request;
use crate::http::response::{Builder, StatusCode};
use crate::http::OwnedOrStatic;
use rocket::http::uri::Path;
use rocket::http::{Header, Status};
use rocket::response::Builder as ResponseBuilder;
use rocket::route::Handler as RocketHandler;
use rocket::route::Outcome;
use rocket::serde::__private::from_utf8_lossy;
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
            .unwrap()
    }
}

struct RequestAdapter<'a, 'b> {
    inner: &'a RocketRequest<'b>,
    path: Path<'a>,
}

struct ResponseBuilderAdapter<'a> {
    inner: ResponseBuilder<'a>,
}

impl<'a, 'b> Request<Outcome<'a>, ResponseBuilderAdapter<'a>> for RequestAdapter<'a, 'b> {
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

    fn response_builder_with_status(self, code: StatusCode) -> ResponseBuilderAdapter<'a> {
        let code: u16 = code.into();
        let mut builder = Response::build();
        builder.status(Status::new(code));
        ResponseBuilderAdapter { inner: builder }
    }
}

impl<'a> Builder<Outcome<'a>> for ResponseBuilderAdapter<'a> {
    fn build(
        self,
        headers: impl Iterator<Item = impl AsRef<Line>>,
        body: Option<impl AsRef<[u8]> + Send>,
    ) -> crate::errors::Result<Outcome<'a>> {
        let mut inner = self.inner;
        headers.for_each(|ref line| {
            let line = line.as_ref().clone();
            inner.header(Header::new(
                from_utf8_lossy(line.key),
                match line.value {
                    OwnedOrStatic::Owned(vec) => Cow::Owned(from_utf8_lossy(&vec).to_string()),
                    OwnedOrStatic::Static(slice) => from_utf8_lossy(slice),
                },
            ));
        });
        if let Some(bytes) = body {
            let bytes = bytes.as_ref();
            let len = bytes.len();
            inner.sized_body(Some(len), Cursor::new(bytes.to_vec()));
        }
        Ok(Outcome::Success(inner.finalize()))
    }
}
