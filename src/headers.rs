use crate::http::headers::{
    Line, ALLOW, COEP, COOP, CORP, CSP, HSTS, X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
    X_XSS_PROTECTION,
};
use lazy_static::lazy_static;

lazy_static! {
    static ref DEFAULT_HEADERS: Vec<(&'static [u8], &'static [u8])> = {
        let headers/*: Vec<(&'static [u8], &'static [u8])>*/ = vec![
            (ALLOW, b"GET, HEAD".as_slice()),
            (X_CONTENT_TYPE_OPTIONS, b"nosniff".as_slice()),
            (X_FRAME_OPTIONS, b"DENY".as_slice()),
            (X_XSS_PROTECTION, b"1; mode=block".as_slice()),
            (CORP, b"same-site".as_slice()),
            (COEP, b"crendentialless".as_slice()),
            (COOP, b"same-origin".as_slice()),
            (CSP, b"default-src 'self';script-src 'wasm-unsafe-eval';script-src-elem 'self' 'unsafe-inline';script-src-attr 'none';worker-src 'self' blob:;style-src 'self' 'unsafe-inline';img-src 'self' data: blob:;font-src 'self' data:;frame-src 'none';object-src 'none';base-uri 'none';frame-ancestors 'none';form-action 'none'".as_slice()),
            (HSTS, b"max-age=63072000; includeSubDomains; preload".as_slice()),
        ];
        headers
    };
    static ref ERROR_HEADERS: Vec<(&'static [u8], &'static [u8])> = {
        let headers/*: Vec<(&'static [u8], &'static [u8])>*/ = vec![
            (ALLOW, b"GET, HEAD".as_slice()),
            (HSTS, b"max-age=63072000; includeSubDomains; preload".as_slice()),
        ];
        headers
    };
}

pub(crate) fn default_headers() -> impl Iterator<Item = Line> {
    crate::headers::DEFAULT_HEADERS.iter().map(|&it| it.into())
}

pub(crate) fn error_headers() -> impl Iterator<Item = Line> {
    crate::headers::ERROR_HEADERS.iter().map(|&it| it.into())
}

// struct DefaultDefaultHeaders;
// impl DefaultHeaders for DefaultDefaultHeaders {
//     fn default_headers() -> impl Iterator<Item=Line> {
//         DEFAULT_HEADERS.iter().map(|&it| it.into())
//     }
//
//     fn error_headers() -> impl Iterator<Item=Line> {
//         ERROR_HEADERS.iter().map(|&it| it.into())
//     }
// }
//
// pub trait DefaultHeaders {
//     fn default_headers() -> impl Iterator<Item = Line>;
//     fn error_headers() -> impl Iterator<Item = Line>;
// }
