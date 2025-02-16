use reqwest::Client;
use salvo::conn::TcpListener;
use salvo::logging::Logger;
use salvo::{handler, Listener, Router, Server, Service};
use zip_static_handler::github::zip_download_branch_url;
use zip_static_handler::handler::Handler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter("zip_static_handler=info,salvo=trace")
        .without_time()
        .with_line_number(false)
        .with_file(false)
        .try_init()
        .expect("could not init tracing subscriber");
    let zip = download(&zip_download_branch_url(
        "programingjd",
        "about.programingjd.me",
        "main",
    ))
    .await?;

    let state = Handler::builder()
        .with_zip_prefix("about.programingjd.me-main/")
        .with_zip(zip)
        .try_build()?;

    let router = Router::new()
        .push(Router::with_path("version").get(version))
        .push(Router::with_path("{**path}").get(state));

    let acceptor = TcpListener::new("127.0.0.1:8080").bind().await;
    Server::new(acceptor)
        .serve(Service::new(router).hoop(Logger::new()))
        .await;
    Ok(())
}

#[handler]
async fn version() -> &'static str {
    "1.0"
}

async fn download(url: &str) -> Result<Vec<u8>, reqwest::Error> {
    let response = Client::default().get(url).send().await?;
    if !response.status().is_success() {
        panic!("failed to download {url} ({})", response.status().as_str());
    }
    Ok(response.bytes().await?.to_vec())
}
