use log::{info, warn};
use num_bigint_dig::*;
use rsa::{hash, PaddingScheme, PublicKey, RSAPublicKey};
use sha2::*;

pub fn verify_signature(data: &[u8], signature: &[u8], public_key: &[u8]) -> bool {
    let mut hasher = Sha256::new();
    hasher.input(&data);
    let hash = hasher.result();
    let public_key = match RSAPublicKey::new(BigUint::from_bytes_be(&public_key), 0x10001u32.into())
    {
        Ok(public_key) => public_key,
        Err(e) => {
            warn!("RSA public key error: {}", e);
            return false;
        }
    };
    match public_key.verify(
        PaddingScheme::PKCS1v15,
        Some(&hash::Hashes::SHA2_256),
        &hash,
        &signature,
    ) {
        Ok(()) => true,
        Err(e) => {
            info!("RSA verification failure: {}", e);
            false
        }
    }
}
