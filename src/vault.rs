use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use argon2::Argon2;
use chacha20poly1305::{
    Key, XChaCha20Poly1305, XNonce,
    aead::{Aead, AeadCore, KeyInit},
};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

const MAGIC: &[u8; 8] = b"BLOBVLT1";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
const KEY_LEN: usize = 32;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Entry {
    pub name: String,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Vault {
    pub entries: Vec<Entry>,
}

/// A decrypted vault plus everything needed to re-encrypt it back to disk.
/// The derived key is zeroized when this is dropped (e.g. on lock).
pub struct UnlockedVault {
    pub vault: Vault,
    key: Zeroizing<[u8; KEY_LEN]>,
    salt: [u8; SALT_LEN],
    path: PathBuf,
}

pub fn vault_path() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "MeanIt", "BlobVault")
        .ok_or_else(|| anyhow!("could not determine a data directory for this user"))?;
    Ok(dirs.data_dir().join("vault.blob"))
}

pub fn exists() -> bool {
    vault_path().map(|p| p.exists()).unwrap_or(false)
}

fn derive_key(password: &str, salt: &[u8; SALT_LEN]) -> Result<Zeroizing<[u8; KEY_LEN]>> {
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, key.as_mut())
        .map_err(|e| anyhow!("key derivation failed: {e}"))?;
    Ok(key)
}

pub fn create(password: &str) -> Result<UnlockedVault> {
    let path = vault_path()?;
    if path.exists() {
        bail!("a vault already exists at {}", path.display());
    }
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let key = derive_key(password, &salt)?;
    let unlocked = UnlockedVault {
        vault: Vault::default(),
        key,
        salt,
        path,
    };
    unlocked.save()?;
    Ok(unlocked)
}

pub fn unlock(password: &str) -> Result<UnlockedVault> {
    let path = vault_path()?;
    let data = std::fs::read(&path)
        .with_context(|| format!("could not read vault file {}", path.display()))?;

    let min_len = MAGIC.len() + SALT_LEN + NONCE_LEN;
    if data.len() < min_len || &data[..MAGIC.len()] != MAGIC {
        bail!("vault file is corrupted or not a BlobVault file");
    }
    let salt: [u8; SALT_LEN] = data[MAGIC.len()..MAGIC.len() + SALT_LEN].try_into().unwrap();
    let nonce_start = MAGIC.len() + SALT_LEN;
    let nonce = XNonce::from_slice(&data[nonce_start..nonce_start + NONCE_LEN]);
    let ciphertext = &data[min_len..];

    let key = derive_key(password, &salt)?;
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&*key));
    let mut plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow!("wrong password"))?;

    let vault: Vault = serde_json::from_slice(&plaintext).context("vault contents are corrupted")?;
    plaintext.zeroize();

    Ok(UnlockedVault {
        vault,
        key,
        salt,
        path,
    })
}

impl UnlockedVault {
    pub fn save(&self) -> Result<()> {
        let mut plaintext = serde_json::to_vec(&self.vault)?;
        let cipher = XChaCha20Poly1305::new(Key::from_slice(&*self.key));
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_slice())
            .map_err(|e| anyhow!("encryption failed: {e}"))?;
        plaintext.zeroize();

        let mut out = Vec::with_capacity(MAGIC.len() + SALT_LEN + NONCE_LEN + ciphertext.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&self.salt);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("could not create {}", parent.display()))?;
        }
        // Write to a temp file first so a crash mid-write can't destroy the vault.
        let tmp = self.path.with_extension("blob.tmp");
        std::fs::write(&tmp, &out)
            .with_context(|| format!("could not write {}", tmp.display()))?;
        std::fs::rename(&tmp, &self.path)
            .with_context(|| format!("could not replace {}", self.path.display()))?;
        Ok(())
    }
}

/// Permanently delete the vault file. There is no backup and no recovery.
pub fn reset() -> Result<()> {
    let path = vault_path()?;
    std::fs::remove_file(&path)
        .with_context(|| format!("could not delete {}", path.display()))?;
    Ok(())
}

pub fn generate_password(len: usize) -> String {
    const CHARSET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+";
    let n = CHARSET.len() as u32;
    // Rejection sampling to avoid modulo bias.
    let bound = u32::MAX - (u32::MAX % n);
    (0..len)
        .map(|_| loop {
            let v = OsRng.next_u32();
            if v < bound {
                break CHARSET[(v % n) as usize] as char;
            }
        })
        .collect()
}
