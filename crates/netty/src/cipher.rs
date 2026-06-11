//! AES-128 / CFB8 cipher pair as a pipeline handler.
//!
//! Mirrors the cipher pair `proxy_core::net::connection::Cfb8State`
//! manages, but exposed as a `ChannelHandler` so the netty-style
//! pipeline can carry it. Used by plugins that want to assemble
//! their own protocol experiments without rebuilding the cipher
//! plumbing.

use aes::Aes128;
use cfb_mode::{BufDecryptor, BufEncryptor};
use cipher::KeyIvInit;

pub struct CipherState {
    enc: BufEncryptor<Aes128>,
    dec: BufDecryptor<Aes128>,
}

impl CipherState {
    pub fn new(shared_secret: &[u8]) -> Result<Self, super::error::CipherError> {
        if shared_secret.len() != 16 {
            return Err(super::error::CipherError::InvalidLength(
                shared_secret.len(),
            ));
        }
        let key: &[u8; 16] = shared_secret.try_into().expect("length checked above");
        let iv: &[u8; 16] = shared_secret.try_into().expect("length checked above");
        Ok(Self {
            enc: BufEncryptor::<Aes128>::new(key.into(), iv.into()),
            dec: BufDecryptor::<Aes128>::new(key.into(), iv.into()),
        })
    }

    pub fn encrypt(&mut self, data: &mut [u8]) {
        self.enc.encrypt(data);
    }

    pub fn decrypt(&mut self, data: &mut [u8]) {
        self.dec.decrypt(data);
    }
}
