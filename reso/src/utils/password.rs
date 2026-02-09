use anyhow::anyhow;
use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

/// Hashes a password using Argon2.
pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);

    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!("Failed to hash password: {}", e))?;

    Ok(hash.to_string())
}

/// Verifies a password against a hash.
pub fn verify_password(password: &str, hash: &str) -> anyhow::Result<()> {
    let hash = match PasswordHash::new(hash) {
        Ok(hash) => hash,
        Err(err) => {
            return Err(anyhow!("failed to parse password hash: {}", err));
        }
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .map_err(|e| anyhow!("failed to verify password: {}", e))?;

    Ok(())
}

pub fn generate_password(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*-_=+?";
    let mut out = String::with_capacity(length);

    // rejection sampling to avoid modulo bias.
    let charset_len = CHARSET.len();
    let max_acceptable = 256 - (256 % charset_len);

    while out.len() < length {
        let mut byte = [0u8; 1];
        OsRng.fill_bytes(&mut byte);
        let v = byte[0] as usize;

        if v < max_acceptable {
            out.push(CHARSET[v % charset_len] as char);
        }
    }

    out
}

#[cfg(test)]
#[path = "password_tests.rs"]
mod password_tests;