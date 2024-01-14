mod compression;
mod errors;
mod github;
mod headers;
mod types;

use crate::compression::{brotli, inflate};
use crate::errors::Error;
use crate::headers::{default_headers, error_headers};
use crate::types::headers_for_type;
use errors::Result;
use http::header::LOCATION;
use http::{HeaderMap, HeaderValue, Request, StatusCode};
use std::collections::HashMap;
use std::io::Cursor;
use zip_structs::zip_central_directory::ZipCDEntry;
use zip_structs::zip_eocd::ZipEOCD;
use zip_structs::zip_local_file_header::ZipLocalFileHeader;

pub struct Handler {
    files: HashMap<String, Entry>,
}

pub struct Response {
    status: StatusCode,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
}

impl Handler {
    fn handle<T>(&self, request: Request<T>) -> Response {
        let path = request.uri().path();
        if let Some(file) = self.files.get(path) {
            todo!()
        } else {
            Response {
                status: StatusCode::NOT_FOUND,
                headers: error_headers(),
                body: None,
            }
        }
    }
}

pub fn handler(
    route_prefix: impl AsRef<str>,
    zip_prefix: impl AsRef<str>,
    zip: &[u8],
) -> Result<Handler> {
    let route_prefix = route_prefix.as_ref();
    let zip_prefix = zip_prefix.as_ref();
    let mut cursor = Cursor::new(zip);
    let directory = ZipEOCD::from_reader(&mut cursor)?;
    let mut routes = HashMap::new();
    for entry in ZipCDEntry::all_from_eocd(&mut cursor, &directory)? {
        if let Some((path, value)) = build_entry(&mut cursor, entry, zip_prefix)? {
            if path.ends_with('/') {
                let no_trailing_slash = &path[..path.len() - 1];
                routes.insert(
                    format!("{route_prefix}{path}"),
                    redirect_entry(&no_trailing_slash),
                );
                routes.insert(format!("{route_prefix}{no_trailing_slash}"), value);
            } else {
                routes.insert(path, value);
            }
        }
    }
    Ok(Handler { files: routes })
}

struct Entry {
    headers: HeaderMap,
    content: Option<Vec<u8>>,
    etag: Option<String>,
}

fn redirect_entry(path: &str) -> Entry {
    let mut headers = default_headers();
    headers.append(
        LOCATION,
        HeaderValue::from_str(path).expect("invalid header value"),
    );
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
) -> Result<Option<(String, Entry)>> {
    let name = String::from_utf8(entry.file_name_raw.clone())?;
    if !name.starts_with(zip_prefix) {
        return Ok(None);
    }
    let filename = filename(&name);
    if filename.starts_with('.') {
        return Ok(None);
    }
    let (extension, precompressed) = match extension(filename) {
        "br" => (extension(&filename[..filename.len() - 3]), true),
        ext => (ext, false),
    };
    if let Some((headers, compressed)) = headers_for_type(filename, extension) {
        let path = path(zip_prefix, &name);
        let zip_file_header = ZipLocalFileHeader::from_central_directory(cursor, &entry)?;
        let etag = Some(format!("{:x}", zip_file_header.crc32));
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

fn path(zip_prefix: &str, name: &str) -> String {
    let name = &name[zip_prefix.len()..];
    let start = name.find(|c| c != '.' && c != '/').unwrap_or(0);
    let end = if name.ends_with("index.html") {
        name.len() - 10
    } else {
        name.len()
    };
    format!("/{}", &name[start..end])
}

fn filename(name: &str) -> &str {
    let byte_position = name.rfind(|c| c == '/').map(|it| it + 1).unwrap_or(0);
    &name[byte_position..]
}

fn extension(filename: &str) -> &str {
    let byte_position = filename.rfind(|c| c == '.').map(|it| it + 1).unwrap_or(0);
    &filename[byte_position..]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::zip_download_branch_url;
    use reqwest::blocking::Client;

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
    fn repo() {
        let zip = download(&zip_download_branch_url("packurl", "wasm_br", "main"));
        assert!(handler("", "", &zip).is_ok());
    }
}
