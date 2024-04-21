use crate::cli::server::server_config::ServerConfig;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::Request;
use lms_core::http::request_handler::handle_request;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

pub async fn run(
    sc: Arc<ServerConfig>,
    server_up_sender: Option<oneshot::Sender<()>>,
) -> anyhow::Result<()> {
    let addr = sc.addr();
    let listener = TcpListener::bind(addr).await?;
    if let Some(sender) = server_up_sender {
        sender
            .send(())
            .or(Err(anyhow::anyhow!("Failed to send message")))?;
    }
    log::info!("Listening on: http://{}", addr);
    loop {
        let stream_result = listener.accept().await;
        match stream_result {
            Ok((stream, _)) => {
                let io = hyper_util::rt::TokioIo::new(stream);
                let sc = sc.clone();
                tokio::spawn(async move {
                    let server = hyper::server::conn::http1::Builder::new()
                        .serve_connection(
                            io,
                            service_fn(move |req: Request<Incoming>| {
                                handle_request(req, sc.auth_db.clone(), sc.actions_db.clone())
                            }),
                        )
                        .await;
                    if let Err(e) = server {
                        log::error!("An error occurred while handling a request: {e}");
                    }
                });
            }
            Err(e) => log::error!("An error occurred while handling request: {e}"),
        }
    }
}
