use crate::{
    metrics,
    modules::users::model::{
        ChangePasswordDto, CreateUserDto, PaginatedUsersResponse, School, UpdateProfileDto, User,
        UserFilterParams, UserRole, UserWithSchool,
    },
    utils::{
        errors::AppError,
        pagination::PaginationMeta,
        password::{hash_password, verify_password},
    },
};
use anyhow::Context;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct UserService;

impl UserService {
    pub async fn create_user(db: &PgPool, dto: CreateUserDto) -> Result<User, AppError> {
        let role = dto.role.unwrap_or_default();
        let password_hash = hash_password(&dto.password)?;

        metrics::track_user_created(&role.to_string());

        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (first_name, last_name, email, password, role, school_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, first_name, last_name, email, role as "role: _", school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at
            "#,
            dto.first_name,
            dto.last_name,
            dto.email,
            password_hash,
            role as _,
            dto.school_id
        )
        .fetch_one(db)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.is_unique_violation() {
                    return AppError::unprocessable(anyhow::anyhow!(
                        "User with this email already exists"
                    ));
                }
            }
            AppError::database(anyhow::Error::new(e).context("Failed to insert user"))
        })?;

        Ok(user)
    }

    pub async fn get_users_paginated(
        db: &PgPool,
        filters: UserFilterParams,
        school_id_filter: Option<Uuid>,
    ) -> Result<PaginatedUsersResponse, AppError> {
        let limit = filters.pagination.limit();
        let offset = filters.pagination.offset();
        let page = filters.pagination.page();

        let mut conditions = vec![];
        let mut param_count = 0;

        let mut query = String::from(
            r#"SELECT id, first_name, last_name, email, role::text as role, school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at FROM users WHERE 1=1"#,
        );

        if let Some(ref first_name) = filters.first_name {
            param_count += 1;
            conditions.push((
                param_count,
                format!("first_name ILIKE ${}", param_count),
                format!("%{}%", first_name),
            ));
        }

        if let Some(ref last_name) = filters.last_name {
            param_count += 1;
            conditions.push((
                param_count,
                format!("last_name ILIKE ${}", param_count),
                format!("%{}%", last_name),
            ));
        }

        if let Some(ref email) = filters.email {
            param_count += 1;
            conditions.push((
                param_count,
                format!("email ILIKE ${}", param_count),
                format!("%{}%", email),
            ));
        }

        if let Some(ref role) = filters.role {
            param_count += 1;
            conditions.push((
                param_count,
                format!("role = ${}", param_count),
                format!("{:?}", role).to_lowercase(),
            ));
        }

        if let Some(sid) = filters.school_id {
            param_count += 1;
            conditions.push((
                param_count,
                format!("school_id = ${}", param_count),
                sid.to_string(),
            ));
        }

        if let Some(sid) = school_id_filter {
            param_count += 1;
            conditions.push((
                param_count,
                format!("school_id = ${}", param_count),
                sid.to_string(),
            ));
        }

        for (_, condition, _) in &conditions {
            query.push_str(&format!(" AND {}", condition));
        }

        let count_query = query.replace(
            r#"SELECT id, first_name, last_name, email, role::text as role, school_id FROM users"#,
            "SELECT COUNT(*) as count FROM users",
        );

        query.push_str(&format!(
            " ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
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
            .map_err(AppError::database)?;

        let users: Result<Vec<User>, AppError> = rows
            .iter()
            .map(|row| {
                let role_str: String = row.try_get("role").map_err(|e| {
                    AppError::database(anyhow::Error::new(e).context("Failed to get role"))
                })?;
                let role = match role_str.as_str() {
                    "system_admin" => UserRole::SystemAdmin,
                    "admin" => UserRole::Admin,
                    "teacher" => UserRole::Teacher,
                    "student" => UserRole::Student,
                    _ => {
                        return Err(AppError::database(anyhow::anyhow!(
                            "Invalid role: {}",
                            role_str
                        )));
                    }
                };
                Ok(User {
                    id: row.try_get("id").map_err(|e| {
                        AppError::database(anyhow::Error::new(e).context("Failed to get id"))
                    })?,
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
                    role,
                    school_id: row.try_get("school_id").map_err(|e| {
                        AppError::database(anyhow::Error::new(e).context("Failed to get school_id"))
                    })?,
                    level_id: row.try_get("level_id").map_err(|e| {
                        AppError::database(anyhow::Error::new(e).context("Failed to get level_id"))
                    })?,
                    branch_id: row.try_get("branch_id").map_err(|e| {
                        AppError::database(anyhow::Error::new(e).context("Failed to get branch_id"))
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
            .map_err(AppError::database)?;

        let meta = PaginationMeta {
            total,
            limit,
            offset: if page.is_none() { Some(offset) } else { None },
            page,
            has_more: offset + limit < total,
        };

        Ok(PaginatedUsersResponse { data: users, meta })
    }

    pub async fn get_user(db: &PgPool, id: Uuid) -> Result<User, AppError> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, first_name, last_name, email, role as "role: _", school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(db)
        .await
        .context("Failed to fetch user by ID")
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("User with id {} not found", id)))?;

        Ok(user)
    }

    pub async fn get_user_with_school(db: &PgPool, id: Uuid) -> Result<UserWithSchool, AppError> {
        let result = sqlx::query!(
            r#"
                SELECT
                    users.id as user_id,
                    users.first_name,
                    users.last_name,
                    users.email,
                    users.role as "role: UserRole",
                    users.school_id,
                    users.level_id,
                    users.branch_id,
                    users.date_of_birth,
                    users.grade_level,
                    users.created_at,
                    users.updated_at,
                    s.id as "school_id_inner?",
                    s.name as "school_name?",
                    s.address as "school_address?"
                FROM users
                LEFT JOIN schools s ON users.school_id = s.id
                WHERE users.id = $1
                "#,
            id
        )
        .fetch_optional(db)
        .await
        .context("Failed to fetch user with school by ID")
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("User with id {} not found", id)))?;

        let user = User {
            id: result.user_id,
            first_name: result.first_name,
            last_name: result.last_name,
            email: result.email,
            role: result.role,
            school_id: result.school_id,
            level_id: result.level_id,
            branch_id: result.branch_id,
            date_of_birth: result.date_of_birth,
            grade_level: result.grade_level,
            created_at: result.created_at,
            updated_at: result.updated_at,
        };

        let school = if let (Some(school_id), Some(school_name)) =
            (result.school_id_inner, result.school_name)
        {
            Some(School {
                id: school_id,
                name: school_name,
                address: result.school_address,
            })
        } else {
            None
        };

        Ok(UserWithSchool { user, school })
    }

    pub async fn update_profile(
        db: &PgPool,
        user_id: Uuid,
        dto: UpdateProfileDto,
    ) -> Result<User, AppError> {
        let existing = Self::get_user(db, user_id).await?;

        let first_name = dto.first_name.unwrap_or(existing.first_name);
        let last_name = dto.last_name.unwrap_or(existing.last_name);

        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET first_name = $1, last_name = $2, updated_at = NOW()
            WHERE id = $3
            RETURNING id, first_name, last_name, email, role as "role: _", school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at
            "#,
            first_name,
            last_name,
            user_id
        )
        .fetch_one(db)
        .await
        .context("Failed to update user profile")
        .map_err(AppError::database)?;

        Ok(user)
    }

    pub async fn change_password(
        db: &PgPool,
        user_id: Uuid,
        dto: ChangePasswordDto,
    ) -> Result<(), AppError> {
        let password_hash =
            sqlx::query_scalar::<_, String>("SELECT password FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_optional(db)
                .await
                .context("Failed to fetch user password")
                .map_err(AppError::database)?
                .ok_or_else(|| AppError::not_found(anyhow::anyhow!("User not found")))?;

        if !verify_password(&dto.current_password, &password_hash)? {
            return Err(AppError::unprocessable(anyhow::anyhow!(
                "Current password is incorrect"
            )));
        }

        let new_password_hash = hash_password(&dto.new_password)?;

        sqlx::query("UPDATE users SET password = $1, updated_at = NOW() WHERE id = $2")
            .bind(&new_password_hash)
            .bind(user_id)
            .execute(db)
            .await
            .context("Failed to update password")
            .map_err(AppError::database)?;

        Ok(())
    }
}
