use crate::to_anyhow;
use lms_core::Instance;

pub struct WasmInstance {}

impl Instance for WasmInstance {
    fn now(&self) -> anyhow::Result<u128> {
        let timer = wasm_timer::SystemTime::now();
        let duration = timer
            .duration_since(wasm_timer::UNIX_EPOCH)
            .map_err(|_| to_anyhow("Cur time generation failed"))?;
        Ok(duration.as_millis())
    }
}

impl WasmInstance {
    pub fn init() -> Self {
        Self {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn test_now() {
        let instance = WasmInstance {};
        let now = instance.now().unwrap();
        assert!(now > 0);
    }
}
