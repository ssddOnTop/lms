pub mod request;
pub mod request_handler;
pub mod response;

pub(super) const AUTH_PAGE: &str = include_str!(concat!(lms_macros::include_path!(), "login.html"));
pub(super) const INDEX_JS: &str = include_str!(concat!(lms_macros::include_path!(), "index.js"));
