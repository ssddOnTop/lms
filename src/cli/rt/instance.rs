use lms_core::Instance;
use std::time::SystemTime;

#[derive(Default, Clone)]
pub struct NativeInstance {}

impl Instance for NativeInstance {
    fn now(&self) -> anyhow::Result<u128> {
        Ok(SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    #[test]
    fn test_now() {
        let instance = NativeInstance::default();
        let now = instance.now().unwrap();
        let system_now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        assert!(now <= system_now);
    }
}
