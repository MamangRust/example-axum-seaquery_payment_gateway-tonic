use async_trait::async_trait;
use sea_query::{Expr, Func, Order, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use tracing::{error, info};

use crate::abstract_trait::UserRepositoryTrait;
use crate::config::ConnectionPool;
use crate::domain::request::user::{CreateUserRequest, UpdateUserRequest};
use crate::model::user::User;
use crate::schema::user::Users;
use crate::utils::AppError;

pub struct UserRepository {
    db_pool: ConnectionPool,
}

impl UserRepository {
    pub fn new(db_pool: ConnectionPool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl UserRepositoryTrait for UserRepository {
    async fn find_all(
        &self,
        page: i32,
        page_size: i32,
        search: Option<String>,
    ) -> Result<(Vec<User>, i64), AppError> {
        info!(
            "👥 [Users] Fetching all users - page: {page}, page_size: {page_size}, search: {:?}",
            search
        );

        let page = if page > 0 { page } else { 1 };
        let page_size = if page_size > 0 { page_size } else { 10 };
        let offset = (page - 1) * page_size;

        info!("🔢 [Users] Using pagination: LIMIT={page_size} OFFSET={offset}");

        let mut select_query = Query::select();
        select_query
            .columns([
                Users::UserId,
                Users::Firstname,
                Users::Lastname,
                Users::Email,
                Users::Password,
                Users::NocTransfer,
                Users::CreatedAt,
                Users::UpdatedAt,
            ])
            .from(Users::Table)
            .order_by(Users::UserId, Order::Asc)
            .limit(page_size as u64)
            .offset(offset as u64);

        if let Some(ref term) = search {
            select_query.and_where(Expr::col(Users::Email).like(format!("{term}%")));
            info!("🔍 [Users] Filtering by email prefix: {}%", term);
        }

        let (sql, values) = select_query.build_sqlx(PostgresQueryBuilder);
        info!("🧾 [Users] Generated SQL: {} | Values: {:?}", sql, values);

        let users_result = sqlx::query_as_with::<_, User, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await;

        let users = match users_result {
            Ok(u) => {
                info!("✅ [Users] Successfully fetched {} user(s)", u.len());
                u
            }
            Err(e) => {
                error!("❌ [Users] Failed to fetch users from database: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        let mut count_query = Query::select();
        count_query
            .expr(Func::count(Expr::col(Users::UserId)))
            .from(Users::Table);

        if let Some(ref term) = search {
            count_query.and_where(Expr::col(Users::Email).like(format!("{term}%")));
        }

        let (count_sql, count_values) = count_query.build_sqlx(PostgresQueryBuilder);
        info!(
            "[Users] Executing count query: {count_sql} | Values: {:?}",
            count_values
        );

        let total_result = sqlx::query_as_with::<_, (i64,), _>(&count_sql, count_values)
            .fetch_one(&self.db_pool)
            .await;

        let total = match total_result {
            Ok((count,)) => {
                info!("📊 [Users] Total users matching criteria: {count}");
                count
            }
            Err(e) => {
                error!("❌ [Users] Failed to count total users: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        info!(
            "🎉 [Users] Pagination complete: {} of {total} user(s) returned",
            users.len(),
        );

        Ok((users, total))
    }

    async fn find_by_email_exists(&self, email: &str) -> Result<bool, AppError> {
        info!("🔍 Checking if user with email '{email}' exists");

        let (sql, values) = Query::select()
            .expr(Expr::col(Users::UserId).count())
            .from(Users::Table)
            .and_where(Expr::col(Users::Email).eq(email))
            .build_sqlx(PostgresQueryBuilder);

        info!(
            "🧾 Generated SQL for email existence check: {sql} | Values: {:?}",
            values
        );

        let count: i64 = sqlx::query_scalar_with(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| match &e {
                sqlx::Error::Database(db_err) => {
                    error!("🗄️ Database error while checking email '{email}': {db_err}");
                    AppError::Custom(format!("Database error: {db_err}"))
                }
                sqlx::Error::PoolTimedOut => {
                    error!(
                        "⏰ Connection pool timeout while checking email '{}'",
                        email
                    );
                    AppError::Custom("Database connection pool timeout".to_string())
                }
                _ => {
                    error!("💥 Unexpected error while checking email '{email}': {e}",);
                    AppError::InternalError(format!("Unexpected database error: {e}"))
                }
            })?;

        info!("✅ Email '{email}' exists: {}", count > 0);
        Ok(count > 0)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        info!("📧 Looking up user by email: '{}'", email);

        let (sql, values) = Query::select()
            .columns([
                Users::UserId,
                Users::Firstname,
                Users::Lastname,
                Users::Email,
                Users::Password,
                Users::NocTransfer,
                Users::CreatedAt,
                Users::UpdatedAt,
            ])
            .from(Users::Table)
            .and_where(Expr::col(Users::Email).eq(email))
            .build_sqlx(PostgresQueryBuilder);

        info!(
            "🧾 Executing query to find user by email: {sql} | Values: {:?}",
            values
        );

        let user = sqlx::query_as_with::<_, User, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ Failed to query database for email '{email}': {e}");
                AppError::SqlxError(e)
            })?;

        match user {
            Some(ref u) => {
                info!(
                    "✅ User found by email '{email}': ID={}, Name={} {}",
                    u.user_id, u.firstname, u.lastname
                );
            }
            None => {
                error!("👤 User with email '{email}' not found in database");
            }
        }

        Ok(user)
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<User>, AppError> {
        info!("🆔 Looking up user by ID: {id}");

        let (sql, values) = Query::select()
            .columns([
                Users::UserId,
                Users::Firstname,
                Users::Lastname,
                Users::Email,
                Users::Password,
                Users::NocTransfer,
                Users::CreatedAt,
                Users::UpdatedAt,
            ])
            .from(Users::Table)
            .and_where(Expr::col(Users::UserId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        info!(
            "🧾 Executing query to find user by ID: {sql} | Values: {:?}",
            values
        );

        let user = sqlx::query_as_with::<_, User, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ Database error while fetching user ID {id}: {e}");
                AppError::SqlxError(e)
            })?;

        match user {
            Some(ref u) => {
                info!(
                    "✅ User found by ID {id}: email={}, name={} {}",
                    u.email, u.firstname, u.lastname
                );
            }
            None => {
                error!("❌ User with ID {id} not found in database");
            }
        }

        Ok(user)
    }

    async fn create_user(&self, input: &CreateUserRequest) -> Result<User, AppError> {
        info!(
            "👤 [User] Creating new user: {} {}",
            input.firstname, input.lastname
        );

        let (sql, values) = Query::insert()
            .into_table(Users::Table)
            .columns([
                Users::Firstname,
                Users::Lastname,
                Users::Email,
                Users::Password,
                Users::NocTransfer,
            ])
            .values([
                input.firstname.clone().into(),
                input.lastname.clone().into(),
                input.email.clone().into(),
                input.password.clone().into(),
                input.noc_transfer.clone().into(),
            ])
            .unwrap()
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        info!("🧾 [User] INSERT query: {} | Values: {:?}", sql, values);

        let user: User = sqlx::query_as_with(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| {
                error!(
                    "❌ [User] Failed to create user '{} {}': {e}",
                    input.firstname, input.lastname,
                );
                AppError::SqlxError(e)
            })?;

        info!(
            "✅ [User] Successfully created user ID: {} | Email: {}",
            user.user_id, user.email
        );
        Ok(user)
    }

    async fn update_user(&self, input: &UpdateUserRequest) -> Result<User, AppError> {
        info!("🔄 [User] Updating user with ID: {}", input.id);

        let id = input.id;

        let mut update_query = Query::update();
        let mut query = update_query
            .table(Users::Table)
            .and_where(Expr::col(Users::UserId).eq(id));

        let mut updated_fields = Vec::new();

        if let Some(ref firstname) = input.firstname {
            query = query.value(Users::Firstname, firstname.clone());
            updated_fields.push(format!("firstname='{firstname}'"));
        }

        if let Some(ref lastname) = input.lastname {
            query = query.value(Users::Lastname, lastname.clone());
            updated_fields.push(format!("lastnameid='{lastname}'"));
        }

        if let Some(ref email) = input.email {
            query = query.value(Users::Email, email.clone());
            updated_fields.push(format!("email='{email}'"));
        }

        if updated_fields.is_empty() {
            info!("🟡 [User] No fields to update for user ID: {id}");
            return Err(AppError::Custom(
                "No fields provided for update".to_string(),
            ));
        }

        query = query.returning_all();

        let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
        info!("🧾 [User] UPDATE query: {sql} | Values: {:?}", values);
        info!("📝 [User] Updating fields: {}", updated_fields.join(", "));

        let user = sqlx::query_as_with(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => {
                    error!("❌ [User] Update failed: User with ID {id} not found");
                    AppError::NotFound(format!("User with ID {id} not found"))
                }
                _ => {
                    error!("❌ [User] Database error while updating user ID {id}: {e}",);
                    AppError::SqlxError(e)
                }
            })?;

        info!(
            "✅ [User] Successfully updated user ID: {id} | Changes: {}",
            updated_fields.join(", ")
        );
        Ok(user)
    }

    async fn delete_user(&self, id: i32) -> Result<(), AppError> {
        info!("🗑️ [User] Deleting user with ID: {}", id);

        let (sql, values) = Query::delete()
            .from_table(Users::Table)
            .and_where(Expr::col(Users::UserId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        info!("🧾 [User] DELETE query: {sql} | Values: {:?}", values);

        let result = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ [User] Failed to delete user ID {id}: {e}");
                AppError::SqlxError(e)
            })?;

        if result.rows_affected() == 0 {
            error!("❌ [User] Deletion failed: No user found with ID {id}");
            return Err(AppError::NotFound(format!("User ID {id} not found")));
        }

        info!("✅ [User] Successfully deleted user ID: {id}");
        Ok(())
    }
}
