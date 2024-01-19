use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::http;
use hyper::http::{HeaderName, HeaderValue};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use reqwest::Client;
use std::fmt::{Display, Formatter};
use std::str::from_utf8;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::spawn;
use zip_static_handler::errors::{Error, Result};
use zip_static_handler::github::zip_download_branch_url;
use zip_static_handler::handler::Handler;
use zip_static_handler::http::headers::Line;
use zip_static_handler::http::request::Request;
use zip_static_handler::http::response::{Builder, StatusCode};

type HyperResponse = hyper::Response<BoxBody<Bytes, hyper::Error>>;
type HyperRequest = hyper::Request<hyper::body::Incoming>;

async fn download(
    url: &str,
) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let response = Client::default().get(url).send().await?;
    if !response.status().is_success() {
        panic!("failed to download {url} ({})", response.status().as_str());
    }
    Ok(response.bytes().await?.to_vec())
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(("0.0.0.0", 8080u16)).await?;
    let zip = download(&zip_download_branch_url(
        "programingjd",
        "about.programingjd.me",
        "main",
    ))
    .await?;
    let handler = Arc::new(
        Handler::builder()
            .with_zip_prefix("about.programingjd.me-main/")
            .with_zip(zip)
            .try_build()
            .map_err(|err| {
                Box::new(ErrorAdapter {
                    message: err.to_string(),
                })
            })?,
    );
    loop {
        let (stream, _remote_address) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let handler = handler.clone();
        spawn(async move {
            let _ = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(move |request| {
                        let handler = handler.clone();
                        async move {
                            match handler.handle(RequestAdapter { inner: request }) {
                                Ok(response) => Ok(response),
                                Err(err) => Err(Box::new(ErrorAdapter {
                                    message: err.to_string(),
                                })),
                            }
                        }
                    }),
                )
                .await;
        });
    }
}

#[derive(Debug)]
struct ErrorAdapter {
    message: String,
}

impl Display for ErrorAdapter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ErrorAdapter {}

struct RequestAdapter {
    inner: HyperRequest,
}

struct ResponseBuilderAdapter {
    inner: http::response::Builder,
}

impl Request<HyperResponse, ResponseBuilderAdapter> for RequestAdapter {
    fn method(&self) -> &[u8] {
        self.inner.method().as_str().as_bytes()
    }

    fn path(&self) -> &[u8] {
        self.inner.uri().path().as_bytes()
    }

    fn first_header_value(&self, key: &'static [u8]) -> Option<&[u8]> {
        from_utf8(key)
            .ok()
            .and_then(|key| self.inner.headers().get(key).map(|it| it.as_bytes()))
    }

    fn response_builder_with_status(code: StatusCode) -> ResponseBuilderAdapter {
        let code: u16 = code.into();
        ResponseBuilderAdapter {
            inner: hyper::Response::builder().status(code),
        }
    }
}

impl ResponseBuilderAdapter {
    fn full(slice: &[u8]) -> BoxBody<Bytes, hyper::Error> {
        Full::new(Bytes::copy_from_slice(slice))
            .map_err(|never| match never {})
            .boxed()
    }
    fn empty() -> BoxBody<Bytes, hyper::Error> {
        Empty::<Bytes>::new()
            .map_err(|never| match never {})
            .boxed()
    }
}

impl Builder<HyperResponse> for ResponseBuilderAdapter {
    fn append_headers(self, headers: impl Iterator<Item = impl AsRef<Line>>) -> Self {
        let mut inner = self.inner;
        let map = inner.headers_mut().unwrap();
        headers.for_each(|ref line| {
            let line = line.as_ref();
            if let Ok(name) = HeaderName::from_bytes(line.key) {
                if let Ok(value) = HeaderValue::from_bytes(line.value.as_ref()) {
                    map.append(name, value);
                }
            }
        });
        Self { inner }
    }

    fn with_body(self, body: Option<&[u8]>) -> Result<HyperResponse> {
        let body = body.map(Self::full).unwrap_or_else(Self::empty);
        self.inner
            .body(body)
            .map_err(|it| Error::Wrapped(Box::new(it)))
    }
}
