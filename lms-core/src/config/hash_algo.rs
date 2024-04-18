use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum Algorithm {
    #[default]
    SHA1,
    Sha256,
    Sha512,
}

impl Algorithm {
    pub fn into_totp(self) -> totp_rs::Algorithm {
        match self {
            Algorithm::SHA1 => totp_rs::Algorithm::SHA1,
            Algorithm::Sha256 => totp_rs::Algorithm::SHA256,
            Algorithm::Sha512 => totp_rs::Algorithm::SHA512,
        }
    }
}
