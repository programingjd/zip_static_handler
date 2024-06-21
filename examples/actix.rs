use actix_web::body::EitherBody;
use actix_web::web::Data;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use bytes::Bytes;
use reqwest::Client;
use std::error::Error;
use std::sync::Arc;
use zip_static_handler::github::zip_download_branch_url;
use zip_static_handler::handler::Handler;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
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
            .try_build()?,
    );
    Ok(HttpServer::new(move || {
        App::new()
            .app_data(Data::new(state.clone()))
            .default_service(web::route().to(static_handler))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?)
}

async fn static_handler(
    state: Data<Arc<Handler>>,
    request: HttpRequest,
) -> HttpResponse<EitherBody<Bytes, ()>> {
    state.handle_actix_request(request)
}

async fn download(url: &str) -> Result<Vec<u8>, reqwest::Error> {
    let response = Client::default().get(url).send().await?;
    if !response.status().is_success() {
        panic!("failed to download {url} ({})", response.status().as_str());
    }
    Ok(response.bytes().await?.to_vec())
}
