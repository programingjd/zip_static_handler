use min_http11_parser::error::Error;
use min_http11_parser::parser::Parser;
use reqwest::Client;
use rocket::yansi::Paint;
use std::sync::Arc;
use tokio::io::{copy, sink, split, AsyncRead, AsyncReadExt, AsyncWrite, BufReader, BufWriter};
use tokio::join;
use tokio::net::TcpListener;
use zip_static_handler::github::zip_download_branch_url;
use zip_static_handler::handler::Handler;
use zip_static_handler::min_http11::{Req, Resp};

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
    let listener = TcpListener::bind(("0.0.0.0", 8080u16))
        .await
        .expect("failed to bind");
    tcp_accept_loop(listener, handler).await;
}

async fn tcp_accept_loop(listener: TcpListener, handler: Arc<Handler>) {
    loop {
        async {
            let (tcp_stream, _remote_address) = match listener.accept().await {
                Err(err) => return,
                Ok(it) => it,
            };
            let (mut reader, mut writer) = split(tcp_stream);
            tokio::spawn(request_loop(reader, writer, handler.clone()));
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
    loop {
        if !async {
            let mut reader = BufReader::new(&mut reader);
            let mut writer = BufWriter::new(&mut writer);
            let mut buffer = vec![];
            let (method, path, known_headers, _) = match parser
                .parse_request_line_and_headers(&mut reader, &mut buffer)
                .await
            {
                Err(Error::ReadTimeout) => return false,
                Err(Error::UnsupportedVersion(_)) => return false,
                Err(Error::UnexpectedEndOfFile) => return false,
                Err(Error::RequestTooLarge) => {
                    respond_413_content_too_large(&mut writer).await;
                    return false;
                }
                Err(Error::UnknownMethod(_)) => {
                    respond_405_unsupported_method(&mut writer).await;
                    return false;
                }
                Err(Error::BadRequest) => {
                    respond_400_bad_request(&mut reader, &mut writer).await;
                    return true;
                }
                Err(_) => unimplemented!(),
                Ok(it) => it,
            };
            let req = Req {
                method,
                path,
                known_headers,
            };
            join!(
                async {
                    // let resp = handler
                    //     .handle_min_http11_request(req)
                    //     .unwrap_or_else(|_| Resp {
                    //         code: StatusCode::InternalServerError,
                    //     });
                },
                async {}
            );
            true
        }
        .await
        {
            break;
        }
    }
}

async fn respond_405_unsupported_method(mut writer: (impl AsyncWrite + Unpin + Sized)) {
    todo!()
}
async fn respond_400_bad_request(
    mut reader: (impl AsyncRead + Unpin + Sized),
    mut writer: (impl AsyncWrite + Unpin + Sized),
) {
    todo!()
}
async fn respond_413_content_too_large(mut writer: (impl AsyncWrite + Unpin + Sized)) {
    todo!()
}

async fn download(url: &str) -> Result<Vec<u8>, reqwest::Error> {
    let response = Client::default().get(url).send().await?;
    if !response.status().is_success() {
        panic!("failed to download {url} ({})", response.status().as_str());
    }
    Ok(response.bytes().await?.to_vec())
}
