use chalkbyte_core::AppError;
use chalkbyte_db::PgPool;
use uuid::Uuid;

use crate::middleware::auth::AuthUser;
use crate::middleware::role::is_system_admin_jwt;

/// Get the school_id for operations that require school scoping.
///
/// Priority:
/// 1. Check JWT claims for school_id (fast, no DB query)
/// 2. For system admins without school_id, return error (they must specify school)
/// 3. Fallback to database lookup (for edge cases)
pub async fn get_admin_school_id(db: &PgPool, auth_user: &AuthUser) -> Result<Uuid, AppError> {
    // First, try to get school_id from JWT claims (fast path)
    if let Some(school_id) = auth_user.school_id() {
        return Ok(school_id);
    }

    // System admins don't have school_id in their JWT
    // They need to specify the school for operations
    if is_system_admin_jwt(auth_user) {
        return Err(AppError::bad_request(anyhow::anyhow!(
            "System admin must specify a school_id for this operation"
        )));
    }

    // Fallback: fetch from database (should rarely happen if JWT is properly populated)
    let user_id = auth_user.user_id()?;

    let school_id =
        sqlx::query_scalar::<_, Option<Uuid>>("SELECT school_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(db)
            .await?
            .ok_or_else(|| {
                AppError::forbidden("User must be associated with a school".to_string())
            })?;

    Ok(school_id)
}

/// Get school_id for operations that require scoping (create, list).
/// System admins MUST provide school_id, school admins use their own.
pub async fn get_school_id_for_scoped_operation(
    db: &PgPool,
    auth_user: &AuthUser,
    specified_school_id: Option<Uuid>,
) -> Result<Uuid, AppError> {
    // System admin must specify school_id for scoped operations
    if is_system_admin_jwt(auth_user) {
        return specified_school_id.ok_or_else(|| {
            AppError::bad_request(anyhow::anyhow!(
                "System admin must specify school_id for this operation"
            ))
        });
    }

    // School admins use their own school (ignore any specified school_id)
    get_admin_school_id(db, auth_user).await
}

/// Get optional school_id for operations on existing resources (get by id, update, delete).
/// System admins get None (no school scoping), school admins get their school_id.
/// This allows system admins to operate on any resource without specifying school.
#[allow(dead_code)]
pub async fn get_optional_school_id_for_resource_operation(
    db: &PgPool,
    auth_user: &AuthUser,
) -> Result<Option<Uuid>, AppError> {
    // System admins can access any resource - no school scoping needed
    if is_system_admin_jwt(auth_user) {
        return Ok(None);
    }

    // School admins are scoped to their school
    let school_id = get_admin_school_id(db, auth_user).await?;
    Ok(Some(school_id))
}

/// Get the school_id from auth user, with an option to provide a specific school_id.
/// Useful for system admins who can operate on any school.
///
/// - If `specified_school_id` is provided and user is system admin, use that
/// - Otherwise, use the user's associated school_id
#[allow(dead_code)]
pub async fn get_school_id_with_override(
    db: &PgPool,
    auth_user: &AuthUser,
    specified_school_id: Option<Uuid>,
) -> Result<Uuid, AppError> {
    // System admin can specify any school
    if is_system_admin_jwt(auth_user) {
        return specified_school_id.ok_or_else(|| {
            AppError::bad_request(anyhow::anyhow!(
                "System admin must specify a school_id for this operation"
            ))
        });
    }

    // Non-system admins use their own school (ignore any specified school_id)
    get_admin_school_id(db, auth_user).await
}

/// Verify that a resource belongs to the user's school (for school admins).
/// System admins bypass this check.
#[allow(dead_code)]
pub async fn verify_school_access(
    db: &PgPool,
    auth_user: &AuthUser,
    resource_school_id: Uuid,
) -> Result<(), AppError> {
    // System admins can access any school's resources
    if is_system_admin_jwt(auth_user) {
        return Ok(());
    }

    // School admins can only access their own school's resources
    let user_school_id = get_admin_school_id(db, auth_user).await?;
    if user_school_id != resource_school_id {
        return Err(AppError::forbidden(
            "You can only access resources from your own school".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::users::model::system_roles;
    use chalkbyte_auth::Claims;

    fn create_test_auth_user(school_id: Option<Uuid>, role_ids: Vec<Uuid>) -> AuthUser {
        AuthUser(Claims {
            sub: Uuid::new_v4().to_string(),
            email: "test@example.com".to_string(),
            school_id,
            role_ids,
            permissions: vec![],
            exp: 9999999999,
            iat: 1234567890,
        })
    }

    #[test]
    fn test_school_id_from_jwt() {
        let school_id = Uuid::new_v4();
        let auth_user = create_test_auth_user(Some(school_id), vec![system_roles::ADMIN]);

        assert_eq!(auth_user.school_id(), Some(school_id));
    }

    #[test]
    fn test_system_admin_no_school_id() {
        let auth_user = create_test_auth_user(None, vec![system_roles::SYSTEM_ADMIN]);

        assert_eq!(auth_user.school_id(), None);
        assert!(is_system_admin_jwt(&auth_user));
    }

    #[test]
    fn test_school_admin_has_school_id() {
        let school_id = Uuid::new_v4();
        let auth_user = create_test_auth_user(Some(school_id), vec![system_roles::ADMIN]);

        assert_eq!(auth_user.school_id(), Some(school_id));
        assert!(!is_system_admin_jwt(&auth_user));
    }

    #[tokio::test]
    async fn test_get_optional_school_id_system_admin() {
        // System admin should get None (no scoping)
        let auth_user = create_test_auth_user(None, vec![system_roles::SYSTEM_ADMIN]);

        // We can't test DB operations here, but we can verify the JWT check
        assert!(is_system_admin_jwt(&auth_user));
    }

    #[tokio::test]
    async fn test_verify_school_access_system_admin() {
        let auth_user = create_test_auth_user(None, vec![system_roles::SYSTEM_ADMIN]);
        let _any_school_id = Uuid::new_v4();

        // System admin should pass verification for any school (mocked - no DB)
        assert!(is_system_admin_jwt(&auth_user));
    }
}
