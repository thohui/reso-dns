#[cfg(test)]
mod tests {
    use super::super::cookie::{encrypt_session_id, decrypt_session_cookie, build_session_cookie, SESSION_COOKIE_KEY};
    use crate::database::models::user_session::UserSession;
    use crate::utils::uuid::EntityId;
    use aes_gcm::{Aes256Gcm, KeyInit, aead::generic_array::GenericArray};
    use uuid::Uuid;

    fn create_test_cipher() -> Aes256Gcm {
        let key = [0u8; 32];
        Aes256Gcm::new(&GenericArray::clone_from_slice(&key))
    }

    #[test]
    fn test_encrypt_and_decrypt_session_id() {
        let cipher = create_test_cipher();
        let session_id = EntityId::<UserSession>::from(Uuid::now_v7());

        let encrypted = encrypt_session_id(&cipher, session_id.clone()).expect("encryption failed");
        assert!(!encrypted.is_empty());

        let decrypted = decrypt_session_cookie(&cipher, &encrypted).expect("decryption failed");
        assert_eq!(session_id.id(), decrypted.id());
    }

    #[test]
    fn test_encrypt_produces_different_results() {
        let cipher = create_test_cipher();
        let session_id = EntityId::<UserSession>::from(Uuid::now_v7());

        let encrypted1 = encrypt_session_id(&cipher, session_id.clone()).expect("encryption failed");
        let encrypted2 = encrypt_session_id(&cipher, session_id.clone()).expect("encryption failed");

        assert_ne!(encrypted1, encrypted2, "Nonce should make each encryption unique");

        let decrypted1 = decrypt_session_cookie(&cipher, &encrypted1).expect("decryption failed");
        let decrypted2 = decrypt_session_cookie(&cipher, &encrypted2).expect("decryption failed");
        assert_eq!(decrypted1.id(), decrypted2.id());
    }

    #[test]
    fn test_decrypt_invalid_base64() {
        let cipher = create_test_cipher();
        let result = decrypt_session_cookie(&cipher, "not-valid-base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_too_short() {
        let cipher = create_test_cipher();
        let short_data = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&[0u8; 10]);
        let result = decrypt_session_cookie(&cipher, &short_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_wrong_key() {
        let cipher1 = create_test_cipher();
        let session_id = EntityId::<UserSession>::from(Uuid::now_v7());

        let encrypted = encrypt_session_id(&cipher1, session_id).expect("encryption failed");

        let wrong_key = [1u8; 32];
        let cipher2 = Aes256Gcm::new(&GenericArray::clone_from_slice(&wrong_key));
        let result = decrypt_session_cookie(&cipher2, &encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_tampered_ciphertext() {
        let cipher = create_test_cipher();
        let session_id = EntityId::<UserSession>::from(Uuid::now_v7());

        let mut encrypted = encrypt_session_id(&cipher, session_id).expect("encryption failed");

        encrypted.push('X');

        let result = decrypt_session_cookie(&cipher, &encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_session_cookie() {
        let session = "test_session_value".to_string();
        let cookie = build_session_cookie(session.clone());

        assert_eq!(cookie.name(), SESSION_COOKIE_KEY);
        assert_eq!(cookie.value(), session);
        assert!(cookie.http_only().unwrap());
        assert_eq!(cookie.path(), Some("/"));
    }

    #[test]
    fn test_session_cookie_key_constant() {
        assert_eq!(SESSION_COOKIE_KEY, "RESO_SESSION");
    }

    #[test]
    fn test_round_trip_multiple_sessions() {
        let cipher = create_test_cipher();
        let sessions: Vec<EntityId<UserSession>> = (0..10)
            .map(|_| EntityId::from(Uuid::now_v7()))
            .collect();

        for session_id in sessions {
            let encrypted = encrypt_session_id(&cipher, session_id.clone()).expect("encryption failed");
            let decrypted = decrypt_session_cookie(&cipher, &encrypted).expect("decryption failed");
            assert_eq!(session_id.id(), decrypted.id());
        }
    }
}