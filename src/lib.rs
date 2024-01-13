mod errors;
mod github;
mod types;

use crate::types::headers_for_type;
use errors::Result;
use http::HeaderMap;
use std::collections::HashMap;
use std::io::Cursor;
use zip_structs::zip_central_directory::ZipCDEntry;
use zip_structs::zip_eocd::ZipEOCD;

pub struct Handler {
    files: HashMap<String, Entry>,
}

pub fn handler(
    route_prefix: impl AsRef<str>,
    zip_prefix: impl AsRef<str>,
    zip: &[u8],
) -> Result<Handler> {
    let route_prefix = route_prefix.as_ref();
    let zip_prefix = route_prefix.as_ref();
    let mut cursor = Cursor::new(zip);
    let directory = ZipEOCD::from_reader(&mut cursor)?;
    let mut routes = HashMap::new();
    for entry in ZipCDEntry::all_from_eocd(&mut cursor, &directory)? {
        if let Some((path, value)) = build_entry(entry, zip_prefix)? {
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
    redirect: bool,
    compressed: bool,
    headers: HeaderMap,
    content: Vec<u8>,
}

fn redirect_entry(path: &str) -> Entry {
    todo!()
}

fn build_entry(entry: ZipCDEntry, zip_prefix: &str) -> Result<Option<(String, Entry)>> {
    let name = String::from_utf8(entry.file_name_raw)?;
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
        Ok(Some((
            path,
            Entry {
                redirect: false,
                compressed,
                headers,
                content: todo!(),
            },
        )))
    } else {
        Ok(None)
    }
}

fn path(zip_prefix: &str, name: &str) -> String {
    // let name2 = if let Some(prefix) = zip_prefix {
    //     &name[prefix.as_ref().len(),..]
    // } else {
    //     name
    // }
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
