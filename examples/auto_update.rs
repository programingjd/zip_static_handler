use axum::extract::{ConnectInfo, Request, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use http::StatusCode;
use reqwest::Client;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;
use tracing::{info, warn};
use zip_static_handler::github::zip_download_branch_url;
use zip_static_handler::handler::Handler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter("auto_update=info,zip_static_handler=info,axum::rejection=trace")
        .without_time()
        .with_line_number(false)
        .with_file(false)
        .try_init()
        .expect("could not init tracing subscriber");
    let listener = TcpListener::bind(("0.0.0.0", 8080u16)).await?;
    let zip = download(&zip_download_branch_url(
        "programingjd",
        "about.programingjd.me",
        "main",
    ))
    .await?;
    let handler = Arc::new(RwLock::new(
        Handler::builder()
            .with_zip_prefix("about.programingjd.me-main/")
            .with_zip(zip)
            .try_build()
            .map_err(|err| err.boxed())?,
    ));
    axum::serve(
        listener,
        Router::new()
            .route("/update_webhook", get(update_webhook))
            .fallback(static_handler)
            .with_state(handler)
            .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

fn is_authorized(_request: &Request) -> bool {
    // check for authorization header
    false
}

async fn update_webhook(
    State(state): State<Arc<RwLock<Handler>>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
) -> Response {
    if !addr.ip().is_loopback() && !is_authorized(&request) {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(().into())
            .unwrap();
    }
    let zip = if let Ok(zip) = download(&zip_download_branch_url(
        "programingjd",
        "about.programingjd.me",
        "main",
    ))
    .await
    {
        zip
    } else {
        warn!("Failed to download zip.");
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(().into())
            .unwrap();
    };
    let handler = if let Ok(handler) = Handler::builder()
        .with_zip_prefix("about.programingjd.me-main/")
        .with_zip(zip)
        .try_build()
    {
        handler
    } else {
        warn!("Failed to build handler.");
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(().into())
            .unwrap();
    };
    let mut guard = state.write().unwrap();
    *guard = handler;
    info!("Handler updated");
    Response::builder()
        .status(StatusCode::OK)
        .body(().into())
        .unwrap()
}

async fn static_handler(State(state): State<Arc<RwLock<Handler>>>, request: Request) -> Response {
    if let Ok(handler) = state.read() {
        handler.handle_request(request).unwrap()
    } else {
        Response::builder().status(500).body(().into()).unwrap()
    }
}

async fn download(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let response = Client::default().get(url).send().await?;
    if !response.status().is_success() {
        panic!("failed to download {url} ({})", response.status().as_str());
    }
    Ok(response.bytes().await?.to_vec())
}
