use crate::handler::Handler;
use crate::http::headers::Line;
use crate::http::request::Request;
use crate::http::response::StatusCode;
use min_http11_parser::method::Method;
use min_http11_parser::request::KnownHeaders;

impl Handler {
    pub fn handle_min_http11_request(&self, req: Req<'_>) -> Resp {
        self.handle(req)
    }
}

pub struct Req<'a> {
    pub method: Method,
    pub path: &'a [u8],
    pub known_headers: KnownHeaders<'a>,
}

pub struct Resp {
    code: StatusCode,
    headers: Vec<Line>,
    body: Option<Box<dyn AsRef<[u8]>>>,
}

impl<'a> Request<Resp> for Req<'a> {
    fn method(&self) -> &[u8] {
        self.method.as_slice()
    }
    fn path(&self) -> &[u8] {
        self.path
    }
    fn first_header_value(&self, key: &'static [u8]) -> Option<&[u8]> {
        match key {
            CONTENT_LENGTH => self.known_headers.content_length,
            IF_MATCH => self.known_headers.if_match,
            IF_NONE_MATCH => self.known_headers.if_none_match,
            _ => unimplemented!(),
        }
    }
    fn response<'b>(
        self,
        code: StatusCode,
        headers: impl Iterator<Item = &'b Line>,
        mut body: Option<impl AsRef<[u8]> + Send>,
    ) -> Resp {
        let inner = body.map(boxed).take();
        Resp {
            code,
            headers: headers.cloned().collect(),
            body: inner,
        }
    }
}

fn boxed(unboxed: impl AsRef<[u8]>) -> Box<dyn AsRef<[u8]>> {
    Box::new(unboxed)
}
