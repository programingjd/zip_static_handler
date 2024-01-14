use http::header::{
    ALLOW, CONTENT_SECURITY_POLICY, STRICT_TRANSPORT_SECURITY, X_CONTENT_TYPE_OPTIONS,
    X_FRAME_OPTIONS, X_XSS_PROTECTION,
};
use http::{HeaderMap, HeaderName, HeaderValue};
use std::cell::OnceCell;

const DEFAULT_HEADERS: OnceCell<HeaderMap> = OnceCell::new();
const ERROR_HEADERS: OnceCell<HeaderMap> = OnceCell::new();

pub(crate) fn default_headers() -> HeaderMap {
    DEFAULT_HEADERS
        .get_or_init(|| {
            let mut headers = HeaderMap::new();
            headers.append(ALLOW, header_value("GET, HEAD"));
            headers.append(X_CONTENT_TYPE_OPTIONS, header_value("nosniff"));
            headers.append(X_FRAME_OPTIONS, header_value("DENY"));
            headers.append(X_XSS_PROTECTION, header_value("1; mode=block"));
            headers.append(
                header_name("cross-origin-resource-policy"),
                header_value("same-site"),
            );
            headers.append(
                header_name("cross-origin-embedder-policy"),
                header_value("crendentialless"),
            );
            headers.append(
                header_name("cross-origin-opener-policy"),
                header_value("same-origin"),
            );
            headers.append(
                CONTENT_SECURITY_POLICY, header_value(
                    "default-src 'self';script-src 'self' 'unsafe-inline';script-src-attr 'none' 'wasm-unsafe-eval';worker-src 'self' blob:;style-src 'self' 'unsafe-inline';img-src 'self' data: blob:;font-src 'self' data:;frame-src 'none';object-src 'none';base-uri 'none';frame-ancestors 'none';form-action 'none'"
                ));
            headers.append(
                STRICT_TRANSPORT_SECURITY, header_value("max-age=63072000; includeSubDomains; preload")
            );
            headers
        })
        .clone()
}

pub(crate) fn error_headers() -> HeaderMap {
    ERROR_HEADERS
        .get_or_init(|| {
            let mut headers = HeaderMap::new();
            headers.append(ALLOW, header_value("GET, HEAD"));
            headers.append(
                STRICT_TRANSPORT_SECURITY,
                header_value("max-age=63072000; includeSubDomains; preload"),
            );
            headers
        })
        .clone()
}

pub(crate) fn header_name(bytes: &'static str) -> HeaderName {
    HeaderName::from_static(bytes)
}

pub(crate) fn header_value(bytes: &'static str) -> HeaderValue {
    HeaderValue::from_static(bytes)
}
