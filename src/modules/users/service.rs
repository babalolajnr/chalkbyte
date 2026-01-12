use crate::{
    metrics,
    modules::users::model::{
        BranchInfo, ChangePasswordDto, CreateUserDto, LevelInfo, PaginatedUsersResponse, RoleInfo,
        School, SchoolInfo, UpdateProfileDto, User, UserFilterParams, UserWithRelations,
        UserWithSchool, system_roles,
    },
    utils::{
        errors::AppError,
        pagination::PaginationMeta,
        password::{hash_password, verify_password},
    },
};
use anyhow::Context;
use chalkbyte_models::ids::{BranchId, LevelId, RoleId, SchoolId, UserId};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

pub struct UserService;

impl UserService {
    #[instrument(skip(db, dto), fields(user.email = %dto.email))]
    pub async fn create_user(db: &PgPool, dto: CreateUserDto) -> Result<User, AppError> {
        debug!(email = %dto.email, "Creating new user");

        let password_hash = hash_password(&dto.password)?;

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (first_name, last_name, email, password, school_id)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, first_name, last_name, email, school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at
            "#,
        )
        .bind(&dto.first_name)
        .bind(&dto.last_name)
        .bind(&dto.email)
        .bind(&password_hash)
        .bind(dto.school_id)
        .fetch_one(db)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e
                && db_err.is_unique_violation()
            {
                warn!(email = %dto.email, "Attempted to create user with existing email");
                return AppError::unprocessable(anyhow::anyhow!(
                    "User with this email already exists"
                ));
            }
            error!(error = %e, "Failed to create user");
            AppError::database(anyhow::Error::new(e).context("Failed to insert user"))
        })?;

        // Assign roles if provided
        if !dto.role_ids.is_empty() {
            for role_id in &dto.role_ids {
                sqlx::query(
                    "INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                )
                .bind(user.id)
                .bind(role_id)
                .execute(db)
                .await
                .map_err(|e| {
                    error!(error = %e, "Failed to assign role to user");
                    AppError::database(anyhow::Error::new(e).context("Failed to assign role"))
                })?;
            }
        }

        // Track metrics based on first role assigned
        let role_name = if let Some(first_role_id) = dto.role_ids.first() {
            system_roles::get_name(first_role_id)
                .unwrap_or("custom")
                .to_lowercase()
        } else {
            "none".to_string()
        };
        metrics::track_user_created(&role_name);

        info!(user.id = %user.id, user.email = %user.email, "User created successfully");
        Ok(user)
    }

    #[instrument(skip(db), fields(school_id = ?school_id_filter))]
    pub async fn get_users_paginated(
        db: &PgPool,
        filters: UserFilterParams,
        school_id_filter: Option<SchoolId>,
    ) -> Result<PaginatedUsersResponse, AppError> {
        let limit = filters.pagination.limit();
        let offset = filters.pagination.offset();
        let page = filters.pagination.page();

        debug!(
            limit = %limit,
            offset = %offset,
            filters = ?filters,
            "Fetching paginated users"
        );

        let mut conditions = vec![];
        let mut param_count = 0;

        // Main query with LEFT JOINs for school, level, branch
        let mut query = String::from(
            r#"SELECT DISTINCT
                u.id, u.first_name, u.last_name, u.email, u.date_of_birth, u.grade_level, u.created_at, u.updated_at,
                s.id as school_id, s.name as school_name, s.address as school_address,
                l.id as level_id, l.name as level_name, l.description as level_description,
                b.id as branch_id, b.name as branch_name, b.description as branch_description
            FROM users u
            LEFT JOIN schools s ON u.school_id = s.id
            LEFT JOIN levels l ON u.level_id = l.id
            LEFT JOIN branches b ON u.branch_id = b.id"#,
        );

        // Join with user_roles if filtering by role
        if filters.role_id.is_some() {
            query.push_str(" INNER JOIN user_roles ur ON u.id = ur.user_id");
        }

        query.push_str(" WHERE 1=1");

        if let Some(ref first_name) = filters.first_name {
            param_count += 1;
            conditions.push((
                param_count,
                format!("u.first_name ILIKE ${}", param_count),
                format!("%{}%", first_name),
            ));
        }

        if let Some(ref last_name) = filters.last_name {
            param_count += 1;
            conditions.push((
                param_count,
                format!("u.last_name ILIKE ${}", param_count),
                format!("%{}%", last_name),
            ));
        }

        if let Some(ref email) = filters.email {
            param_count += 1;
            conditions.push((
                param_count,
                format!("u.email ILIKE ${}", param_count),
                format!("%{}%", email),
            ));
        }

        if let Some(role_id) = filters.role_id {
            param_count += 1;
            conditions.push((
                param_count,
                format!("ur.role_id = ${}", param_count),
                role_id.to_string(),
            ));
        }

        if let Some(sid) = filters.school_id {
            param_count += 1;
            conditions.push((
                param_count,
                format!("u.school_id = ${}", param_count),
                sid.to_string(),
            ));
        }

        if let Some(sid) = school_id_filter {
            param_count += 1;
            conditions.push((
                param_count,
                format!("u.school_id = ${}", param_count),
                sid.to_string(),
            ));
        }

        for (_, condition, _) in &conditions {
            query.push_str(&format!(" AND {}", condition));
        }

        // Build count query
        let count_query = if filters.role_id.is_some() {
            format!(
                "SELECT COUNT(DISTINCT u.id) FROM users u INNER JOIN user_roles ur ON u.id = ur.user_id WHERE 1=1{}",
                conditions
                    .iter()
                    .map(|(_, c, _)| format!(" AND {}", c))
                    .collect::<String>()
            )
        } else {
            format!(
                "SELECT COUNT(*) FROM users u WHERE 1=1{}",
                conditions
                    .iter()
                    .map(|(_, c, _)| format!(" AND {}", c))
                    .collect::<String>()
            )
        };

        query.push_str(&format!(
            " ORDER BY u.created_at DESC LIMIT ${} OFFSET ${}",
            param_count + 1,
            param_count + 2
        ));

        let mut query_builder = sqlx::query(&query);
        let mut count_query_builder = sqlx::query_scalar::<_, i64>(&count_query);

        for (_, _, value) in &conditions {
            if value.starts_with('%') {
                query_builder = query_builder.bind(value);
                count_query_builder = count_query_builder.bind(value);
            } else if let Ok(uuid_val) = Uuid::parse_str(value) {
                query_builder = query_builder.bind(uuid_val);
                count_query_builder = count_query_builder.bind(uuid_val);
            } else {
                query_builder = query_builder.bind(value);
                count_query_builder = count_query_builder.bind(value);
            }
        }

        query_builder = query_builder.bind(limit).bind(offset);

        let rows = query_builder
            .fetch_all(db)
            .await
            .context("Failed to fetch paginated users")
            .map_err(|e| {
                error!(error = %e, "Database error fetching users");
                AppError::database(e)
            })?;

        // Collect user IDs for batch role fetch
        let user_ids: Vec<UserId> = rows
            .iter()
            .filter_map(|row| row.try_get::<UserId, _>("id").ok())
            .collect();

        // Fetch roles for all users in one query
        let roles_map: HashMap<UserId, Vec<RoleInfo>> = if !user_ids.is_empty() {
            let uuid_user_ids: Vec<Uuid> = user_ids.iter().map(|id| id.into_inner()).collect();
            let roles_rows = sqlx::query(
                r#"SELECT ur.user_id, r.id, r.name, r.description, r.is_system_role
                   FROM user_roles ur
                   INNER JOIN roles r ON ur.role_id = r.id
                   WHERE ur.user_id = ANY($1)"#,
            )
            .bind(&uuid_user_ids)
            .fetch_all(db)
            .await
            .context("Failed to fetch user roles")
            .map_err(|e| {
                error!(error = %e, "Database error fetching roles");
                AppError::database(e)
            })?;

            let mut map: HashMap<UserId, Vec<RoleInfo>> = HashMap::new();
            for row in roles_rows {
                let user_id: UserId = row
                    .try_get::<Uuid, _>("user_id")
                    .map(UserId::from)
                    .unwrap_or_default();
                let role = RoleInfo {
                    id: row
                        .try_get::<Uuid, _>("id")
                        .map(RoleId::from)
                        .unwrap_or_default(),
                    name: row.try_get("name").unwrap_or_default(),
                    description: row.try_get("description").ok(),
                    is_system_role: row.try_get("is_system_role").unwrap_or(false),
                };
                map.entry(user_id).or_default().push(role);
            }
            map
        } else {
            HashMap::new()
        };

        let users: Result<Vec<UserWithRelations>, AppError> = rows
            .iter()
            .map(|row| {
                let user_id: UserId =
                    row.try_get::<Uuid, _>("id")
                        .map(UserId::from)
                        .map_err(|e| {
                            AppError::database(anyhow::Error::new(e).context("Failed to get id"))
                        })?;

                let school = row
                    .try_get::<Option<Uuid>, _>("school_id")
                    .ok()
                    .flatten()
                    .map(|id| SchoolInfo {
                        id: SchoolId::from(id),
                        name: row.try_get("school_name").unwrap_or_default(),
                        address: row.try_get("school_address").ok(),
                    });

                let level = row
                    .try_get::<Option<Uuid>, _>("level_id")
                    .ok()
                    .flatten()
                    .map(|id| LevelInfo {
                        id: LevelId::from(id),
                        name: row.try_get("level_name").unwrap_or_default(),
                        description: row.try_get("level_description").ok(),
                    });

                let branch = row
                    .try_get::<Option<Uuid>, _>("branch_id")
                    .ok()
                    .flatten()
                    .map(|id| BranchInfo {
                        id: BranchId::from(id),
                        name: row.try_get("branch_name").unwrap_or_default(),
                        description: row.try_get("branch_description").ok(),
                    });

                let roles = roles_map.get(&user_id).cloned().unwrap_or_default();

                Ok(UserWithRelations {
                    id: user_id,
                    first_name: row.try_get("first_name").map_err(|e| {
                        AppError::database(
                            anyhow::Error::new(e).context("Failed to get first_name"),
                        )
                    })?,
                    last_name: row.try_get("last_name").map_err(|e| {
                        AppError::database(anyhow::Error::new(e).context("Failed to get last_name"))
                    })?,
                    email: row.try_get("email").map_err(|e| {
                        AppError::database(anyhow::Error::new(e).context("Failed to get email"))
                    })?,
                    date_of_birth: row.try_get("date_of_birth").map_err(|e| {
                        AppError::database(
                            anyhow::Error::new(e).context("Failed to get date_of_birth"),
                        )
                    })?,
                    grade_level: row.try_get("grade_level").map_err(|e| {
                        AppError::database(
                            anyhow::Error::new(e).context("Failed to get grade_level"),
                        )
                    })?,
                    school,
                    level,
                    branch,
                    roles,
                    created_at: row.try_get("created_at").map_err(|e| {
                        AppError::database(
                            anyhow::Error::new(e).context("Failed to get created_at"),
                        )
                    })?,
                    updated_at: row.try_get("updated_at").map_err(|e| {
                        AppError::database(
                            anyhow::Error::new(e).context("Failed to get updated_at"),
                        )
                    })?,
                })
            })
            .collect();

        let users = users?;

        let total = count_query_builder
            .fetch_one(db)
            .await
            .context("Failed to count users")
            .map_err(|e| {
                error!(error = %e, "Database error counting users");
                AppError::database(e)
            })?;

        let meta = PaginationMeta {
            total,
            limit,
            offset: if page.is_none() { Some(offset) } else { None },
            page,
            has_more: offset + limit < total,
        };

        debug!(
            total = %total,
            returned = %users.len(),
            "Users fetched successfully"
        );

        Ok(PaginatedUsersResponse { data: users, meta })
    }

    #[instrument(skip(db), fields(user.id = %id))]
    pub async fn get_user(db: &PgPool, id: UserId) -> Result<User, AppError> {
        debug!("Fetching user by ID");

        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, first_name, last_name, email, school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at
            FROM users WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(db)
        .await
        .context("Failed to fetch user")
        .map_err(|e| {
            error!(error = %e, "Database error fetching user");
            AppError::database(e)
        })?
        .ok_or_else(|| {
            warn!(user.id = %id, "User not found");
            AppError::not_found(anyhow::anyhow!("User not found"))
        })?;

        debug!(user.email = %user.email, "User fetched successfully");
        Ok(user)
    }

    #[instrument(skip(db), fields(user.id = %id))]
    pub async fn get_user_with_school(db: &PgPool, id: UserId) -> Result<UserWithSchool, AppError> {
        debug!("Fetching user with school information");

        let user = Self::get_user(db, id).await?;

        let school = if let Some(school_id) = user.school_id {
            sqlx::query_as::<_, School>(
                r#"SELECT id, name, address, created_at, updated_at FROM schools WHERE id = $1"#,
            )
            .bind(school_id)
            .fetch_optional(db)
            .await
            .context("Failed to fetch school")
            .map_err(|e| {
                error!(error = %e, "Database error fetching school");
                AppError::database(e)
            })?
        } else {
            None
        };

        debug!(
            user.email = %user.email,
            has_school = school.is_some(),
            "User with school fetched successfully"
        );

        Ok(UserWithSchool { user, school })
    }

    #[instrument(skip(db, dto), fields(user.id = %user_id))]
    pub async fn update_profile(
        db: &PgPool,
        user_id: UserId,
        dto: UpdateProfileDto,
    ) -> Result<User, AppError> {
        debug!("Updating user profile");

        // Build dynamic update query
        let mut updates = vec![];
        let mut param_count = 1; // $1 is user_id

        if dto.first_name.is_some() {
            param_count += 1;
            updates.push(format!("first_name = ${}", param_count));
        }

        if dto.last_name.is_some() {
            param_count += 1;
            updates.push(format!("last_name = ${}", param_count));
        }

        if updates.is_empty() {
            return Self::get_user(db, user_id).await;
        }

        updates.push("updated_at = NOW()".to_string());

        let query = format!(
            "UPDATE users SET {} WHERE id = $1 RETURNING id, first_name, last_name, email, school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at",
            updates.join(", ")
        );

        let mut query_builder = sqlx::query_as::<_, User>(&query).bind(user_id);

        if let Some(ref first_name) = dto.first_name {
            query_builder = query_builder.bind(first_name);
        }

        if let Some(ref last_name) = dto.last_name {
            query_builder = query_builder.bind(last_name);
        }

        let user = query_builder
            .fetch_one(db)
            .await
            .context("Failed to update profile")
            .map_err(|e| {
                error!(error = %e, "Database error updating profile");
                AppError::database(e)
            })?;

        info!(user.id = %user.id, "Profile updated successfully");
        Ok(user)
    }

    #[instrument(skip(db, dto), fields(user.id = %user_id))]
    pub async fn change_password(
        db: &PgPool,
        user_id: UserId,
        dto: ChangePasswordDto,
    ) -> Result<(), AppError> {
        debug!("Changing user password");

        // Get current password hash
        let current_hash: String = sqlx::query_scalar("SELECT password FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(db)
            .await
            .context("Failed to fetch current password")
            .map_err(|e| {
                error!(error = %e, "Database error fetching password");
                AppError::database(e)
            })?;

        // Verify current password
        if !verify_password(&dto.current_password, &current_hash)? {
            warn!(user.id = %user_id, "Invalid current password provided");
            return Err(AppError::unauthorized(
                "Current password is incorrect".to_string(),
            ));
        }

        // Hash and update new password
        let new_hash = hash_password(&dto.new_password)?;

        sqlx::query("UPDATE users SET password = $1, updated_at = NOW() WHERE id = $2")
            .bind(&new_hash)
            .bind(user_id)
            .execute(db)
            .await
            .context("Failed to update password")
            .map_err(|e| {
                error!(error = %e, "Database error updating password");
                AppError::database(e)
            })?;

        info!(user.id = %user_id, "Password changed successfully");
        Ok(())
    }

    /// Check if user has a specific system role
    #[allow(dead_code)]
    pub async fn user_has_system_role(
        db: &PgPool,
        user_id: UserId,
        role_id: RoleId,
    ) -> Result<bool, AppError> {
        let has_role = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM user_roles WHERE user_id = $1 AND role_id = $2)",
        )
        .bind(user_id)
        .bind(role_id)
        .fetch_one(db)
        .await
        .context("Failed to check user role")
        .map_err(AppError::database)?;

        Ok(has_role)
    }

    /// Get user's primary role (first assigned role, preferring system roles)
    #[allow(dead_code)]
    pub async fn get_user_primary_role(
        db: &PgPool,
        user_id: UserId,
    ) -> Result<Option<RoleId>, AppError> {
        let role_id = sqlx::query_scalar::<_, RoleId>(
            r#"
            SELECT ur.role_id
            FROM user_roles ur
            INNER JOIN roles r ON ur.role_id = r.id
            WHERE ur.user_id = $1
            ORDER BY r.is_system_role DESC, ur.assigned_at ASC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(db)
        .await
        .context("Failed to fetch primary role")
        .map_err(AppError::database)?;

        Ok(role_id)
    }

    /// Check if user is a system admin
    #[allow(dead_code)]
    pub async fn is_system_admin(db: &PgPool, user_id: UserId) -> Result<bool, AppError> {
        Self::user_has_system_role(db, user_id, system_roles::SYSTEM_ADMIN).await
    }

    /// Check if user is an admin (school admin)
    #[allow(dead_code)]
    pub async fn is_admin(db: &PgPool, user_id: UserId) -> Result<bool, AppError> {
        Self::user_has_system_role(db, user_id, system_roles::ADMIN).await
    }

    /// Check if user has any of the specified roles
    pub async fn user_has_any_role(
        db: &PgPool,
        user_id: UserId,
        role_ids: &[RoleId],
    ) -> Result<bool, AppError> {
        if role_ids.is_empty() {
            return Ok(false);
        }

        let uuid_role_ids: Vec<Uuid> = role_ids.iter().map(|id| id.into_inner()).collect();
        let has_role = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM user_roles WHERE user_id = $1 AND role_id = ANY($2))",
        )
        .bind(user_id)
        .bind(&uuid_role_ids)
        .fetch_one(db)
        .await
        .context("Failed to check user roles")
        .map_err(AppError::database)?;

        Ok(has_role)
    }
}
