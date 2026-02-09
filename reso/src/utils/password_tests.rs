#[cfg(test)]
mod tests {
    use super::super::password::{hash_password, verify_password, generate_password};

    #[test]
    fn test_hash_password_creates_hash() {
        let password = "test_password_123";
        let hash = hash_password(password).expect("hashing failed");

        assert!(!hash.is_empty());
        assert!(hash.starts_with("$argon2"));
    }

    #[test]
    fn test_hash_password_different_hashes() {
        let password = "test_password_123";
        let hash1 = hash_password(password).expect("hashing failed");
        let hash2 = hash_password(password).expect("hashing failed");

        assert_ne!(hash1, hash2, "Salt should make each hash unique");
    }

    #[test]
    fn test_verify_password_success() {
        let password = "correct_password";
        let hash = hash_password(password).expect("hashing failed");

        let result = verify_password(password, &hash);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_password_failure() {
        let password = "correct_password";
        let hash = hash_password(password).expect("hashing failed");

        let result = verify_password("wrong_password", &hash);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_password_empty() {
        let password = "";
        let hash = hash_password(password).expect("hashing failed");

        let result = verify_password("", &hash);
        assert!(result.is_ok());

        let result = verify_password("nonempty", &hash);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_password_invalid_hash() {
        let result = verify_password("password", "invalid_hash");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_password_length() {
        let length = 16;
        let password = generate_password(length);
        assert_eq!(password.len(), length);

        let length = 32;
        let password = generate_password(length);
        assert_eq!(password.len(), length);

        let length = 8;
        let password = generate_password(length);
        assert_eq!(password.len(), length);
    }

    #[test]
    fn test_generate_password_uniqueness() {
        let password1 = generate_password(16);
        let password2 = generate_password(16);

        assert_ne!(password1, password2, "Generated passwords should be unique");
    }

    #[test]
    fn test_generate_password_charset() {
        const CHARSET: &str = "ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*-_=+?";

        let password = generate_password(100);

        for ch in password.chars() {
            assert!(CHARSET.contains(ch), "Character {} not in allowed charset", ch);
        }
    }

    #[test]
    fn test_generate_password_no_ambiguous() {
        let password = generate_password(100);

        let ambiguous = ['0', 'O', 'o', 'I', 'l', '1'];
        for ch in password.chars() {
            assert!(!ambiguous.contains(&ch), "Password contains ambiguous character {}", ch);
        }
    }

    #[test]
    fn test_generate_password_zero_length() {
        let password = generate_password(0);
        assert_eq!(password.len(), 0);
    }

    #[test]
    fn test_generate_password_large_length() {
        let password = generate_password(1000);
        assert_eq!(password.len(), 1000);
    }

    #[test]
    fn test_hash_and_verify_special_chars() {
        let password = "p@ssw0rd!#$%^&*()";
        let hash = hash_password(password).expect("hashing failed");
        let result = verify_password(password, &hash);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hash_and_verify_unicode() {
        let password = "пароль密码🔐";
        let hash = hash_password(password).expect("hashing failed");
        let result = verify_password(password, &hash);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hash_and_verify_long_password() {
        let password = "a".repeat(1000);
        let hash = hash_password(&password).expect("hashing failed");
        let result = verify_password(&password, &hash);
        assert!(result.is_ok());
    }
}