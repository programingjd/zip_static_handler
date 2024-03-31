use min_http11_parser::error::Error;
use min_http11_parser::method::Method;
use min_http11_parser::parser::Parser;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{split, AsyncBufRead, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpListener;
use zip_static_handler::github::zip_download_branch_url;
use zip_static_handler::handler::Handler;
use zip_static_handler::http::response::StatusCode;

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

// Keep-alive header returns 60 (not configurable), and we add 5s of leeway.
pub const KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(65);

async fn request_loop(
    mut reader: (impl AsyncRead + Unpin + Sized),
    mut writer: (impl AsyncWrite + Unpin + Sized),
    handler: Arc<Handler>,
) {
    let parser = Parser::default().with_request_line_read_timeout(KEEP_ALIVE_TIMEOUT);
    let mut reader = BufReader::new(&mut reader);
    let mut writer = BufWriter::new(&mut writer);
    let mut buffer1 = vec![];
    let mut buffer2 = vec![];
    while let Ok((method, path)) = parser.parse_request_line(&mut reader, &mut buffer1).await {
        match path {
            b"/healthcheck" => {
                if handle_healthcheck(
                    handler.as_ref(),
                    &method,
                    &parser,
                    &mut reader,
                    &mut writer,
                    &mut buffer2,
                )
                .await
                .is_some()
                {
                    if writer.flush().await.is_err() {
                        break;
                    }
                } else {
                    let _ = writer.flush().await;
                    break;
                }
            }
            _ => {
                if handler
                    .handle_path(
                        &method,
                        path,
                        &parser,
                        &mut reader,
                        &mut writer,
                        &mut buffer2,
                    )
                    .await
                    .is_some()
                {
                    if writer.flush().await.is_err() {
                        break;
                    }
                } else {
                    let _ = writer.flush().await;
                    break;
                }
            }
        }
    }
}

async fn handle_healthcheck<R: AsyncBufRead + Unpin, W: AsyncWrite + Unpin>(
    handler: &Handler,
    method: &Method,
    parser: &Parser,
    reader: &mut R,
    writer: &mut W,
    buffer: &mut Vec<u8>,
) -> Option<()> {
    if *method != Method::Head {
        handler
            .write_status_line(writer, StatusCode::MethodNotAllowed)
            .await?;
        handler.write_error_headers(writer, false).await?;
        let _ = writer.flush().await;
        None
    } else {
        match parser.parse_headers(reader, buffer).await {
            Err(Error::ReadTimeout) => None,
            Err(Error::RequestTooLarge) => {
                handler
                    .write_status_line(writer, StatusCode::RequestTooLarge)
                    .await?;
                handler.write_error_headers(writer, true).await?;
                None
            }
            Err(Error::BadRequest) => {
                handler
                    .write_status_line(writer, StatusCode::BadRequest)
                    .await?;
                handler.write_error_headers(writer, true).await?;
                None
            }
            Err(_) => unimplemented!(),
            Ok(_) => {
                handler.write_status_line(writer, StatusCode::OK).await?;
                handler.write_error_headers(writer, true).await?;
                Some(())
            }
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
