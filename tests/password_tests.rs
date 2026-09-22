use mairie360_api_lib::password::{
    error::PasswordError, hash_password, is_hashed, verify_password,
};

/**
 * This module contains tests for the password module.
 * It tests argon2id hashing, verification, and detection of already-hashed values.
 * These are pure functions and do not need the shared Docker test database.
 */
#[cfg(test)]
mod password_tests {
    use super::*;

    #[test]
    fn test_hash_password_produces_argon2id_phc_string() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(
            hash.starts_with("$argon2id$"),
            "Hash does not use the argon2id PHC format: {hash}"
        );
    }

    #[test]
    fn test_hash_password_uses_a_unique_salt_each_time() {
        let first = hash_password("same password").unwrap();
        let second = hash_password("same password").unwrap();
        assert_ne!(
            first, second,
            "Hashing the same password twice should yield different salts/hashes"
        );
    }

    #[test]
    fn test_verify_password_succeeds_for_the_matching_password() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert_eq!(
            verify_password("correct horse battery staple", &hash),
            Ok(true),
            "Verification should succeed for the password that was hashed"
        );
    }

    #[test]
    fn test_verify_password_fails_for_a_wrong_password() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert_eq!(
            verify_password("wrong password", &hash),
            Ok(false),
            "Verification should fail for a password that was not hashed"
        );
    }

    #[test]
    fn test_verify_password_rejects_a_non_argon2id_value() {
        let result = verify_password("any password", "not a phc hash");
        assert_eq!(
            result,
            Err(PasswordError::InvalidHash),
            "A value that isn't a valid PHC string must be rejected, never compared in clear"
        );
    }

    #[test]
    fn test_is_hashed_detects_argon2id_values() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(
            is_hashed(&hash),
            "A freshly produced argon2id hash should be detected as hashed"
        );
    }

    #[test]
    fn test_is_hashed_rejects_plaintext_values() {
        assert!(
            !is_hashed("plaintext-password"),
            "A plaintext value must never be reported as already hashed"
        );
        assert!(
            !is_hashed(""),
            "An empty value must never be reported as already hashed"
        );
    }
}
