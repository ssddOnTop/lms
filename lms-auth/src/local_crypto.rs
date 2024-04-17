use totp_rs::TOTP;
use anyhow::Result;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use sha2::{Digest, Sha256};

const IV: [u8; 16] = [7; 16];

pub fn gen_totp(totp: &TOTP) -> Result<String> {
    Ok(totp.generate_current()?)
}

pub fn verify_totp(totp: &TOTP, code: &str) -> Result<bool> {
    Ok(totp.check_current(code)?)
}

pub fn encrypt_aes(key: &[u8], data: &str) -> Result<String> {
    let key: &[u8; 16] = key.try_into()?;
    let cipher  = libaes::Cipher::new_128(key);
    let encrypted  = cipher.cbc_encrypt(&IV, data.as_bytes());
    let encrypted = BASE64_STANDARD.encode(encrypted);
    Ok(encrypted)
}

pub fn decrypt_aes<T: AsRef<[u8]>>(key: &[u8], data: T) -> Result<String> {
    let key: &[u8; 16] = key.try_into()?;
    let cipher  = libaes::Cipher::new_128(key);
    let data = BASE64_STANDARD.decode(data.as_ref())?;
    let decrypted = cipher.cbc_decrypt(&IV, &data);
    Ok(String::from_utf8(decrypted)?)
}

pub fn hash_256<T: AsRef<str>>(data: T) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_ref().as_bytes());
    let data = hasher.finalize();
    let data = hex::encode(data);
    data
}

#[cfg(test)]
mod tests {
    use totp_rs::{Algorithm, Secret};
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = [1; 16];
        let data = "hello world";
        let encrypted = encrypt_aes(&key, data).unwrap();
        let decrypted = decrypt_aes(&key, &encrypted).unwrap();
        assert_eq!(data, decrypted);
    }

    #[test]
    fn test_gen_verify_totp() {
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            Secret::Raw("JBSWY3DPEHPK3PXP".as_bytes().to_vec()).to_bytes().unwrap(),
        ).unwrap();

        let generated_totp = gen_totp(&totp).unwrap();

        let is_valid = verify_totp(&totp, &generated_totp).unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_hash_256() {
        let data = "hello world";
        let hashed = hash_256(data);
        assert_eq!(hashed, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }
}