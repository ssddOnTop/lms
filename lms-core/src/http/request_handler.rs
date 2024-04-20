use crate::app_ctx::AppContext;
use crate::authdb::auth_db::AuthDB;
use anyhow::{Context, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response};
use lazy_static::lazy_static;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn handle_request(
    req: Request<Incoming>,
    app_ctx: Arc<AppContext>,
    auth_db: Arc<RwLock<AuthDB>>,
) -> Result<Response<Full<Bytes>>> {
    match *req.method() {
        Method::GET => handle_get(req, app_ctx).await,
        Method::POST => handle_post(req, app_ctx, auth_db).await,
        _ => not_found(),
    }
}

async fn into_bytes(req: Request<Incoming>) -> Result<Bytes> {
    let bytes = req
        .into_body()
        .frame()
        .await
        .context("unable to extract frame")??
        .into_data()
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    Ok(bytes)
}

async fn handle_post(
    req: Request<Incoming>,
    _app_ctx: Arc<AppContext>,
    auth_db: Arc<RwLock<AuthDB>>,
) -> Result<Response<Full<Bytes>>> {
    let path = req.uri().path().to_string();
    let body = into_bytes(req).await?;
    match path.as_str() {
        "/auth" => auth_db
            .write()
            .await
            .handle_request(body)
            .await
            .into_hyper_response(),
        &_ => not_found(),
    }
}

/// Get requests should return a html response
async fn handle_get(
    req: hyper::Request<hyper::body::Incoming>,
    _app_ctx: Arc<AppContext>,
) -> Result<Response<Full<Bytes>>> {
    let path = req.uri().path();
    match path {
        &_ => not_found(),
    }
}

lazy_static! {
    static ref PAGE_404: String = {
        let html = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../html/notfound.html"
        ));
        let routes = serde_json::json!({
            "available_routes": ["home", "signon"],
            "home": "It contains Homepage",
            "signon": "Allows user to login or signup",
        });
        let routes = routes.to_string();
        let html = html.replace("SOME_UNIQUE_STRING_TO_BE_REPLACED", routes.as_str());
        html
    };
}

fn not_found() -> Result<Response<Full<Bytes>>> {
    let response = Response::builder()
        .status(404)
        .header("Content-Type", "text/html")
        .body(Full::new(Bytes::from(PAGE_404.as_str())))?;
    Ok(response)
}

#[cfg(test)]
mod test {
    use crate::http::request_handler::PAGE_404;
    use anyhow::Result;

    #[tokio::test]
    async fn contains_all_routes() -> Result<()> {
        assert!(PAGE_404.as_str().contains("home"));
        assert!(PAGE_404.as_str().contains("signon"));
        Ok(())
    }
}
