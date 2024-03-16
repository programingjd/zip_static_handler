use min_http11_parser::parser::Parser;
use reqwest::Client;
use std::sync::Arc;
use tokio::io::{split, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpListener;
use zip_static_handler::github::zip_download_branch_url;
use zip_static_handler::handler::Handler;

#[tokio::main]
async fn main() {
    let zip = download(&zip_download_branch_url(
        "programingjd",
        "about.programingjd.me",
        "main",
    ))
    .await
    .expect("failed to download zip");
    let handler = Arc::new(
        Handler::builder()
            .with_zip_prefix("about.programingjd.me-main/")
            .with_zip(zip)
            .try_build()
            .expect("failed to parse zip"),
    );
    let listener = TcpListener::bind(("127.0.0.1", 8080u16))
        .await
        .expect("failed to bind");
    tcp_accept_loop(listener, handler).await;
}

async fn tcp_accept_loop(listener: TcpListener, handler: Arc<Handler>) {
    loop {
        async {
            let (tcp_stream, _remote_address) = match listener.accept().await {
                Err(_) => return,
                Ok(it) => it,
            };
            let (reader, writer) = split(tcp_stream);
            let handler = handler.clone();
            tokio::spawn(async move {
                request_loop(reader, writer, handler).await;
            });
        }
        .await;
    }
}

async fn request_loop(
    mut reader: (impl AsyncRead + Unpin + Sized),
    mut writer: (impl AsyncWrite + Unpin + Sized),
    handler: Arc<Handler>,
) {
    let parser = Parser::default();
    let mut reader = BufReader::new(&mut reader);
    let mut writer = BufWriter::new(&mut writer);
    let mut buffer = vec![];
    loop {
        let cont = handler
            .async_handle(&parser, &mut reader, &mut writer, &mut buffer)
            .await
            .is_some();
        if writer.flush().await.is_err() || !cont {
            break;
        }
    }
}

async fn download(url: &str) -> Result<Vec<u8>, reqwest::Error> {
    let response = Client::default().get(url).send().await?;
    if !response.status().is_success() {
        panic!("failed to download {url} ({})", response.status().as_str());
    }
    Ok(response.bytes().await?.to_vec())
}
