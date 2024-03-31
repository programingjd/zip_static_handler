use crate::handler::Handler;
use crate::http::headers::Line;
use crate::http::response::StatusCode;
use min_http11_parser::error::Error;
use min_http11_parser::method::Method;
use min_http11_parser::parser::Parser;
use tokio::io::{AsyncBufRead, AsyncWrite, AsyncWriteExt};

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
                self.write_status_line(writer, StatusCode::RequestTooLarge)
                    .await?;
                self.write_headers(writer, self.error_headers.iter(), true)
                    .await?;
                None
            }
            Err(Error::UnknownMethod(_)) => None,
            Err(Error::BadRequest) => {
                self.write_status_line(writer, StatusCode::BadRequest)
                    .await?;
                self.write_headers(writer, self.error_headers.iter(), true)
                    .await?;
                None
            }
            Err(_) => unimplemented!(),
            Ok(it) => Some(it),
        }
    }

    pub async fn handle_path<R: AsyncBufRead + Unpin, W: AsyncWrite + Unpin>(
        &self,
        method: &Method,
        path: &[u8],
        parser: &Parser,
        reader: &mut R,
        writer: &mut W,
        buffer: &mut Vec<u8>,
    ) -> Option<()> {
        match method {
            Method::Head | Method::Get => {}
            _ => {
                self.write_status_line(writer, StatusCode::BadRequest)
                    .await?;
                self.write_error_headers(writer, true).await?;
                return None;
            }
        }
        let known_headers = match parser.parse_headers(reader, buffer).await {
            Err(Error::ReadTimeout) => return None,
            Err(Error::RequestTooLarge) => {
                self.write_status_line(writer, StatusCode::RequestTooLarge)
                    .await?;
                self.write_error_headers(writer, true).await?;
                return None;
            }
            Err(Error::BadRequest) => {
                self.write_status_line(writer, StatusCode::BadRequest)
                    .await?;
                self.write_error_headers(writer, true).await?;
                return None;
            }
            Err(_) => unimplemented!(),
            Ok((known_headers, _)) => known_headers,
        };
        if let Some(value) = known_headers.content_length {
            if value != b"0" {
                self.write_status_line(writer, StatusCode::BadRequest)
                    .await?;
                self.write_error_headers(writer, true).await?;
                return None;
            }
        }
        let is_get = match method {
            Method::Get => true,
            Method::Head => false,
            _ => {
                self.write_status_line(writer, StatusCode::BadRequest)
                    .await?;
                self.write_error_headers(writer, true).await?;
                return None;
            }
        };
        let path = String::from_utf8_lossy(path);
        if let Some(file) = self.files.get(path.as_ref()) {
            let headers = &file.headers;
            if file.etag.is_some() {
                let etag = file.etag.as_ref().map(|it| it.as_bytes());
                let none_match = known_headers.if_none_match;
                let if_match = known_headers.if_match;
                if none_match.is_some() && none_match == etag {
                    self.write_status_line(writer, StatusCode::NotModified)
                        .await?;
                    self.write_headers(writer, headers.iter(), false).await?;
                } else if if_match.is_some() && if_match != etag {
                    self.write_status_line(writer, StatusCode::PreconditionFailed)
                        .await?;
                    self.write_headers(writer, headers.iter(), false).await?;
                } else {
                    self.write_status_line(writer, StatusCode::OK).await?;
                    self.write_headers(writer, headers.iter(), false).await?;
                    if is_get {
                        if let Some(ref body) = file.content {
                            self.write_body(writer, body).await?;
                        }
                    }
                }
            } else {
                self.write_status_line(writer, StatusCode::PermanentRedirect)
                    .await?;
                self.write_headers(writer, headers.iter(), false).await?;
            }
        } else {
            self.write_status_line(writer, StatusCode::NotFound).await?;
            self.write_headers(writer, self.error_headers.iter(), false)
                .await?;
        }
        Some(())
    }

    pub async fn write_status_line<T: AsyncWrite + Unpin>(
        &self,
        writer: &mut T,
        code: StatusCode,
    ) -> Option<()> {
        writer
            .write_all(match code {
                StatusCode::OK => b"HTTP/1.1 200 OK\r\n",
                StatusCode::NotModified => b"HTTP/1.1 304 Not Modified\r\n",
                StatusCode::TemporaryRedirect => b"HTTP/1.1 307 Temporary Redirect\r\n",
                StatusCode::PermanentRedirect => b"HTTP/1.1 308 Permanent Redirect\r\n",
                StatusCode::BadRequest => b"HTTP/1.1 400 Bad Request\r\n",
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
        self.write_headers(writer, self.error_headers.iter(), close)
            .await
    }

    async fn write_headers<T: AsyncWrite + Unpin>(
        &self,
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
        &self,
        writer: &mut T,
        body: impl AsRef<[u8]>,
    ) -> Option<()> {
        writer.write_all(body.as_ref()).await.ok()
    }
}
