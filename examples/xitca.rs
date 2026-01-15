use reqwest::Client;
use std::borrow::Borrow;
use std::sync::Arc;
use xitca_http::body::ResponseBody;
use xitca_http::http::RequestExt;
use xitca_http::{util::service::handler::handler_service, Request, Response};
use xitca_web::route::get;
use xitca_web::{App, WebContext};
use zip_static_handler::github::zip_download_branch_url;
use zip_static_handler::handler::Handler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter("zip_static_handler=info,axum::rejection=trace")
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
    let state = State(Arc::new(
        Handler::builder()
            .with_zip_prefix("about.programingjd.me-main/")
            .with_zip(zip)
            .try_build()?,
    ));
    App::new()
        .with_state(state)
        .at("/*", get(handler_service(static_handler)))
        .serve()
        .bind("127.0.0.1:8080")?
        .run()
        .await?;
    Ok(())
}

#[derive(Clone)]
struct State(Arc<Handler>);

impl Borrow<Handler> for State {
    fn borrow(&self) -> &Handler {
        self.0.as_ref()
    }
}

async fn static_handler<E>(
    req: Request<RequestExt<E>>,
    ctx: &WebContext<'_, State>,
) -> Response<ResponseBody> {
    let handler: &Handler = ctx.state().borrow();
    handler.handle_xitca_request(req)
}

async fn download(url: &str) -> Result<Vec<u8>, reqwest::Error> {
    let response = Client::default().get(url).send().await?;
    if !response.status().is_success() {
        panic!("failed to download {url} ({})", response.status().as_str());
    }
    Ok(response.bytes().await?.to_vec())
}
