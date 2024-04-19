use crate::cli::rt;
use anyhow::Result;
use lms_core::app_ctx::AppContext;
use lms_core::authdb::auth_db::AuthDB;
use lms_core::blueprint::Blueprint;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ServerConfig {
    pub app_ctx: Arc<AppContext>,
    pub auth_db: Arc<RwLock<AuthDB>>,
}
impl ServerConfig {
    pub async fn new(blueprint: Blueprint) -> Result<Self> {
        let users = blueprint.extensions.users.clone();
        let app_ctx = AppContext {
            runtime: rt::init(),
            blueprint,
        };
        let app_ctx = Arc::new(app_ctx);
        let auth_db = AuthDB::init(app_ctx.clone(), users).await?;
        let auth_db = Arc::new(RwLock::new(auth_db));

        Ok(Self { app_ctx, auth_db })
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
