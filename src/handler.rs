use crate::compression::{brotli_decompressed_crc32, compress_brotli, decompress_entry};
use crate::errors::Result;
use crate::http::headers::{
    Line, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LENGTH, ETAG, IF_MATCH, IF_NONE_MATCH, LOCATION,
};
use crate::http::method;
use crate::http::request::Request;
use crate::http::response::StatusCode;
use crate::path::{extension, filename, path};
use bytes::Bytes;
use std::collections::HashMap;
use std::io::Cursor;
use tracing::{debug, trace};
use zip_structs::zip_central_directory::ZipCDEntry;
use zip_structs::zip_local_file_header::ZipLocalFileHeader;

pub struct Handler {
    pub(crate) paths: HashMap<String, Entry>,
    pub(crate) error_headers: &'static [Line],
}

impl Handler {
    pub fn entry(&self, path: &str) -> Option<&Entry> {
        self.paths.get(path)
    }
    pub fn handle<Resp, Req: Request<Resp>>(&self, request: Req) -> Resp {
        if let Some(value) = request.first_header_value(CONTENT_LENGTH) {
            if value != b"0" {
                return request.response(StatusCode::BadRequest, self.error_headers.iter(), None);
            }
        }
        let is_get = match request.method() {
            method::GET => true,
            method::HEAD => false,
            _ => {
                return request.response(
                    StatusCode::MethodNotAllowed,
                    self.error_headers.iter(),
                    None,
                )
            }
        };
        let path = String::from_utf8_lossy(request.path());
        if let Some(file) = self.entry(path.as_ref()) {
            let headers = &file.headers;
            if file.etag.is_some() {
                let etag = file.etag.as_ref().map(|it| it.as_bytes());
                let none_match = request.first_header_value(IF_NONE_MATCH);
                let if_match = request.first_header_value(IF_MATCH);
                if none_match.is_some() && none_match == etag {
                    request.response(
                        StatusCode::NotModified,
                        headers
                            .iter()
                            .filter(|&line| !matches!(line.key, CONTENT_LENGTH | CONTENT_ENCODING)),
                        None,
                    )
                } else if if_match.is_some() && if_match != etag {
                    request.response(
                        StatusCode::PreconditionFailed,
                        headers
                            .iter()
                            .filter(|&line| !matches!(line.key, CONTENT_LENGTH | CONTENT_ENCODING)),
                        None,
                    )
                } else if let Some(ref body) = file.content {
                    request.response(
                        StatusCode::OK,
                        headers.iter(),
                        if is_get { Some(body.clone()) } else { None },
                    )
                } else if headers.iter().any(|it| it.key == LOCATION) {
                    request.response(StatusCode::TemporaryRedirect, headers.iter(), None)
                } else {
                    request.response(
                        StatusCode::NoContent,
                        headers
                            .iter()
                            .filter(|&line| !matches!(line.key, CONTENT_LENGTH | CONTENT_ENCODING)),
                        None,
                    )
                }
            } else {
                request.response(StatusCode::PermanentRedirect, headers.iter(), None)
            }
        } else {
            request.response(StatusCode::NotFound, self.error_headers.iter(), None)
        }
    }
}

pub struct Entry {
    pub headers: Vec<Line>,
    pub content: Option<Bytes>,
    pub etag: Option<String>,
}

pub trait HeaderSelector {
    fn headers_for_extension(
        &self,
        filename: &str,
        extension: &str,
    ) -> Option<HeadersAndCompression>;
    fn error_headers(&self) -> &'static [Line];
}

pub struct HeadersAndCompression {
    pub headers: Vec<Line>,
    pub compressible: bool,
    pub redirection: bool,
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
        redirection,
    }) = header_selector.headers_for_extension(filename, extension)
    {
        let path = path(zip_prefix, &name);
        debug!(unprefixed_path = path);
        let zip_file_header = ZipLocalFileHeader::from_central_directory(cursor, entry)?;
        let crc32 = zip_file_header.crc32;
        let etag = if headers.iter().any(|it| it.key == CACHE_CONTROL) {
            let etag = format!("{:x}", crc32);
            trace!(etag = etag.as_str());
            headers.push(Line::with_owned_value(ETAG, etag.as_bytes().to_vec()));
            Some(etag)
        } else {
            None
        };
        let content = Some(if compressible {
            let compressed_name = format!("{name}.br");
            let compressed_name_raw = compressed_name.as_bytes();
            if let Some(entry) = entries.iter().find_map(|entry| {
                if entry.file_name_raw == compressed_name_raw {
                    let zip_file_header =
                        ZipLocalFileHeader::from_central_directory(cursor, entry).ok()?;
                    let decompressed = decompress_entry(zip_file_header).ok()?;
                    if let Some(uncompressed_crc32) =
                        brotli_decompressed_crc32(decompressed.as_ref())
                    {
                        if uncompressed_crc32 == crc32 {
                            Some(decompressed)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }) {
                entry
            } else if let Some(entry) = previous.and_then(|it| {
                it.paths
                    .get(&compressed_name)
                    .filter(|&entry| entry.etag == etag)
            }) {
                entry.content.clone().unwrap()
            } else {
                debug!("brotli {path}", path = path);
                let compressed_size = zip_file_header.compressed_size as usize;
                Bytes::from(compress_brotli(
                    decompress_entry(zip_file_header)?.as_ref(),
                    compressed_size,
                ))
            }
        } else {
            decompress_entry(zip_file_header)?
        });
        if let Some(content) = content {
            if redirection {
                headers.push(Line::with_slice_value(CONTENT_LENGTH, b"0"));
                let end = content
                    .iter()
                    .position(|&b| b.is_ascii_whitespace())
                    .unwrap_or(content.len());
                headers.push(Line::with_owned_value(LOCATION, content[..end].into()));
                Ok(Some((
                    path,
                    Entry {
                        headers,
                        content: None,
                        etag,
                    },
                )))
            } else {
                headers.push(Line::with_owned_value(
                    CONTENT_LENGTH,
                    format!("{}", content.len()).into_bytes(),
                ));
                if compressible {
                    headers.push(Line::with_array_ref_value(CONTENT_ENCODING, b"br"));
                }
                Ok(Some((
                    path,
                    Entry {
                        headers,
                        content: Some(content),
                        etag,
                    },
                )))
            }
        } else {
            Ok(None)
        }
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

    const COMMIT_HASH: &str = "cf874829749d85c92eeeabae44ed8050864f400f";

    fn download(url: &str) -> Vec<u8> {
        debug!(url = url);
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
            COMMIT_HASH,
        ));
        let handler = Handler::builder()
            .with_zip_prefix(&format!("about.programingjd.me-{COMMIT_HASH}/"))
            .with_zip(zip)
            .try_build();
        assert!(handler.is_ok());
        let handler = handler.unwrap();
        let favicon = handler.paths.get("/favicon.png");
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
        assert!(handler.paths.get("/.idea/modules.xml").is_none());
        assert!(handler.paths.get("/").is_some());
        assert!(handler.paths.get("/").unwrap().etag.is_some());
        assert!(handler.paths.get("/index.html").is_none());
        assert!(handler.paths.get("/about").is_some());
        assert!(handler.paths.get("/about").unwrap().content.is_none());
        assert!(handler.paths.get("/about").unwrap().etag.is_none());
        assert!(handler
            .paths
            .get("/about")
            .unwrap()
            .headers
            .iter()
            .any(|it| it.key == LOCATION));
        assert!(handler
            .paths
            .get("/about")
            .unwrap()
            .headers
            .iter()
            .any(|it| it.key == LOCATION));
        assert_eq!(
            handler
                .paths
                .get("/about")
                .unwrap()
                .headers
                .iter()
                .find(|it| it.key == LOCATION)
                .unwrap()
                .value
                .as_ref(),
            b"."
        );
        assert!(handler.paths.get("/profile.jpg").is_some());
        assert!(handler.paths.get("/profile.jpg").unwrap().content.is_none());
        assert!(handler.paths.get("/profile.jpg").unwrap().etag.is_some());
        assert!(handler
            .paths
            .get("/profile.jpg")
            .unwrap()
            .headers
            .iter()
            .any(|it| it.key == LOCATION));
        assert_eq!(
            handler
                .paths
                .get("/profile.jpg")
                .unwrap()
                .headers
                .iter()
                .find(|it| it.key == LOCATION)
                .unwrap()
                .value
                .as_ref(),
            b"profile_512.jpg"
        );
        assert!(handler.paths.get("/about/").is_some());
        assert!(handler.paths.get("/about/").unwrap().content.is_none());
        assert!(handler.paths.get("/about/").unwrap().etag.is_none());
        assert!(handler
            .paths
            .get("/about/")
            .unwrap()
            .headers
            .iter()
            .any(|it| it.key == LOCATION));
        assert_eq!(
            handler
                .paths
                .get("/about/")
                .unwrap()
                .headers
                .iter()
                .find(|it| it.key == LOCATION)
                .unwrap()
                .value
                .as_ref(),
            b"/about"
        );
    }

    #[test]
    fn from_github_with_prefix() {
        let zip = download(&zip_download_commit_url(
            "programingjd",
            "about.programingjd.me",
            COMMIT_HASH,
        ));
        let handler = Handler::builder()
            .with_zip_prefix(&format!("about.programingjd.me-{COMMIT_HASH}/"))
            .with_zip(zip)
            .with_root_prefix("test/")
            .try_build();
        assert!(handler.is_ok());
        let handler = handler.unwrap();
        let favicon = handler.paths.get("/test/favicon.png");
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
        assert!(handler.paths.get("/.idea/modules.xml").is_none());
        assert!(handler.paths.get("/").is_none());
        assert!(handler.paths.get("/index.html").is_none());
        assert!(handler.paths.get("/test/.idea/modules.xml").is_none());
        assert!(handler.paths.get("/test/index.html").is_none());
        assert!(handler.paths.get("/test/").is_some());
        assert!(handler.paths.get("/test/").unwrap().content.is_none());
        assert!(handler.paths.get("/test/").unwrap().etag.is_none());
        assert!(handler
            .paths
            .get("/test/")
            .unwrap()
            .headers
            .iter()
            .any(|it| it.key == LOCATION));
        assert_eq!(
            handler
                .paths
                .get("/test/")
                .unwrap()
                .headers
                .iter()
                .find(|it| it.key == LOCATION)
                .unwrap()
                .value
                .as_ref(),
            b"/test"
        );
        assert!(handler.paths.get("/test").is_some());
        assert!(handler.paths.get("/test").unwrap().etag.is_some());
        assert!(handler.paths.get("/test/about").is_some());
        assert!(handler.paths.get("/test/about").unwrap().content.is_none());
        assert!(handler.paths.get("/test/about").unwrap().etag.is_none());
        assert!(handler
            .paths
            .get("/test/about")
            .unwrap()
            .headers
            .iter()
            .any(|it| it.key == LOCATION));
        assert_eq!(
            handler
                .paths
                .get("/test/about")
                .unwrap()
                .headers
                .iter()
                .find(|it| it.key == LOCATION)
                .unwrap()
                .value
                .as_ref(),
            b"."
        );
        assert!(handler.paths.get("/test/profile.jpg").is_some());
        assert!(handler
            .paths
            .get("/test/profile.jpg")
            .unwrap()
            .content
            .is_none());
        assert!(handler
            .paths
            .get("/test/profile.jpg")
            .unwrap()
            .etag
            .is_some());
        assert!(handler
            .paths
            .get("/test/profile.jpg")
            .unwrap()
            .headers
            .iter()
            .any(|it| it.key == LOCATION));
        assert_eq!(
            handler
                .paths
                .get("/test/profile.jpg")
                .unwrap()
                .headers
                .iter()
                .find(|it| it.key == LOCATION)
                .unwrap()
                .value
                .as_ref(),
            b"profile_512.jpg"
        );
        assert!(handler.paths.get("/test/about/").is_some());
        assert!(handler.paths.get("/test/about/").unwrap().content.is_none());
        assert!(handler.paths.get("/test/about/").unwrap().etag.is_none());
        assert!(handler
            .paths
            .get("/test/about/")
            .unwrap()
            .headers
            .iter()
            .any(|it| it.key == LOCATION));
        assert_eq!(
            handler
                .paths
                .get("/test/about/")
                .unwrap()
                .headers
                .iter()
                .find(|it| it.key == LOCATION)
                .unwrap()
                .value
                .as_ref(),
            b"/test/about"
        );
    }
}
