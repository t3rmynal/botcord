use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD_NO_PAD as B64;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(thiserror::Error, Debug)]
pub enum CryptoError {
    #[error("bad password")]
    BadPassword,
    #[error("corrupted vault data")]
    Corrupted,
    #[error("crypto: {0}")]
    Other(String),
}

#[derive(Clone, ZeroizeOnDrop)]
pub struct DataKey(pub [u8; 32]);

impl DataKey {
    pub fn random() -> Self {
        let mut b = [0u8; 32];
        OsRng.fill_bytes(&mut b);
        Self(b)
    }
    fn key(&self) -> &Key<Aes256Gcm> {
        Key::<Aes256Gcm>::from_slice(&self.0)
    }
}

#[derive(Serialize, Deserialize)]
pub struct VaultMeta {
    pub salt: String,       // base64
    pub m_cost: u32,        // KiB
    pub t_cost: u32,
    pub p_cost: u32,
    pub wrapped_dek: String, // base64(nonce|ct)
}

const ARGON_M: u32 = 65536; // 64 MiB
const ARGON_T: u32 = 3;
const ARGON_P: u32 = 1;

pub fn create_vault(password: &str) -> Result<(VaultMeta, DataKey), CryptoError> {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);

    let kek = derive_kek(password, &salt, ARGON_M, ARGON_T, ARGON_P)?;
    let dek = DataKey::random();
    let wrapped = encrypt_with(&kek, &dek.0)?;

    Ok((
        VaultMeta {
            salt: B64.encode(salt),
            m_cost: ARGON_M,
            t_cost: ARGON_T,
            p_cost: ARGON_P,
            wrapped_dek: B64.encode(wrapped),
        },
        dek,
    ))
}

pub fn unlock_vault(meta: &VaultMeta, password: &str) -> Result<DataKey, CryptoError> {
    let salt = B64.decode(&meta.salt).map_err(|_| CryptoError::Corrupted)?;
    let kek = derive_kek(password, &salt, meta.m_cost, meta.t_cost, meta.p_cost)?;
    let wrapped = B64.decode(&meta.wrapped_dek).map_err(|_| CryptoError::Corrupted)?;
    let dek_bytes = decrypt_with(&kek, &wrapped).map_err(|_| CryptoError::BadPassword)?;
    if dek_bytes.len() != 32 {
        return Err(CryptoError::Corrupted);
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&dek_bytes);
    let mut dek = dek_bytes;
    dek.zeroize();
    Ok(DataKey(out))
}

fn derive_kek(password: &str, salt: &[u8], m: u32, t: u32, p: u32) -> Result<DataKey, CryptoError> {
    let params = Params::new(m, t, p, Some(32)).map_err(|e| CryptoError::Other(e.to_string()))?;
    let a = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    a.hash_password_into(password.as_bytes(), salt, &mut out)
        .map_err(|e| CryptoError::Other(e.to_string()))?;
    Ok(DataKey(out))
}

fn encrypt_with(key: &DataKey, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new(key.key());
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ct = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| CryptoError::Other(e.to_string()))?;
    let mut out = Vec::with_capacity(12 + ct.len());
    out.extend_from_slice(nonce.as_slice());
    out.extend_from_slice(&ct);
    Ok(out)
}

fn decrypt_with(key: &DataKey, blob: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if blob.len() < 12 {
        return Err(CryptoError::Corrupted);
    }
    let (n, ct) = blob.split_at(12);
    let cipher = Aes256Gcm::new(key.key());
    let nonce = Nonce::from_slice(n);
    cipher
        .decrypt(nonce, ct)
        .map_err(|_| CryptoError::BadPassword)
}

pub fn encrypt_field(key: &DataKey, plaintext: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let cipher = Aes256Gcm::new(key.key());
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ct = cipher.encrypt(&nonce, plaintext).expect("aes-gcm encrypt");
    (ct, nonce.to_vec())
}

pub fn decrypt_field(key: &DataKey, ct: &[u8], nonce: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new(key.key());
    let n = Nonce::from_slice(nonce);
    cipher.decrypt(n, ct).map_err(|_| CryptoError::BadPassword)
}
