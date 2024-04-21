use crate::http::{to_request, to_response};
use crate::runtime;
use http_body_util::Full;
use lazy_static::lazy_static;
use lms_core::actions_db::actions_db::ActionsDB;
use lms_core::app_ctx::AppContext;
use lms_core::authdb::auth_db::AuthDB;
use lms_core::blueprint::Blueprint;
use lms_core::config::reader::ConfigReader;
use lms_core::http::request::Request;
use lms_core::http::request_handler::handle_request;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

lazy_static! {
    static ref APP_CTX: RwLock<Option<(String, Arc<WasmContext>)>> = RwLock::new(None);
}

struct WasmContext {
    pub auth_db: Arc<tokio::sync::RwLock<AuthDB>>,
    pub actions_db: Arc<ActionsDB>,
}

pub async fn fetch(
    req: worker::Request,
    env: worker::Env,
    _: worker::Context,
) -> anyhow::Result<worker::Response> {
    log::info!(
        "{} {:?}",
        req.method().to_string(),
        req.url().map(|u| u.to_string())
    );

    let req = to_request(req).await?;

    let env = Rc::new(env);
    let wasm_ctx = match get_app_ctx(env, &req).await? {
        Ok(app_ctx) => app_ctx,
        Err(e) => return to_response(e).await,
    };
    let resp = handle_request(req, wasm_ctx.auth_db.clone(), wasm_ctx.actions_db.clone()).await?;
    to_response(resp).await
}

/// Initializes the worker once and caches the app context
/// for future requests.
async fn get_app_ctx(
    env: Rc<worker::Env>,
    req: &Request,
) -> anyhow::Result<Result<Arc<WasmContext>, hyper::Response<Full<bytes::Bytes>>>> {
    // Read context from cache
    let file_path = req
        .url
        .query()
        .and_then(|x| serde_qs::from_str::<HashMap<String, String>>(x).ok())
        .and_then(|x| x.get("config").cloned());

    if let Some(file_path) = &file_path {
        if let Some(app_ctx) = read_app_ctx() {
            if app_ctx.0.eq(file_path) {
                log::info!("Using cached application context");
                return Ok(Ok(app_ctx.clone().1));
            }
        }
    }

    let config_path = match file_path {
        Some(path) => path,
        None => {
            return Ok(Err(hyper_resp(
                "No Config URL specified, pass using `?config=<lint/path>`",
            )));
        }
    };

    let runtime = runtime::init(env)?;
    let reader = ConfigReader::init(runtime.clone());
    let module = match reader.read(config_path).await {
        Ok(module) => module,
        Err(e) => {
            return Ok(Err(hyper_resp(format!("Failed to read config: {}", e))));
        }
    };

    let blueprint = match Blueprint::try_from(module) {
        Ok(blueprint) => blueprint,
        Err(e) => {
            return Ok(Err(hyper_resp(format!(
                "Unable to create blueprint: {}",
                e
            ))));
        }
    };

    let app_ctx = AppContext { blueprint, runtime };
    let app_ctx = Arc::new(app_ctx);

    let auth_db = AuthDB::init(app_ctx.clone()).await?;
    let auth_db = Arc::new(tokio::sync::RwLock::new(auth_db));

    let actions_db = ActionsDB::init(app_ctx).await?;
    let actions_db = Arc::new(actions_db);

    let wasm_ctx = WasmContext {
        auth_db,
        actions_db,
    };

    Ok(Ok(Arc::new(wasm_ctx)))
}

fn hyper_resp<T: AsRef<str>>(e: T) -> hyper::Response<Full<bytes::Bytes>> {
    let mut response = hyper::Response::new(Full::from(e.as_ref().as_bytes().to_vec()));
    *response.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
    response
}

fn read_app_ctx() -> Option<(String, Arc<WasmContext>)> {
    APP_CTX.read().unwrap().clone()
}
