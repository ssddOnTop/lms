use crate::cli::rt;
use lms_core::app_ctx::AppContext;
use lms_core::blueprint::Blueprint;
use std::net::SocketAddr;
use std::sync::Arc;

pub struct ServerConfig {
    pub app_ctx: Arc<AppContext>,
}
impl ServerConfig {
    pub async fn new(blueprint: Blueprint) -> Self {
        let app_ctx = AppContext {
            runtime: rt::init(),
            blueprint,
        };
        let app_ctx = Arc::new(app_ctx);

        Self { app_ctx }
    }

    pub fn addr(&self) -> SocketAddr {
        (
            self.app_ctx.blueprint.server.hostname,
            self.app_ctx.blueprint.server.port,
        )
            .into()
    }
    pub fn playground_url(&self) -> String {
        self.addr().to_string()
    }
}
