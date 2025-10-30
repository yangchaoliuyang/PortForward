use aes_gcm::{
    aead::{Aead, KeyInit, generic_array::GenericArray},
    Aes256Gcm
};

use aes_gcm::aead::generic_array::typenum::U12;
use std::io;
// 固定密钥和固定 nonce
const FIXED_KEY: [u8; 32] = [
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
    0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
    0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
    0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
];

const FIXED_NONCE: [u8; 12] = [
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
];

pub struct SimpleEncryptionContext {
    cipher: Aes256Gcm,
    nonce: GenericArray<u8, U12>,
}

impl SimpleEncryptionContext {
    pub fn new() -> Self {
        let key = GenericArray::from_slice(&FIXED_KEY);
        let cipher = Aes256Gcm::new(key);
        let nonce = GenericArray::clone_from_slice(&FIXED_NONCE);
        Self { cipher, nonce }
    }

    pub fn encrypt(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        self.cipher.encrypt(&self.nonce, data)
            .map_err(|_e| io::Error::new(io::ErrorKind::Other, "Encryption failed"))
    }

    pub fn decrypt(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        self.cipher.decrypt(&self.nonce, data)
            .map_err(|_e| io::Error::new(io::ErrorKind::Other, "Decryption failed"))
    }
}



// 加密数据并添加4字节长度头
pub async fn encrypt_and_prepend_length(
    data: &[u8],
    ctx: &SimpleEncryptionContext,
) -> io::Result<Vec<u8>> {
    // 1. 加密数据
    let encrypted_data = ctx.encrypt(data)?;
    

    // 2. 添加4字节长度头
    let data_len = encrypted_data.len() as u32;
    let mut result = Vec::with_capacity(4 + encrypted_data.len());
    result.extend_from_slice(&data_len.to_be_bytes());
    result.extend_from_slice(&encrypted_data);
    
    Ok(result)
}
