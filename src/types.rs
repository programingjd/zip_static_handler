use crate::headers::{default_headers, header_name, header_value};
use http::header::{CACHE_CONTROL, CONTENT_TYPE};
use http::HeaderMap;

const CACHE_CONTROL_NO_CACHE: &str =
    "public,no-cache,max-age=0,must-revalidate;stale-if-error=3600";
const CACHE_CONTROL_REVALIDATE: &str = "public,max-age=3600,must-revalidate,stale-if-error=3600";
const CACHE_CONTROL_IMMUTABLE: &str =
    "public,max-age=86400,immutable,stale-while-revalidate=864000,stale-if-error=3600";

pub(crate) fn headers_for_type(filename: &str, extension: &str) -> Option<(HeaderMap, bool)> {
    match extension {
        "html" | "htm" => Some((headers("text/html", CACHE_CONTROL_REVALIDATE), true)),
        "css" => Some((headers("text/css", CACHE_CONTROL_REVALIDATE), true)),
        "js" | "mjs" | "map" => Some((
            if filename.starts_with("service-worker.") || filename.starts_with("sw.") {
                let mut headers = headers("application/javascript", CACHE_CONTROL_REVALIDATE);
                headers.append(header_name("service-worker-allowed"), header_value("/"));
                headers
            } else {
                headers("application/javascript", CACHE_CONTROL_REVALIDATE)
            },
            true,
        )),
        "wasm" => Some((headers("application/wasm", CACHE_CONTROL_REVALIDATE), true)),
        "woff2" => Some((headers("font/woff2", CACHE_CONTROL_REVALIDATE), false)),
        "ico" => Some((headers("image/x-icon", CACHE_CONTROL_IMMUTABLE), true)),
        "webp" => Some((headers("image/webp", CACHE_CONTROL_IMMUTABLE), false)),
        "avif" => Some((headers("image/avif", CACHE_CONTROL_IMMUTABLE), false)),
        "png" => Some((headers("image/png", CACHE_CONTROL_IMMUTABLE), false)),
        "jpg" => Some((headers("image/jpeg", CACHE_CONTROL_IMMUTABLE), false)),
        "svg" => Some((headers("image/svg+xml", CACHE_CONTROL_IMMUTABLE), true)),
        _ => None,
    }
}

fn headers(content_type: &'static str, cache_control: &'static str) -> HeaderMap {
    let mut headers = default_headers();
    headers.append(CONTENT_TYPE, header_value(content_type));
    headers.append(CACHE_CONTROL, header_value(cache_control));
    headers
}
