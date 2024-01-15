use crate::compression::{brotli, inflate};
use crate::errors::{Error, Result};
use crate::headers::{default_headers, error_headers};
use crate::http::headers::{
    Line, CONTENT_ENCODING, CONTENT_LENGTH, ETAG, IF_MATCH, IF_NONE_MATCH, LOCATION,
};
use crate::http::method;
use crate::http::request::Request;
use crate::http::response::{Builder, StatusCode};
use crate::path::{extension, filename, path};
use crate::types::headers_for_type;
use std::collections::HashMap;
use std::io::Cursor;
use std::iter::once;
use std::ops::DerefMut;
use tracing::{debug, trace};
use zip_structs::zip_central_directory::ZipCDEntry;
use zip_structs::zip_eocd::ZipEOCD;
use zip_structs::zip_local_file_header::ZipLocalFileHeader;

pub struct Handler {
    files: HashMap<String, Entry>,
}

impl Handler {
    pub fn handle<R, B: Builder<R>, T: Request<R, B>>(&self, request: T) -> Result<R> {
        if let Some(value) = request.first_header_value(CONTENT_LENGTH) {
            if value != b"0" {
                return T::response_builder_with_status(StatusCode::BadRequest)
                    .append_headers(error_headers())
                    .with_body(None);
            }
        }
        let method = request.method();
        match method {
            method::GET | method::HEAD => {}
            _ => {
                return T::response_builder_with_status(StatusCode::MethodNotAllowed)
                    .append_headers(error_headers())
                    .with_body(None)
            }
        };
        let path = String::from_utf8_lossy(request.path());
        if let Some(file) = self.files.get(path.as_ref()) {
            let headers = &file.headers;
            if file.etag.is_some() {
                let etag = file.etag.as_ref().map(|it| it.as_bytes());
                if request.first_header_value(IF_NONE_MATCH) == etag {
                    T::response_builder_with_status(StatusCode::NotModified)
                        .append_headers(headers.iter())
                        .with_body(None)
                } else if request.first_header_value(IF_MATCH) != etag {
                    T::response_builder_with_status(StatusCode::PreconditionFailed)
                        .append_headers(headers.iter())
                        .with_body(None)
                } else {
                    T::response_builder_with_status(StatusCode::OK)
                        .append_headers(headers.iter())
                        .with_body(if method == method::GET {
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
                T::response_builder_with_status(StatusCode::PermanentRedirect)
                    .append_headers(headers.iter())
                    .with_body(None)
            }
        } else {
            T::response_builder_with_status(StatusCode::NotFound)
                .append_headers(error_headers())
                .with_body(None)
        }
    }
}

impl Handler {
    pub fn try_new(
        route_prefix: impl AsRef<str>,
        zip_prefix: impl AsRef<str>,
        zip: &[u8],
    ) -> crate::errors::Result<Handler> {
        let route_prefix = route_prefix.as_ref();
        let zip_prefix = zip_prefix.as_ref();
        trace!(route_prefix = route_prefix, zip_prefix = zip_prefix);
        let mut cursor = Cursor::new(zip);
        let directory = ZipEOCD::from_reader(&mut cursor)?;
        let mut routes = HashMap::new();
        for entry in ZipCDEntry::all_from_eocd(&mut cursor, &directory)? {
            if let Some((path, value)) = build_entry(&mut cursor, entry, zip_prefix)? {
                if path.ends_with('/') {
                    let no_trailing_slash = &path[..path.len() - 1];
                    routes.insert(
                        format!("{route_prefix}{path}"),
                        redirect_entry(no_trailing_slash),
                    );
                    routes.insert(format!("{route_prefix}{no_trailing_slash}"), value);
                } else {
                    routes.insert(path, value);
                }
            }
        }
        Ok(Handler { files: routes })
    }
}

struct Entry {
    headers: Vec<Line>,
    content: Option<Vec<u8>>,
    etag: Option<String>,
}

fn redirect_entry(path: &str) -> Entry {
    let headers = default_headers()
        .chain(once(Line::with_owned_value(
            LOCATION,
            path.as_bytes().to_vec(),
        )))
        .collect();
    Entry {
        headers,
        content: None,
        etag: None,
    }
}

fn build_entry(
    cursor: &mut Cursor<&[u8]>,
    entry: ZipCDEntry,
    zip_prefix: &str,
) -> crate::errors::Result<Option<(String, Entry)>> {
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
    let (extension, precompressed) = match extension(filename) {
        "br" => (extension(&filename[..filename.len() - 3]), true),
        ext => (ext, false),
    };
    trace!(extension = extension);
    if let Some((mut headers, compressed)) = headers_for_type(filename, extension) {
        let headers = headers.deref_mut();
        let path = path(zip_prefix, &name);
        debug!(path = path);
        let zip_file_header = ZipLocalFileHeader::from_central_directory(cursor, &entry)?;
        let etag = format!("{:x}", zip_file_header.crc32);
        trace!(etag = etag.as_str());
        let mut headers: Vec<Line> = headers
            .chain(once(Line::with_owned_value(ETAG, etag.as_bytes().to_vec())))
            .collect();
        let etag = Some(etag);
        let content = Some(if compressed {
            match zip_file_header.compression_method {
                0u16 => {
                    if precompressed {
                        zip_file_header.compressed_data.to_vec()
                    } else {
                        brotli(
                            zip_file_header.compressed_data.as_ref(),
                            zip_file_header.compressed_size as usize,
                        )
                    }
                }
                8u16 => {
                    if precompressed {
                        inflate(
                            zip_file_header.compressed_data.as_ref(),
                            zip_file_header.uncompressed_size as usize,
                        )
                    } else {
                        brotli(
                            &inflate(
                                zip_file_header.compressed_data.as_ref(),
                                zip_file_header.uncompressed_size as usize,
                            ),
                            zip_file_header.compressed_size as usize,
                        )
                    }
                }
                _ => return Err(Error::Message("unsupported compression")),
            }
        } else {
            match zip_file_header.compression_method {
                0u16 => zip_file_header.compressed_data.to_vec(),
                8u16 => inflate(
                    zip_file_header.compressed_data.as_ref(),
                    zip_file_header.uncompressed_size as usize,
                ),
                _ => return Err(Error::Message("unsupported compression")),
            }
        });
        if let Some(ref content) = content {
            headers.push(Line::with_owned_value(
                CONTENT_LENGTH,
                format!("{}", content.len()).into_bytes(),
            ));
            if compressed {
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
            "43fc826fd10790699f882a8d37d2c3da6192a499",
        ));
        let handler = Handler::try_new(
            "",
            "about.programingjd.me-43fc826fd10790699f882a8d37d2c3da6192a499/",
            &zip,
        );
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
