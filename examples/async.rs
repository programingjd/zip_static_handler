use std::future::Future;
use tokio::io::{AsyncRead, AsyncWrite};
use zip_static_handler::http::headers::Line;
use zip_static_handler::http::request::Request;
use zip_static_handler::http::response::{Builder, StatusCode};

struct HttpResponse {
    boxed: Box<dyn Future<Output=()>>
}

struct HttpRequest<'a, R: AsyncRead + Unpin + Sized, W: AsyncWrite + Unpin + Sized> {
    reader: R,
    writer: W,
    method: &'a [u8],
    path: &'a [u8],
    headers: ()
}

struct HttpResponseBuilder<W: AsyncWrite + Unpin + Sized> {
    writer: W
}

impl <'a, R: AsyncRead + Unpin + Sized, W: AsyncWrite + Unpin + Sized> Request<HttpResponse, HttpResponseBuilder<W>> for HttpRequest<'a, R, W> {
    fn method(&self) -> &[u8] {
        self.method
    }

    fn path(&self) -> &[u8] {
        self.path
    }

    fn first_header_value(&self, key: &'static [u8]) -> Option<&[u8]> {
        todo!()
    }

    fn response_builder_with_status(&mut self, code: StatusCode) -> HttpResponseBuilder<W> {
        todo!()
    }
}

impl <W: AsyncWrite + Unpin + Sized> Builder<HttpResponse> for HttpResponseBuilder<W> {
    fn append_headers(self, headers: impl Iterator<Item=impl AsRef<Line>>) -> Self {
        todo!()
    }

    fn with_body(self, body: Option<&[u8]>) -> zip_static_handler::errors::Result<HttpResponse> {
        todo!()
    }
}


#[tokio::main]
async fn main() {

}
