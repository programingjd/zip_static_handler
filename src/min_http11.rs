use crate::handler::Handler;
use crate::http::headers::Line;
use crate::http::request::Request;
use crate::http::response::StatusCode;
use min_http11_parser::method::Method;
use min_http11_parser::request::KnownHeaders;

impl Handler {
    pub fn handle_min_http11_request<'a>(&self, req: Req<'a>) -> Resp<'a> {
        self.handle(req)
    }
}

pub struct Req<'a> {
    pub buffer: &'a mut Vec<u8>,
    pub method: Method,
    pub path: &'a [u8],
    pub known_headers: KnownHeaders<'a>,
}

pub struct Resp<'a> {
    pub buffer: &'a mut Vec<u8>,
    body: Vec<u8>,
}

impl<'a> Request<Resp<'a>> for Req<'a> {
    fn method(&self) -> &[u8] {
        self.method.as_slice()
    }
    fn path(&self) -> &[u8] {
        self.path
    }
    fn first_header_value(&self, key: &'static [u8]) -> Option<&[u8]> {
        match key {
            crate::http::headers::CONTENT_LENGTH => self.known_headers.content_length,
            crate::http::headers::IF_MATCH => self.known_headers.if_match,
            crate::http::headers::IF_NONE_MATCH => self.known_headers.if_none_match,
            _ => unimplemented!(),
        }
    }
    fn response<'b>(
        self,
        code: StatusCode,
        headers: impl Iterator<Item = &'b Line>,
        body: Option<&'b [u8]>,
    ) -> Resp<'a> {
        let buffer = self.buffer;
        buffer.clear();
        buffer.extend_from_slice(match code {
            StatusCode::OK => b"HTTP/1.1 200 OK\r\n",
            StatusCode::NotModified => b"HTTP/1.1 304 Not Modified\r\n",
            StatusCode::TemporaryRedirect => b"HTTP/1.1 307 Temporary Redirect\r\n",
            StatusCode::PermanentRedirect => b"HTTP/1.1 308 Permanent Redirect\r\n",
            StatusCode::BadRequest => b"HTTP/1.1 400 Bad Request\r\n",
            StatusCode::NotFound => b"HTTP/1.1 404 Not Found\r\n",
            StatusCode::MethodNotAllowed => b"HTTP/1.1 405 Method Not Allowed\r\n",
            StatusCode::PreconditionFailed => b"HTTP/1.1 412 Precondition Failed\r\n",
            StatusCode::InternalServerError => b"HTTP/1.1 500 Internal Server Error\r\n",
        });
        for line in headers {
            buffer.extend_from_slice(line.key);
            buffer.extend_from_slice(b": ");
            buffer.extend_from_slice(line.value.as_ref());
        }
        let body = body.unwrap_or(&[]).to_vec();
        Resp { buffer, body }
    }
}
