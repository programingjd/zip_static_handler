use crate::handler::Handler;
use crate::http::headers::Line;
use crate::http::response::StatusCode;
use min_http11_parser::error::Error;
use min_http11_parser::method::Method;
use min_http11_parser::parser::Parser;
use std::str::from_utf8;
use std::time::Duration;
use tokio::io::{copy, sink, AsyncBufRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::join;
use tokio::time::timeout;

impl Handler {
    pub async fn async_handle<R: AsyncBufRead + Unpin, W: AsyncWrite + Unpin>(
        &self,
        parser: &Parser,
        reader: &mut R,
        writer: &mut W,
        buffer: &mut Vec<u8>,
    ) -> Option<()> {
        buffer.clear();
        let (method, path, known_headers, _) = match timeout(
            Duration::from_secs(65),
            parser.parse_request_line_and_headers(reader, buffer),
        )
        .await
        {
            Err(_) => return None,
            Ok(Err(Error::ReadTimeout)) => return None,
            Ok(Err(Error::UnsupportedVersion(_))) => return None,
            Ok(Err(Error::UnexpectedEndOfFile)) => return None,
            Ok(Err(Error::RequestTooLarge)) => {
                write_status_line(writer, StatusCode::RequestTooLarge).await?;
                write_headers(writer, self.error_headers.iter(), true).await?;
                return None;
            }
            Ok(Err(Error::UnknownMethod(_))) => {
                return None;
            }
            Ok(Err(Error::BadRequest)) => {
                write_status_line(writer, StatusCode::BadRequest).await?;
                write_headers(writer, self.error_headers.iter(), true).await?;
                return None;
            }
            Ok(Err(_)) => unimplemented!(),
            Ok(Ok(it)) => it,
        };

        if let Some(value) = known_headers.content_length {
            if value != b"0" {
                let content_length = known_headers
                    .content_length
                    .and_then(|it| from_utf8(it).ok())
                    .and_then(|it| it.parse::<usize>().ok())?;
                let (r, w) = join!(
                    async {
                        copy(&mut reader.take(content_length as u64), &mut sink())
                            .await
                            .ok()
                    },
                    async {
                        write_status_line(writer, StatusCode::BadRequest).await?;
                        write_headers(writer, self.error_headers.iter(), false).await?;
                        Some(())
                    }
                );
                r?;
                w?;
            }
        }
        let is_get = match method {
            Method::Get => true,
            Method::Head => false,
            _ => {
                write_status_line(writer, StatusCode::BadRequest).await?;
                write_headers(writer, self.error_headers.iter(), false).await?;
                return Some(());
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
                    write_status_line(writer, StatusCode::NotModified).await?;
                    write_headers(writer, headers.iter(), false).await?;
                } else if if_match.is_some() && if_match != etag {
                    write_status_line(writer, StatusCode::PreconditionFailed).await?;
                    write_headers(writer, headers.iter(), false).await?;
                } else {
                    write_status_line(writer, StatusCode::OK).await?;
                    write_headers(writer, headers.iter(), false).await?;
                    if is_get {
                        if let Some(ref body) = file.content {
                            write_body(writer, body).await?;
                        }
                    }
                }
            } else {
                write_status_line(writer, StatusCode::PermanentRedirect).await?;
                write_headers(writer, headers.iter(), false).await?;
            }
        } else {
            write_status_line(writer, StatusCode::NotFound).await?;
            write_headers(writer, self.error_headers.iter(), false).await?;
        }
        Some(())
    }
}

async fn write_status_line<T: AsyncWrite + Unpin>(writer: &mut T, code: StatusCode) -> Option<()> {
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

async fn write_headers<T: AsyncWrite + Unpin>(
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

async fn write_body<T: AsyncWrite + Unpin>(writer: &mut T, body: impl AsRef<[u8]>) -> Option<()> {
    writer.write_all(body.as_ref()).await.ok()
}
