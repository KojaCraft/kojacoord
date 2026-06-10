//! Session-key crypto for the Notchian login handshake.
//!
//! The proxy generates a 2048-bit RSA keypair at startup, advertises
//! the public key in `EncryptionRequest`, and decrypts the
//! `EncryptionResponse` (shared-secret + verify-token) the client
//! returns. Everything after that point on the wire is AES-128 in
//! CFB8 mode with the shared secret as both key and IV — that's the
//! `Cfb8State` machinery in `net::connection`.

use aes::Aes128;
use cfb_mode::{BufDecryptor, BufEncryptor};
use cipher::KeyIvInit;
use rand::RngCore;
use rsa::{pkcs8::EncodePublicKey, RsaPrivateKey, RsaPublicKey};

use crate::error::AuthError;

pub fn generate_rsa_keypair() -> Result<RsaPrivateKey, rsa::errors::Error> {
    let mut rng = rand::rngs::OsRng;
    RsaPrivateKey::new(&mut rng, 2048)
}

pub fn public_key_der(private_key: &RsaPrivateKey) -> Result<Vec<u8>, AuthError> {
    RsaPublicKey::from(private_key)
        .to_public_key_der()
        .map(|doc| doc.into_vec())
        .map_err(|e| AuthError::EncryptionSetupFailed(e.to_string()))
}

pub fn generate_verify_token() -> [u8; 4] {
    let mut token = [0u8; 4];
    rand::rngs::OsRng.fill_bytes(&mut token);
    token
}

pub fn rsa_decrypt(private_key: &RsaPrivateKey, ciphertext: &[u8]) -> Result<Vec<u8>, AuthError> {
    private_key
        .decrypt(rsa::pkcs1v15::Pkcs1v15Encrypt, ciphertext)
        .map_err(AuthError::RsaDecryptionFailed)
}

pub type Aes128CfbEnc = BufEncryptor<Aes128>;
pub type Aes128CfbDec = BufDecryptor<Aes128>;

pub fn init_aes_cfb8(shared_secret: &[u8]) -> Result<(Aes128CfbEnc, Aes128CfbDec), AuthError> {
    let key: &[u8; 16] = shared_secret.try_into().map_err(|_| {
        AuthError::EncryptionSetupFailed(format!("expected 16 bytes, got {}", shared_secret.len()))
    })?;
    let iv: &[u8; 16] = key;
    let enc = Aes128CfbEnc::new(key.into(), iv.into());
    let dec = Aes128CfbDec::new(key.into(), iv.into());
    Ok((enc, dec))
}

pub fn decrypt_pkcs1v15(key: &RsaPrivateKey, data: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
    use rsa::Pkcs1v15Encrypt;
    Ok(key.decrypt(Pkcs1v15Encrypt, data)?)
}
