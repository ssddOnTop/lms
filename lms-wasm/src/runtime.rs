use crate::{env, file, http, instance};
use anyhow::anyhow;
use lms_core::runtime::TargetRuntime;
use lms_core::{EnvIO, FileIO, HttpIO, Instance};
use std::rc::Rc;
use std::sync::Arc;

fn init_file(env: Rc<worker::Env>, bucket_id: &str) -> anyhow::Result<Arc<dyn FileIO>> {
    Ok(Arc::new(file::WasmFileIO::init(env, bucket_id)?))
}

fn init_http() -> Arc<dyn HttpIO> {
    Arc::new(http::WasmHttp::init())
}
fn init_inst() -> Arc<dyn Instance> {
    Arc::new(instance::WasmInstance::init())
}
fn init_env(env: Rc<worker::Env>) -> Arc<dyn EnvIO> {
    Arc::new(env::WasmEnv::init(env))
}

pub fn init(env: Rc<worker::Env>) -> anyhow::Result<TargetRuntime> {
    let http = init_http();
    let instance = init_inst();
    let env_io = init_env(env.clone());

    let bucket_id = env_io
        .get("BUCKET")
        .ok_or(anyhow!("BUCKET var is not set"))?;

    Ok(TargetRuntime {
        http: http.clone(),
        file: init_file(env.clone(), &bucket_id)?,
        env: env_io,
        instance,
    })
}
