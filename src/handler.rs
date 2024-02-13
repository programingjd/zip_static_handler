use crate::compression::{brotli, decompress};
use crate::errors::Result;
use crate::http::headers::{
    Line, CONTENT_ENCODING, CONTENT_LENGTH, ETAG, IF_MATCH, IF_NONE_MATCH, LOCATION,
};
use crate::http::method;
use crate::http::request::Request;
use crate::http::response::{Builder, StatusCode};
use crate::path::{extension, filename, path};
use crate::types::error_headers;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Cursor;
use tracing::{debug, info, trace};
use zip_structs::zip_central_directory::ZipCDEntry;
use zip_structs::zip_local_file_header::ZipLocalFileHeader;

pub struct Handler {
    pub(crate) files: HashMap<String, Entry>,
}

impl Handler {
    pub fn handle<R, B: Builder<R>, T: Request<R, B>>(&self, mut request: T) -> Result<R> {
        if let Some(value) = request.first_header_value(CONTENT_LENGTH) {
            if value != b"0" {
                return request
                    .response_builder_with_status(StatusCode::BadRequest)
                    .append_headers(error_headers())
                    .with_body(None);
            }
        }
        let is_get = match request.method() {
            method::GET => true,
            method::HEAD => false,
            _ => {
                return request
                    .response_builder_with_status(StatusCode::MethodNotAllowed)
                    .append_headers(error_headers())
                    .with_body(None)
            }
        };
        let path = String::from_utf8_lossy(request.path());
        if let Some(file) = self.files.get(path.as_ref()) {
            let headers = &file.headers;
            if file.etag.is_some() {
                let etag = file.etag.as_ref().map(|it| it.as_bytes());
                let none_match = request.first_header_value(IF_NONE_MATCH);
                let if_match = request.first_header_value(IF_MATCH);
                if none_match.is_some() && none_match == etag {
                    request
                        .response_builder_with_status(StatusCode::NotModified)
                        .append_headers(headers.iter())
                        .with_body(None)
                } else if if_match.is_some() && if_match != etag {
                    request
                        .response_builder_with_status(StatusCode::PreconditionFailed)
                        .append_headers(headers.iter())
                        .with_body(None)
                } else {
                    request
                        .response_builder_with_status(StatusCode::OK)
                        .append_headers(headers.iter())
                        .with_body(if is_get {
                            if let Some(ref body) = file.content {
                                Some(body.as_slice())
                            } else {
                                None
                            }
                        } else {
                            None
                        })
                }
            } else {
                request
                    .response_builder_with_status(StatusCode::PermanentRedirect)
                    .append_headers(headers.iter())
                    .with_body(None)
            }
        } else {
            request
                .response_builder_with_status(StatusCode::NotFound)
                .append_headers(error_headers())
                .with_body(None)
        }
    }
}

pub(crate) struct Entry {
    headers: Vec<Line>,
    content: Option<Vec<u8>>,
    etag: Option<String>,
}

pub trait HeaderSelector {
    fn headers_for_extension(
        &self,
        filename: &str,
        extension: &str,
    ) -> Option<HeadersAndCompression>;
}

pub struct HeadersAndCompression {
    pub headers: Vec<Line>,
    pub compressible: bool,
}

pub(crate) fn redirect_entry(path: &str) -> Entry {
    let headers = vec![Line::with_owned_value(LOCATION, path.as_bytes().to_vec())];
    Entry {
        headers,
        content: None,
        etag: None,
    }
}

pub(crate) fn build_entry(
    cursor: &mut Cursor<&[u8]>,
    zip_prefix: &str,
    entry: &ZipCDEntry,
    entries: &[ZipCDEntry],
    header_selector: &dyn HeaderSelector,
    previous: Option<&Handler>,
) -> Result<Option<(String, Entry)>> {
    let name = String::from_utf8(entry.file_name_raw.clone())?;
    trace!(entry_name = name);
    if !name.starts_with(zip_prefix) {
        trace!("entry skipped (doesn't start with zip prefix)");
        return Ok(None);
    }
    let filename = filename(&name);
    if filename.starts_with('.') || name.starts_with('.') || name.contains("/.") {
        trace!("entry skipped");
        return Ok(None);
    }
    let extension = extension(filename);
    if extension == "br" {
        return Ok(None);
    };
    trace!(extension = extension);
    if let Some(HeadersAndCompression {
        mut headers,
        compressible,
    }) = header_selector.headers_for_extension(filename, extension)
    {
        let path = path(zip_prefix, &name);
        info!(path = path);
        let zip_file_header = ZipLocalFileHeader::from_central_directory(cursor, entry)?;
        let crc32 = zip_file_header.crc32;
        let etag = format!("{:x}", crc32);
        trace!(etag = etag.as_str());
        headers.push(Line::with_owned_value(ETAG, etag.as_bytes().to_vec()));
        let etag = Some(etag);
        let content = Some(if compressible {
            let compressed_name = format!("{name}.br");
            let compressed_name_raw = compressed_name.as_bytes();
            if let Some(entry) = entries
                .iter()
                .find(|it| it.file_name_raw == compressed_name_raw && it.crc32 == crc32)
            {
                let zip_file_header = ZipLocalFileHeader::from_central_directory(cursor, entry)?;
                match decompress(zip_file_header)? {
                    Cow::Owned(it) => it,
                    Cow::Borrowed(it) => it.to_vec(),
                }
            } else if let Some(entry) = previous.and_then(|it| {
                it.files
                    .get(&compressed_name)
                    .filter(|&entry| entry.etag == etag)
            }) {
                entry.content.clone().unwrap()
            } else {
                debug!("brotli {path}", path = path);
                let compressed_size = zip_file_header.compressed_size as usize;
                brotli(decompress(zip_file_header)?.as_ref(), compressed_size)
            }
        } else {
            match decompress(zip_file_header)? {
                Cow::Owned(it) => it,
                Cow::Borrowed(it) => it.to_vec(),
            }
        });
        if let Some(ref content) = content {
            headers.push(Line::with_owned_value(
                CONTENT_LENGTH,
                format!("{}", content.len()).into_bytes(),
            ));
            if compressible {
                headers.push(Line::with_array_ref_value(CONTENT_ENCODING, b"br"));
            }
        }
        Ok(Some((
            path,
            Entry {
                headers,
                content,
                etag,
            },
        )))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::zip_download_commit_url;
    use crate::http::headers::CONTENT_TYPE;
    use reqwest::blocking::Client;
    use test_tracing::test;

    fn download(url: &str) -> Vec<u8> {
        let response = Client::default()
            .get(url)
            .send()
            .expect(&format!("failed to download {url}"));
        if !response.status().is_success() {
            panic!("failed to download {url} ({})", response.status().as_str());
        }
        response.bytes().unwrap().to_vec()
    }

    #[test]
    fn from_github() {
        let zip = download(&zip_download_commit_url(
            "programingjd",
            "about.programingjd.me",
            "b9ea1260114c63a9d5761fe214b85299cc617c5c",
        ));
        let handler = Handler::builder()
            .with_zip_prefix("about.programingjd.me-b9ea1260114c63a9d5761fe214b85299cc617c5c/")
            .with_zip(zip)
            .try_build();
        assert!(handler.is_ok());
        let handler = handler.unwrap();
        let favicon = handler.files.get("/favicon.png");
        assert!(favicon.is_some());
        let favicon = favicon.unwrap();
        assert_eq!(
            favicon
                .headers
                .iter()
                .find_map(|ref line| if line.key == CONTENT_TYPE {
                    Some(line.value.as_ref())
                } else {
                    None
                }),
            Some(b"image/png".as_slice())
        );
        assert!(handler.files.get("/.idea/modules.xml").is_none());
        assert!(handler.files.get("/").is_some());
        assert!(handler.files.get("/index.html").is_none());
    }
}
