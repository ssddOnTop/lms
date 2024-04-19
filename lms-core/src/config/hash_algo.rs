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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_into_totp_sha1() {
        let algo = Algorithm::SHA1;
        assert_eq!(algo.into_totp(), totp_rs::Algorithm::SHA1);
    }

    #[test]
    fn test_into_totp_sha256() {
        let algo = Algorithm::Sha256;
        assert_eq!(algo.into_totp(), totp_rs::Algorithm::SHA256);
    }

    #[test]
    fn test_into_totp_sha512() {
        let algo = Algorithm::Sha512;
        assert_eq!(algo.into_totp(), totp_rs::Algorithm::SHA512);
    }
}
