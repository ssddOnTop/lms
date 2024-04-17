use crate::blueprint::Blueprint;
use crate::runtime::TargetRuntime;

pub struct AppContext {
    pub blueprint: Blueprint,
    pub runtime: TargetRuntime,
}
