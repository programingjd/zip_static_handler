use reqwest::Client;
use rocket::http::Method::{Get, Head};
use rocket::shield::{Frame, NoSniff, Shield};
use rocket::{Config, Route};
use std::error::Error;
use std::sync::Arc;
use zip_static_handler::github::zip_download_branch_url;
use zip_static_handler::handler::Handler;
use zip_static_handler::rocket::HandlerAdapter;

#[rocket::main]
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
    let head = Route::new(Head, "/<path..>", HandlerAdapter);
    let get = Route::new(Get, "/<path..>", HandlerAdapter);
    let rocket = rocket::build()
        .configure(Config {
            address: [127, 0, 0, 1].into(),
            port: 8080,
            ..Config::default()
        })
        .manage(state)
        // X-Content-Type-Options: nosniff and X-Frame-Options: deny
        // are already set by the handler.
        .attach(Shield::new().disable::<NoSniff>().disable::<Frame>())
        .mount("/", vec![head, get]);
    rocket.launch().await?;
    Ok(())
}

async fn download(url: &str) -> Result<Vec<u8>, reqwest::Error> {
    let response = Client::default().get(url).send().await?;
    if !response.status().is_success() {
        panic!("failed to download {url} ({})", response.status().as_str());
    }
    Ok(response.bytes().await?.to_vec())
}
