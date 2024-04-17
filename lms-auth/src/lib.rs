pub mod auth;
mod local_crypto;

pub fn is_default<T: Default + Eq>(val: &T) -> bool {
    *val == T::default()
}