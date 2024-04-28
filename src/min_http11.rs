use crate::handler::{Entry, Handler};
use crate::http::headers::{Line, LOCATION};
use crate::http::response::StatusCode;
use min_http11_parser::error::Error;
use min_http11_parser::method::Method;
use min_http11_parser::parser::Parser;
use tokio::io::{AsyncBufRead, AsyncWrite, AsyncWriteExt};

pub struct Accepted<'a>(&'a Entry);

impl Handler {
    pub async fn read_request_line<'a, R: AsyncBufRead + Unpin, W: AsyncWrite + Unpin>(
        &self,
        parser: &Parser,
        reader: &mut R,
        writer: &mut W,
        buffer: &'a mut Vec<u8>,
    ) -> Option<(Method, &'a [u8])> {
        match parser.parse_request_line(reader, buffer).await {
            Err(Error::ReadTimeout) => None,
            Err(Error::UnsupportedVersion(_)) => None,
            Err(Error::UnexpectedEndOfFile) => None,
            Err(Error::RequestTooLarge) => {
                Self::write_status_line(writer, StatusCode::RequestTooLarge).await?;
                Self::write_headers(writer, self.error_headers.iter(), true).await?;
                None
            }
            Err(Error::UnknownMethod(_)) => None,
            Err(Error::BadRequest) => {
                Self::write_status_line(writer, StatusCode::BadRequest).await?;
                Self::write_headers(writer, self.error_headers.iter(), true).await?;
                None
            }
            Err(_) => unimplemented!(),
            Ok(it) => Some(it),
        }
    }

    pub fn accept(&self, path: &str) -> Option<Accepted> {
        self.paths.get(path).map(Accepted)
    }

    pub async fn handle_not_found<R: AsyncBufRead + Unpin, W: AsyncWrite + Unpin>(
        &self,
        method: &Method,
        parser: &Parser,
        reader: &mut R,
        writer: &mut W,
        buffer: &mut Vec<u8>,
    ) -> Option<()> {
        match method {
            Method::Head | Method::Get => {}
            _ => {
                Self::write_status_line(writer, StatusCode::BadRequest).await?;
                self.write_error_headers(writer, true).await?;
                return None;
            }
        }
        let known_headers = match parser.parse_headers(reader, buffer).await {
            Err(Error::ReadTimeout) => return None,
            Err(Error::RequestTooLarge) => {
                Self::write_status_line(writer, StatusCode::RequestTooLarge).await?;
                self.write_error_headers(writer, true).await?;
                return None;
            }
            Err(Error::BadRequest) => {
                Self::write_status_line(writer, StatusCode::BadRequest).await?;
                self.write_error_headers(writer, true).await?;
                return None;
            }
            Err(_) => unimplemented!(),
            Ok((known_headers, _)) => known_headers,
        };
        if let Some(value) = known_headers.content_length {
            if value != b"0" {
                Self::write_status_line(writer, StatusCode::BadRequest).await?;
                self.write_error_headers(writer, true).await?;
                return None;
            }
        }
        Self::write_status_line(writer, StatusCode::NotFound).await?;
        self.write_error_headers(writer, true).await?;
        Some(())
    }

    pub async fn handle_path<R: AsyncBufRead + Unpin, W: AsyncWrite + Unpin>(
        &self,
        method: &Method,
        accepted: Accepted<'_>,
        parser: &Parser,
        reader: &mut R,
        writer: &mut W,
        buffer: &mut Vec<u8>,
    ) -> Option<()> {
        let entry = accepted.0;
        match method {
            Method::Head | Method::Get => {}
            _ => {
                Self::write_status_line(writer, StatusCode::BadRequest).await?;
                self.write_error_headers(writer, true).await?;
                return None;
            }
        }
        let known_headers = match parser.parse_headers(reader, buffer).await {
            Err(Error::ReadTimeout) => return None,
            Err(Error::RequestTooLarge) => {
                Self::write_status_line(writer, StatusCode::RequestTooLarge).await?;
                self.write_error_headers(writer, true).await?;
                return None;
            }
            Err(_) => {
                Self::write_status_line(writer, StatusCode::BadRequest).await?;
                self.write_error_headers(writer, true).await?;
                return None;
            }
            Ok((known_headers, _)) => known_headers,
        };
        if let Some(value) = known_headers.content_length {
            if value != b"0" {
                Self::write_status_line(writer, StatusCode::BadRequest).await?;
                self.write_error_headers(writer, true).await?;
                return None;
            }
        }
        let is_get = match method {
            Method::Get => true,
            Method::Head => false,
            _ => {
                Self::write_status_line(writer, StatusCode::BadRequest).await?;
                self.write_error_headers(writer, true).await?;
                return None;
            }
        };
        let headers = &entry.headers;
        if entry.etag.is_some() {
            let etag = entry.etag.as_ref().map(|it| it.as_bytes());
            let none_match = known_headers.if_none_match;
            let if_match = known_headers.if_match;
            if none_match.is_some() && none_match == etag {
                Self::write_status_line(writer, StatusCode::NotModified).await?;
                Self::write_headers(writer, headers.iter(), false).await?;
            } else if if_match.is_some() && if_match != etag {
                Self::write_status_line(writer, StatusCode::PreconditionFailed).await?;
                Self::write_headers(writer, headers.iter(), false).await?;
            } else if let Some(ref body) = entry.content {
                Self::write_status_line(writer, StatusCode::OK).await?;
                Self::write_headers(writer, headers.iter(), false).await?;
                if is_get {
                    Self::write_body(writer, body).await?;
                }
            } else if headers.iter().any(|it| it.key == LOCATION) {
                Self::write_status_line(writer, StatusCode::TemporaryRedirect).await?;
                Self::write_headers(writer, headers.iter(), false).await?;
            } else {
                Self::write_status_line(writer, StatusCode::NoContent).await?;
                Self::write_headers(writer, headers.iter(), false).await?;
            }
        } else {
            Self::write_status_line(writer, StatusCode::PermanentRedirect).await?;
            Self::write_headers(writer, headers.iter(), false).await?;
        }
        Some(())
    }

    pub async fn write_status_line<T: AsyncWrite + Unpin>(
        writer: &mut T,
        code: StatusCode,
    ) -> Option<()> {
        writer
            .write_all(match code {
                StatusCode::OK => b"HTTP/1.1 200 OK\r\n",
                StatusCode::NoContent => b"HTTP/1.1 204 No Content\r\n",
                StatusCode::NotModified => b"HTTP/1.1 304 Not Modified\r\n",
                StatusCode::TemporaryRedirect => b"HTTP/1.1 307 Temporary Redirect\r\n",
                StatusCode::PermanentRedirect => b"HTTP/1.1 308 Permanent Redirect\r\n",
                StatusCode::BadRequest => b"HTTP/1.1 400 Bad Request\r\n",
                StatusCode::Unauthorized => b"HTTP/1.1 401 Unauthorized\r\n",
                StatusCode::Forbidden => b"HTTP/1.1 403 Forbidden\r\n",
                StatusCode::NotFound => b"HTTP/1.1 404 Not Found\r\n",
                StatusCode::MethodNotAllowed => b"HTTP/1.1 405 Method Not Allowed\r\n",
                StatusCode::PreconditionFailed => b"HTTP/1.1 412 Precondition Failed\r\n",
                StatusCode::RequestTooLarge => b"HTTP/1.1 413 Request Too Large\r\n",
                StatusCode::InternalServerError => b"HTTP/1.1 500 Internal Server Error\r\n",
            })
            .await
            .ok()
    }

    pub async fn write_error_headers<T: AsyncWrite + Unpin>(
        &self,
        writer: &mut T,
        close: bool,
    ) -> Option<()> {
        Self::write_headers(writer, self.error_headers.iter(), close).await
    }

    pub async fn write_headers<T: AsyncWrite + Unpin>(
        writer: &mut T,
        headers: impl Iterator<Item = &Line>,
        close: bool,
    ) -> Option<()> {
        for line in headers {
            writer.write_all(line.key).await.ok()?;
            writer.write_all(b": ").await.ok()?;
            writer.write_all(line.value.as_ref()).await.ok()?;
            writer.write_all(b"\r\n").await.ok()?;
        }
        if close {
            writer.write_all(b"connection: close\r\n\r\n").await.ok()
        } else {
            writer
                .write_all(b"connection: keep-alive\r\nkeep-alive: 60\r\n\r\n")
                .await
                .ok()
        }
    }

    pub async fn write_body<T: AsyncWrite + Unpin>(
        writer: &mut T,
        body: impl AsRef<[u8]>,
    ) -> Option<()> {
        writer.write_all(body.as_ref()).await.ok()
    }
}
