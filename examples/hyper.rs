use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use reqwest::Client;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::spawn;
use zip_static_handler::github::zip_download_branch_url;
use zip_static_handler::handler::{Handler, HeaderSelector, HeadersAndCompression};
use zip_static_handler::http::headers::{Line, ALLOW, CACHE_CONTROL, CONTENT_TYPE};

async fn download(url: &str) -> Result<Vec<u8>, reqwest::Error> {
    let response = Client::default().get(url).send().await?;
    if !response.status().is_success() {
        panic!("failed to download {url} ({})", response.status().as_str());
    }
    Ok(response.bytes().await?.to_vec())
}

struct HSelector;

impl HeaderSelector for HSelector {
    fn headers_for_extension(
        &self,
        _filename: &str,
        extension: &str,
    ) -> Option<HeadersAndCompression> {
        match extension {
            "html" => Some(headers_and_compression(b"text/html", b"no-cache", true)),
            "css" => Some(headers_and_compression(b"text/css", b"no-cache", true)),
            "json" => Some(headers_and_compression(
                b"application/json",
                b"no-cache",
                true,
            )),
            "ico" => Some(headers_and_compression(
                b"image/x-icon",
                b"max-age=604800,immutable",
                true,
            )),
            "jpg" => Some(headers_and_compression(
                b"image/jpg",
                b"max-age=604800,immutable",
                true,
            )),
            "webp" => Some(headers_and_compression(
                b"image/webp",
                b"max-age=604800,immutable",
                true,
            )),
            _ => None,
        }
    }
}

fn default_headers() -> Vec<Line> {
    vec![Line::with_array_ref_value(ALLOW, b"GET, HEAD")]
}

fn headers_and_compression(
    content_type: &'static [u8],
    cache_control: &'static [u8],
    compressible: bool,
) -> HeadersAndCompression {
    let mut headers = default_headers();
    headers.push(Line::with_slice_value(CONTENT_TYPE, content_type));
    headers.push(Line::with_slice_value(CACHE_CONTROL, cache_control));
    HeadersAndCompression {
        headers,
        compressible,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
            .with_custom_header_selector(&HSelector)
            .with_zip(zip)
            .try_build()?,
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
                            match handler.handle_hyper_request(request) {
                                Ok(response) => Ok(response),
                                Err(_) => hyper::Response::builder().status(500).body(empty()),
                            }
                        }
                    }),
                )
                .await;
        });
    }
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}
