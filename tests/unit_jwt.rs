use chalkbyte::config::jwt::JwtConfig;
use chalkbyte::modules::users::model::UserRole;
use chalkbyte::utils::jwt::{create_access_token, verify_token};
use uuid::Uuid;

fn get_test_jwt_config() -> JwtConfig {
    JwtConfig {
        secret: "test_secret_key_for_testing_purposes".to_string(),
        access_token_expiry: 3600,
        refresh_token_expiry: 604800,
    }
}

#[test]
fn test_create_access_token_success() {
    let jwt_config = get_test_jwt_config();
    let user_id = Uuid::new_v4();
    let email = "test@example.com";
    let role = UserRole::Student;

    let result = create_access_token(user_id, email, &role, &jwt_config);

    assert!(result.is_ok());
    let token = result.unwrap();
    assert!(!token.is_empty());
}

#[test]
fn test_create_access_token_all_roles() {
    let jwt_config = get_test_jwt_config();
    let user_id = Uuid::new_v4();
    let email = "test@example.com";

    let roles = vec![
        UserRole::SystemAdmin,
        UserRole::Admin,
        UserRole::Teacher,
        UserRole::Student,
    ];

    for role in roles {
        let result = create_access_token(user_id, email, &role, &jwt_config);
        assert!(result.is_ok());
    }
}

#[test]
fn test_verify_token_success() {
    let jwt_config = get_test_jwt_config();
    let user_id = Uuid::new_v4();
    let email = "test@example.com";
    let role = UserRole::Student;

    let token = create_access_token(user_id, email, &role, &jwt_config).unwrap();
    let result = verify_token(&token, &jwt_config);

    assert!(result.is_ok());
    let claims = result.unwrap();
    assert_eq!(claims.email, email);
    assert_eq!(claims.sub, user_id.to_string());
    assert_eq!(claims.role, "student");
}

#[test]
fn test_verify_token_invalid() {
    let jwt_config = get_test_jwt_config();
    let invalid_token = "invalid.token.here";

    let result = verify_token(invalid_token, &jwt_config);

    assert!(result.is_err());
}

#[test]
fn test_verify_token_wrong_secret() {
    let jwt_config = get_test_jwt_config();
    let user_id = Uuid::new_v4();
    let email = "test@example.com";
    let role = UserRole::Student;

    let token = create_access_token(user_id, email, &role, &jwt_config).unwrap();

    let wrong_jwt_config = JwtConfig {
        secret: "different_secret_key".to_string(),
        access_token_expiry: 3600,
        refresh_token_expiry: 604800,
    };

    let result = verify_token(&token, &wrong_jwt_config);

    assert!(result.is_err());
}

#[test]
fn test_verify_token_empty() {
    let jwt_config = get_test_jwt_config();
    let empty_token = "";

    let result = verify_token(empty_token, &jwt_config);

    assert!(result.is_err());
}

#[test]
fn test_token_contains_correct_role_system_admin() {
    let jwt_config = get_test_jwt_config();
    let user_id = Uuid::new_v4();
    let email = "admin@example.com";
    let role = UserRole::SystemAdmin;

    let token = create_access_token(user_id, email, &role, &jwt_config).unwrap();
    let claims = verify_token(&token, &jwt_config).unwrap();

    assert_eq!(claims.role, "system_admin");
}

#[test]
fn test_token_contains_correct_role_admin() {
    let jwt_config = get_test_jwt_config();
    let user_id = Uuid::new_v4();
    let email = "admin@example.com";
    let role = UserRole::Admin;

    let token = create_access_token(user_id, email, &role, &jwt_config).unwrap();
    let claims = verify_token(&token, &jwt_config).unwrap();

    assert_eq!(claims.role, "admin");
}

#[test]
fn test_token_contains_correct_role_teacher() {
    let jwt_config = get_test_jwt_config();
    let user_id = Uuid::new_v4();
    let email = "teacher@example.com";
    let role = UserRole::Teacher;

    let token = create_access_token(user_id, email, &role, &jwt_config).unwrap();
    let claims = verify_token(&token, &jwt_config).unwrap();

    assert_eq!(claims.role, "teacher");
}

#[test]
fn test_token_contains_correct_role_student() {
    let jwt_config = get_test_jwt_config();
    let user_id = Uuid::new_v4();
    let email = "student@example.com";
    let role = UserRole::Student;

    let token = create_access_token(user_id, email, &role, &jwt_config).unwrap();
    let claims = verify_token(&token, &jwt_config).unwrap();

    assert_eq!(claims.role, "student");
}

#[test]
fn test_token_expiry_is_set() {
    let jwt_config = get_test_jwt_config();
    let user_id = Uuid::new_v4();
    let email = "test@example.com";
    let role = UserRole::Student;

    let token = create_access_token(user_id, email, &role, &jwt_config).unwrap();
    let claims = verify_token(&token, &jwt_config).unwrap();

    assert!(claims.exp > claims.iat);
    assert_eq!(
        claims.exp - claims.iat,
        jwt_config.access_token_expiry as usize
    );
}

#[test]
fn test_token_with_special_characters_in_email() {
    let jwt_config = get_test_jwt_config();
    let user_id = Uuid::new_v4();
    let email = "test+special@example.co.uk";
    let role = UserRole::Student;

    let token = create_access_token(user_id, email, &role, &jwt_config).unwrap();
    let claims = verify_token(&token, &jwt_config).unwrap();

    assert_eq!(claims.email, email);
}

#[test]
fn test_verify_token_malformed() {
    let jwt_config = get_test_jwt_config();
    let malformed_tokens = vec![
        "not.enough.parts",
        "too.many.parts.here.extra",
        "!!!.invalid.chars",
        "header.payload.",
        ".payload.signature",
    ];

    for token in malformed_tokens {
        let result = verify_token(token, &jwt_config);
        assert!(result.is_err());
    }
}

#[test]
fn test_create_token_different_users_different_tokens() {
    let jwt_config = get_test_jwt_config();
    let user_id1 = Uuid::new_v4();
    let user_id2 = Uuid::new_v4();
    let email1 = "user1@example.com";
    let email2 = "user2@example.com";
    let role = UserRole::Student;

    let token1 = create_access_token(user_id1, email1, &role, &jwt_config).unwrap();
    let token2 = create_access_token(user_id2, email2, &role, &jwt_config).unwrap();

    assert_ne!(token1, token2);

    let claims1 = verify_token(&token1, &jwt_config).unwrap();
    let claims2 = verify_token(&token2, &jwt_config).unwrap();

    assert_eq!(claims1.sub, user_id1.to_string());
    assert_eq!(claims2.sub, user_id2.to_string());
    assert_eq!(claims1.email, email1);
    assert_eq!(claims2.email, email2);
}
