use actix_web::body::BoxBody;
use actix_web::web::Data;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use reqwest::Client;
use std::sync::Arc;
use zip_static_handler::errors::Error;
use zip_static_handler::github::zip_download_branch_url;
use zip_static_handler::handler::Handler;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let zip = download(&zip_download_branch_url(
        "programingjd",
        "about.programingjd.me",
        "main",
    ))
    .await
    .map_err(|err| Error::Wrapped(err))?;
    let state = Arc::new(
        Handler::builder()
            .with_zip_prefix("about.programingjd.me-main/")
            .with_zip(zip)
            .try_build()?,
    );
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(state.clone()))
            .default_service(web::route().to(static_handler))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

async fn static_handler(state: Data<Arc<Handler>>, request: HttpRequest) -> HttpResponse<BoxBody> {
    state.handle_request(request).unwrap()
}

async fn download(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let response = Client::default().get(url).send().await?;
    if !response.status().is_success() {
        panic!("failed to download {url} ({})", response.status().as_str());
    }
    Ok(response.bytes().await?.to_vec())
}
