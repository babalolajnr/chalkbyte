use chalkbyte::middleware::auth::AuthUser;
use chalkbyte::middleware::role::{
    check_any_role, check_role, check_role_hierarchy, role_hierarchy_level,
};
use chalkbyte::modules::auth::model::Claims;
use chalkbyte::modules::users::model::UserRole;

fn create_test_auth_user(role: &str) -> AuthUser {
    let claims = Claims {
        sub: "00000000-0000-0000-0000-000000000000".to_string(),
        email: "test@example.com".to_string(),
        role: role.to_string(),
        exp: 9999999999,
        iat: 1234567890,
    };
    AuthUser(claims)
}

#[test]
fn test_check_role_exact_match() {
    let auth_user = create_test_auth_user("system_admin");
    assert!(check_role(&auth_user, UserRole::SystemAdmin).is_ok());

    let auth_user = create_test_auth_user("admin");
    assert!(check_role(&auth_user, UserRole::Admin).is_ok());

    let auth_user = create_test_auth_user("teacher");
    assert!(check_role(&auth_user, UserRole::Teacher).is_ok());

    let auth_user = create_test_auth_user("student");
    assert!(check_role(&auth_user, UserRole::Student).is_ok());
}

#[test]
fn test_check_role_no_match() {
    let auth_user = create_test_auth_user("student");
    assert!(check_role(&auth_user, UserRole::Admin).is_err());

    let auth_user = create_test_auth_user("teacher");
    assert!(check_role(&auth_user, UserRole::SystemAdmin).is_err());

    let auth_user = create_test_auth_user("admin");
    assert!(check_role(&auth_user, UserRole::SystemAdmin).is_err());
}

#[test]
fn test_check_any_role_single_match() {
    let auth_user = create_test_auth_user("admin");
    let allowed = vec![UserRole::Admin];
    assert!(check_any_role(&auth_user, &allowed).is_ok());
}

#[test]
fn test_check_any_role_multiple_match() {
    let allowed = vec![UserRole::Admin, UserRole::Teacher, UserRole::Student];

    let auth_user = create_test_auth_user("admin");
    assert!(check_any_role(&auth_user, &allowed).is_ok());

    let auth_user = create_test_auth_user("teacher");
    assert!(check_any_role(&auth_user, &allowed).is_ok());

    let auth_user = create_test_auth_user("student");
    assert!(check_any_role(&auth_user, &allowed).is_ok());
}

#[test]
fn test_check_any_role_no_match() {
    let allowed = vec![UserRole::Admin, UserRole::Teacher];
    let auth_user = create_test_auth_user("student");
    assert!(check_any_role(&auth_user, &allowed).is_err());
}

#[test]
fn test_check_any_role_empty_list() {
    let allowed = vec![];
    let auth_user = create_test_auth_user("admin");
    assert!(check_any_role(&auth_user, &allowed).is_err());
}

#[test]
fn test_role_hierarchy_levels() {
    assert_eq!(role_hierarchy_level(&UserRole::SystemAdmin), 3);
    assert_eq!(role_hierarchy_level(&UserRole::Admin), 2);
    assert_eq!(role_hierarchy_level(&UserRole::Teacher), 1);
    assert_eq!(role_hierarchy_level(&UserRole::Student), 0);
}

#[test]
fn test_role_hierarchy_ordering() {
    assert!(role_hierarchy_level(&UserRole::SystemAdmin) > role_hierarchy_level(&UserRole::Admin));
    assert!(role_hierarchy_level(&UserRole::Admin) > role_hierarchy_level(&UserRole::Teacher));
    assert!(role_hierarchy_level(&UserRole::Teacher) > role_hierarchy_level(&UserRole::Student));
}

#[test]
fn test_check_role_hierarchy_system_admin_highest() {
    assert!(check_role_hierarchy(&UserRole::SystemAdmin, &UserRole::Admin).is_ok());
    assert!(check_role_hierarchy(&UserRole::SystemAdmin, &UserRole::Teacher).is_ok());
    assert!(check_role_hierarchy(&UserRole::SystemAdmin, &UserRole::Student).is_ok());
}

#[test]
fn test_check_role_hierarchy_admin_over_staff() {
    assert!(check_role_hierarchy(&UserRole::Admin, &UserRole::Teacher).is_ok());
    assert!(check_role_hierarchy(&UserRole::Admin, &UserRole::Student).is_ok());
}

#[test]
fn test_check_role_hierarchy_teacher_over_student() {
    assert!(check_role_hierarchy(&UserRole::Teacher, &UserRole::Student).is_ok());
}

#[test]
fn test_check_role_hierarchy_same_level() {
    assert!(check_role_hierarchy(&UserRole::SystemAdmin, &UserRole::SystemAdmin).is_ok());
    assert!(check_role_hierarchy(&UserRole::Admin, &UserRole::Admin).is_ok());
    assert!(check_role_hierarchy(&UserRole::Teacher, &UserRole::Teacher).is_ok());
    assert!(check_role_hierarchy(&UserRole::Student, &UserRole::Student).is_ok());
}

#[test]
fn test_check_role_hierarchy_lower_cannot_access_higher() {
    assert!(check_role_hierarchy(&UserRole::Student, &UserRole::Teacher).is_err());
    assert!(check_role_hierarchy(&UserRole::Student, &UserRole::Admin).is_err());
    assert!(check_role_hierarchy(&UserRole::Student, &UserRole::SystemAdmin).is_err());
    assert!(check_role_hierarchy(&UserRole::Teacher, &UserRole::Admin).is_err());
    assert!(check_role_hierarchy(&UserRole::Teacher, &UserRole::SystemAdmin).is_err());
    assert!(check_role_hierarchy(&UserRole::Admin, &UserRole::SystemAdmin).is_err());
}

#[test]
fn test_user_role_equality() {
    assert_eq!(UserRole::SystemAdmin, UserRole::SystemAdmin);
    assert_eq!(UserRole::Admin, UserRole::Admin);
    assert_ne!(UserRole::SystemAdmin, UserRole::Admin);
    assert_ne!(UserRole::Teacher, UserRole::Student);
}
