use http_body_util::{Either, Empty, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use reqwest::Client;
use std::convert::Infallible;
use std::sync::{Arc, LazyLock};
use tokio::net::TcpListener;
use tokio::spawn;
use zip_static_handler::github::zip_download_branch_url;
use zip_static_handler::handler::{Handler, HeaderSelector, HeadersAndCompression};
use zip_static_handler::http::headers::{Line, ALLOW, CACHE_CONTROL, CONTENT_TYPE};

static DEFAULT_HEADERS: LazyLock<Vec<Line>> =
    LazyLock::new(|| vec![Line::with_array_ref_value(ALLOW, b"GET, HEAD")]);

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
            "html" => Some(headers_and_compression(
                Some(b"text/html"),
                Some(b"no-cache"),
                true,
            )),
            "css" => Some(headers_and_compression(
                Some(b"text/css"),
                Some(b"no-cache"),
                true,
            )),
            "json" => Some(headers_and_compression(
                Some(b"application/json"),
                Some(b"no-cache"),
                true,
            )),
            "ico" => Some(headers_and_compression(
                Some(b"image/x-icon"),
                Some(b"max-age=604800,immutable"),
                true,
            )),
            "jpg" => Some(headers_and_compression(
                Some(b"image/jpg"),
                Some(b"max-age=604800,immutable"),
                true,
            )),
            "webp" => Some(headers_and_compression(
                Some(b"image/webp"),
                Some(b"max-age=604800,immutable"),
                true,
            )),
            "307" => Some(headers_and_compression(None, Some(b"no-cache"), false)),
            "308" => Some(headers_and_compression(None, None, false)),
            _ => None,
        }
    }

    fn error_headers(&self) -> &'static [Line] {
        default_headers()
    }
}

fn default_headers() -> &'static [Line] {
    DEFAULT_HEADERS.as_slice()
}

fn headers_and_compression(
    content_type: Option<&'static [u8]>,
    cache_control: Option<&'static [u8]>,
    compressible: bool,
) -> HeadersAndCompression {
    let mut headers = default_headers().to_vec();
    if let Some(content_type) = content_type {
        headers.push(Line::with_slice_value(CONTENT_TYPE, content_type));
    }
    if let Some(cache_control) = cache_control {
        headers.push(Line::with_slice_value(CACHE_CONTROL, cache_control));
    }
    HeadersAndCompression {
        headers,
        compressible,
        redirection: content_type.is_none(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zip = download(&zip_download_branch_url(
        "programingjd",
        "about.programingjd.me",
        "main",
    ))
    .await?;
    let listener = TcpListener::bind(("127.0.0.1", 8080u16)).await?;
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
                            Ok::<hyper::Response<Either<Full<Bytes>, Empty<Bytes>>>, Infallible>(
                                handler.handle_hyper_request(request),
                            )
                        }
                    }),
                )
                .await;
        });
    }
}
