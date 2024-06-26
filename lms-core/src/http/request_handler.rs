use crate::actions_db::actions_db::ActionsDB;
use crate::authdb::auth_db::AuthDB;
use crate::http::request::Request;
use anyhow::Result;
use bytes::Bytes;
use http_body_util::Full;

use super::{AUTH_PAGE, INDEX_JS};
use crate::app_ctx::AppContext;
use crate::authdb::auth_actors::Authority;
use hyper::{Method, Response};
use lazy_static::lazy_static;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn handle_request(
    req: Request,
    app_context: Arc<AppContext>,
    auth_db: Arc<RwLock<AuthDB>>,
    actions_db: Arc<ActionsDB>,
) -> Result<Response<Full<Bytes>>> {
    log::info!("Request: {} {}", req.method, req.url.path());
    match req.method {
        Method::GET => handle_get(req, app_context).await,
        Method::POST => handle_post(req, auth_db, actions_db).await,
        _ => not_found(),
    }
}

async fn handle_post(
    req: Request,
    auth_db: Arc<RwLock<AuthDB>>,
    actions_db: Arc<ActionsDB>,
) -> Result<Response<Full<Bytes>>> {
    let path = req.url.path().to_string();
    match path.as_str() {
        "/auth" => auth_db
            .write()
            .await
            .handle_request(req.body)
            .await
            .into_hyper_response(),
        "/fs" => actions_db
            .handle_request(req.body)
            .await
            .into_hyper_response(),
        &_ => not_found(),
    }
}

/// Get requests should return a html response
async fn handle_get(req: Request, app_context: Arc<AppContext>) -> Result<Response<Full<Bytes>>> {
    let path = req.url.path();
    match path {
        "/getauthority" => {
            let authority = schemars::schema_for!(Authority);
            let authority = authority.schema.enum_values.unwrap();
            let authority = serde_json::to_string(&authority)?;
            let response = Response::builder()
                .status(200)
                .header("Access-Control-Allow-Origin", "*")
                .body(Full::new(Bytes::from(authority)))?;
            Ok(response)
        }
        "/getbatches" => {
            let batches = serde_json::to_string(&app_context.blueprint.batch_info)?;
            let response = Response::builder()
                .status(200)
                .header("Access-Control-Allow-Origin", "*")
                .body(Full::new(Bytes::from(batches)))?;
            Ok(response)
        }
        "/auth" => {
            let response = Response::builder()
                .status(200)
                .header("Access-Control-Allow-Origin", "*")
                .body(Full::new(Bytes::from(AUTH_PAGE)))?;
            Ok(response)
        }

        "/index.js" => {
            let response = Response::builder()
                .status(200)
                .header("Access-Control-Allow-Origin", "*")
                // .header("Content-Type", "application/js")
                .body(Full::new(Bytes::from(INDEX_JS)))?;
            Ok(response)
        }
        "/helloworld" => {
            let response = Response::builder()
                .status(200)
                .header("Content-Type", "text/html")
                .body(Full::new(Bytes::from("Hello World!")))?;
            Ok(response)
        }
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
