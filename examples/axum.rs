use axum::extract::{Request, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use reqwest::Client;
use std::sync::Arc;
use tokio::net::TcpListener;
use zip_static_handler::github::zip_download_branch_url;
use zip_static_handler::handler::Handler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .compact()
        //.with_max_level(tracing_subscriber::filter::LevelFilter::TRACE)
        .with_env_filter("zip_static_handler=info,axum::rejection=trace")
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
    let state = Arc::new(
        Handler::builder()
            .with_zip_prefix("about.programingjd.me-main/")
            .with_zip(zip)
            .try_build()
            .map_err(|err| err.boxed())?,
    );
    axum::serve(listener, app().with_state(state)).await?;
    Ok(())
}

fn app() -> Router<Arc<Handler>> {
    Router::new()
        .route("/version", get(version_handler))
        .fallback(static_handler)
}

async fn version_handler() -> &'static str {
    "1.0"
}

async fn static_handler(State(state): State<Arc<Handler>>, request: Request) -> Response {
    state.handle_request(request).unwrap()
}

async fn download(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let response = Client::default().get(url).send().await?;
    if !response.status().is_success() {
        panic!("failed to download {url} ({})", response.status().as_str());
    }
    Ok(response.bytes().await?.to_vec())
}
