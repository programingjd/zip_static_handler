use crate::headers::default_headers;
use crate::http::headers::{Line, CACHE_CONTROL, CONTENT_TYPE, SERVICE_WORKER_ALLOWED};

const CACHE_CONTROL_NO_CACHE: &[u8] =
    b"public,no-cache,max-age=0,must-revalidate;stale-if-error=3600";
const CACHE_CONTROL_REVALIDATE: &[u8] = b"public,max-age=3600,must-revalidate,stale-if-error=3600";
const CACHE_CONTROL_IMMUTABLE: &[u8] =
    b"public,max-age=86400,immutable,stale-while-revalidate=864000,stale-if-error=3600";

pub(crate) fn headers_for_type(
    filename: &str,
    extension: &str,
) -> Option<(Box<dyn Iterator<Item = Line>>, bool)> {
    match extension {
        "html" | "htm" => Some((headers(b"text/html", CACHE_CONTROL_REVALIDATE), true)),
        "css" => Some((headers(b"text/css", CACHE_CONTROL_REVALIDATE), true)),
        "js" | "mjs" | "map" => Some((
            if filename.starts_with("service-worker.") || filename.starts_with("sw.") {
                let headers = headers(b"application/javascript", CACHE_CONTROL_NO_CACHE);
                Box::new(headers.chain(vec![Line::with_array_ref_value(
                    SERVICE_WORKER_ALLOWED,
                    b"/",
                )]))
            } else {
                headers(b"application/javascript", CACHE_CONTROL_REVALIDATE)
            },
            true,
        )),
        "wasm" => Some((headers(b"application/wasm", CACHE_CONTROL_REVALIDATE), true)),
        "woff2" => Some((headers(b"font/woff2", CACHE_CONTROL_REVALIDATE), false)),
        "ico" => Some((headers(b"image/x-icon", CACHE_CONTROL_IMMUTABLE), true)),
        "webp" => Some((headers(b"image/webp", CACHE_CONTROL_IMMUTABLE), false)),
        "avif" => Some((headers(b"image/avif", CACHE_CONTROL_IMMUTABLE), false)),
        "png" => Some((headers(b"image/png", CACHE_CONTROL_IMMUTABLE), false)),
        "jpg" => Some((headers(b"image/jpeg", CACHE_CONTROL_IMMUTABLE), false)),
        "svg" => Some((headers(b"image/svg+xml", CACHE_CONTROL_IMMUTABLE), true)),
        _ => None,
    }
}

fn headers(
    content_type: &'static [u8],
    cache_control: &'static [u8],
) -> Box<dyn Iterator<Item = Line>> {
    let default_headers = default_headers();
    let new_headers: Vec<Line> = vec![
        Line::with_slice_value(CONTENT_TYPE, content_type),
        Line::with_slice_value(CACHE_CONTROL, cache_control),
    ];
    Box::new(default_headers.chain(new_headers))
}
