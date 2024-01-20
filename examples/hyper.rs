use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use reqwest::Client;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::spawn;
use zip_static_handler::github::zip_download_branch_url;
use zip_static_handler::handler::Handler;

async fn download(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let response = Client::default().get(url).send().await?;
    if !response.status().is_success() {
        panic!("failed to download {url} ({})", response.status().as_str());
    }
    Ok(response.bytes().await?.to_vec())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
            .map_err(|err| err.boxed())?,
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
                        async move { handler.handle_request(request).map_err(|err| err.boxed()) }
                    }),
                )
                .await;
        });
    }
}
