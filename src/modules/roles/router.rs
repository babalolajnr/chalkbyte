use axum::{
    Router,
    routing::{delete, get, post, put},
};

use crate::state::AppState;

use super::controller::{
    assign_permissions, assign_role_to_user, create_role, delete_role, get_permission_by_id,
    get_permissions, get_role_by_id, get_roles, get_user_permissions, get_user_roles,
    remove_permission, remove_role_from_user, update_role,
};

pub fn init_roles_router() -> Router<AppState> {
    Router::new()
        // Permission endpoints
        .route("/permissions", get(get_permissions))
        .route("/permissions/{id}", get(get_permission_by_id))
        // Custom role endpoints
        .route("/", post(create_role).get(get_roles))
        .route("/{id}", get(get_role_by_id).delete(delete_role))
        .route("/{id}", put(update_role))
        // Role permission management
        .route("/{id}/permissions", post(assign_permissions))
        .route(
            "/{role_id}/permissions/{permission_id}",
            delete(remove_permission),
        )
}

pub fn init_user_roles_router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_user_roles).post(assign_role_to_user))
        .route("/{role_id}", delete(remove_role_from_user))
}

pub fn init_user_permissions_router() -> Router<AppState> {
    Router::new().route("/", get(get_user_permissions))
}
