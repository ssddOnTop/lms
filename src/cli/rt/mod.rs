use lms_core::runtime::TargetRuntime;
use std::sync::Arc;

mod file;
mod http;
mod instance;

pub fn init() -> TargetRuntime {
    TargetRuntime {
        http: Arc::new(http::NativeHttp::default()),
        file: Arc::new(file::NativeFileIO::default()),
        instance: Arc::new(instance::NativeInstance::default()),
    }
}
