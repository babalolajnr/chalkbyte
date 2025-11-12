use chalkbyte::utils::password::{hash_password, verify_password};

#[test]
fn test_hash_password_success() {
    let password = "testpassword123";
    let result = hash_password(password);

    assert!(result.is_ok());
    let hash = result.unwrap();
    assert!(!hash.is_empty());
    assert_ne!(hash, password);
}

#[test]
fn test_hash_password_empty() {
    let password = "";
    let result = hash_password(password);

    assert!(result.is_ok());
}

#[test]
fn test_verify_password_correct() {
    let password = "correctpassword";
    let hash = hash_password(password).unwrap();

    let result = verify_password(password, &hash);

    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn test_verify_password_incorrect() {
    let password = "correctpassword";
    let wrong_password = "wrongpassword";
    let hash = hash_password(password).unwrap();

    let result = verify_password(wrong_password, &hash);

    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
fn test_verify_password_invalid_hash() {
    let password = "testpassword";
    let invalid_hash = "not_a_valid_bcrypt_hash";

    let result = verify_password(password, invalid_hash);

    assert!(result.is_err());
}

#[test]
fn test_hash_generates_unique_hashes() {
    let password = "samepassword";
    let hash1 = hash_password(password).unwrap();
    let hash2 = hash_password(password).unwrap();

    assert_ne!(hash1, hash2);
    assert!(verify_password(password, &hash1).unwrap());
    assert!(verify_password(password, &hash2).unwrap());
}

#[test]
fn test_hash_special_characters() {
    let password = "p@ssw0rd!#$%^&*()";
    let hash = hash_password(password).unwrap();

    let result = verify_password(password, &hash);

    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn test_hash_unicode_characters() {
    let password = "пароль密码🔒";
    let hash = hash_password(password).unwrap();

    let result = verify_password(password, &hash);

    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn test_hash_long_password() {
    let password = "a".repeat(100);
    let hash = hash_password(&password).unwrap();

    let result = verify_password(&password, &hash);

    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn test_verify_case_sensitive() {
    let password = "Password123";
    let hash = hash_password(password).unwrap();

    let result1 = verify_password("password123", &hash);
    let result2 = verify_password("PASSWORD123", &hash);

    assert!(result1.is_ok());
    assert!(!result1.unwrap());
    assert!(result2.is_ok());
    assert!(!result2.unwrap());
}
