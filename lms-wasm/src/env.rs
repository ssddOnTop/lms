use std::borrow::Cow;
use std::rc::Rc;

use lms_core::EnvIO;
use worker::Env;

pub struct WasmEnv {
    env: Rc<Env>,
}

unsafe impl Send for WasmEnv {}
unsafe impl Sync for WasmEnv {}

impl EnvIO for WasmEnv {
    fn get(&self, key: &str) -> Option<Cow<'_, str>> {
        self.env.var(key).ok().map(|v| Cow::from(v.to_string()))
    }
}

impl WasmEnv {
    pub fn init(env: Rc<Env>) -> Self {
        Self { env }
    }
}
