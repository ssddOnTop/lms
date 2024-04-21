use anyhow::anyhow;

mod env;
mod file;
mod handle;
mod http;
mod instance;
mod runtime;

#[worker::event(fetch)]
async fn fetch(
    req: worker::Request,
    env: worker::Env,
    ctx: worker::Context,
) -> anyhow::Result<worker::Response> {
    let result = handle::fetch(req, env, ctx).await;

    match result {
        Ok(response) => Ok(response),
        Err(message) => {
            log::error!("ServerError: {}", message.to_string());
            worker::Response::error(message.to_string(), 500).map_err(to_anyhow)
        }
    }
}

#[worker::event(start)]
fn start() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    init_log();
}

pub(crate) fn init_log() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
}

fn to_anyhow<T: std::fmt::Debug>(e: T) -> anyhow::Error {
    anyhow!("{:?}", e)
}
