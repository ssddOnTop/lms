use crate::cli::rt;
use anyhow::Result;
use lms_core::actions_db::actions_db::ActionsDB;
use lms_core::app_ctx::AppContext;
use lms_core::authdb::auth_db::AuthDB;
use lms_core::blueprint::Blueprint;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ServerConfig {
    pub app_ctx: Arc<AppContext>,
    pub auth_db: Arc<RwLock<AuthDB>>,
    pub actions_db: Arc<ActionsDB>,
}
impl ServerConfig {
    pub async fn new(blueprint: Blueprint) -> Result<Self> {
        let app_ctx = AppContext {
            // avoid storing app ctx if it's not used anywhere
            runtime: rt::init(),
            blueprint,
        };
        let app_ctx = Arc::new(app_ctx);
        let auth_db = AuthDB::init(app_ctx.clone()).await?;
        let auth_db = Arc::new(RwLock::new(auth_db));

        let actions_db = ActionsDB::init(app_ctx.clone()).await?;
        let actions_db = Arc::new(actions_db);
        Ok(Self {
            app_ctx,
            auth_db,
            actions_db,
        })
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
