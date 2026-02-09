use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, OsRng, rand_core::RngCore},
};
use anyhow::anyhow;
use axum_extra::extract::cookie::{Cookie, SameSite};
use base64::{
    Engine, alphabet,
    engine::{self, general_purpose},
};
use uuid::Uuid;

use crate::{database::models::user_session::UserSession, utils::uuid::EntityId};

/// Identifier for the session cookie.
pub const SESSION_COOKIE_KEY: &str = "RESO_SESSION";

#[cfg(debug_assertions)]
const SAME_SITE: SameSite = SameSite::Lax;

#[cfg(not(debug_assertions))]
const SAME_SITE: SameSite = SameSite::Strict;

pub fn build_session_cookie<'a>(session: String) -> Cookie<'a> {
    Cookie::build((SESSION_COOKIE_KEY, session))
        .http_only(true)
        .path("/")
        .secure(false)
        .same_site(SAME_SITE)
        .build()
}
const BASE64_ENGINE: engine::GeneralPurpose = engine::GeneralPurpose::new(&alphabet::URL_SAFE, general_purpose::NO_PAD);

pub fn encrypt_session_id(cipher: &Aes256Gcm, id: EntityId<UserSession>) -> anyhow::Result<String> {
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let bytes = id.id().as_bytes();
    let ciphertext = cipher.encrypt(nonce, bytes.as_slice()).map_err(|e| anyhow!(e))?;

    let mut encrypted_session_id = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
    encrypted_session_id.extend_from_slice(&nonce_bytes);
    encrypted_session_id.extend_from_slice(&ciphertext);

    Ok(BASE64_ENGINE.encode(&encrypted_session_id))
}

pub fn decrypt_session_cookie(cipher: &Aes256Gcm, encoded: &str) -> anyhow::Result<EntityId<UserSession>> {
    let data = BASE64_ENGINE.decode(encoded).map_err(|e| anyhow::anyhow!(e))?;

    // must at least contain a 12-byte nonce + 16-byte GCM tag
    anyhow::ensure!(data.len() >= 12 + 16, "session cookie too short");

    let (nonce_bytes, ciphertext) = data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| anyhow::anyhow!(e))?;

    anyhow::ensure!(plaintext.len() == 16, "invalid session id length");

    let uuid = Uuid::from_slice(&plaintext).map_err(|e| anyhow::anyhow!(e))?;

    Ok(EntityId::from(uuid))
}

#[cfg(test)]
#[path = "cookie_tests.rs"]
mod cookie_tests;