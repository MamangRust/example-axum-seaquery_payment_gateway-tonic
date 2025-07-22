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
            "Getting all users - page: {page}, page_size: {page_size}, search: {:?}",
            search
        );
        let offset = (page - 1) * page_size;

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

        if let Some(term) = &search {
            select_query.and_where(Expr::col(Users::Email).like(format!("{term}%")));
        }

        let (sql, values) = select_query.build_sqlx(PostgresQueryBuilder);

        let users_result = sqlx::query_as_with::<_, User, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await;

        let users = match users_result {
            Ok(u) => u,
            Err(e) => {
                error!("Error fetching users: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        info!("Found {} users", users.len());

        let mut count_query = Query::select();
        count_query
            .expr(Func::count(Expr::col(Users::UserId)))
            .from(Users::Table);

        if let Some(term) = &search {
            count_query.and_where(Expr::col(Users::Email).like(format!("{term}%")));
        }

        let (count_sql, count_values) = count_query.build_sqlx(PostgresQueryBuilder);

        let total_result = sqlx::query_as_with::<_, (i64,), _>(&count_sql, count_values)
            .fetch_one(&self.db_pool)
            .await;

        let total = match total_result {
            Ok(count) => count.0,
            Err(e) => {
                error!("Error counting users: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        info!("Found {} users out of total {total}", users.len());

        Ok((users, total))
    }

    async fn find_by_email_exists(&self, email: &str) -> Result<bool, AppError> {
        let (sql, values) = Query::select()
            .expr(Expr::col(Users::UserId).count())
            .from(Users::Table)
            .and_where(Expr::col(Users::Email).eq(email))
            .build_sqlx(PostgresQueryBuilder);

        let count: i64 = sqlx::query_scalar_with(&sql, values)
            .fetch_one(&self.db_pool)
            .await?;

        Ok(count > 0)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        info!("Finding user by email: {}", email);

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
            .to_owned()
            .build_sqlx(PostgresQueryBuilder);

        let user = sqlx::query_as_with::<_, User, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await?;

        if user.is_none() {
            error!("User with email {email} not found");

            return Err(AppError::NotFound(format!(
                "User with email {email} not found",
            )));
        }

        Ok(user)
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<User>, AppError> {
        info!("Finding user by id: {id}");

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
            .to_owned()
            .build_sqlx(PostgresQueryBuilder);

        let user = sqlx::query_as_with(&sql, values)
            .fetch_optional(&self.db_pool)
            .await?;

        match user {
            Some(_) => {
                info!("successfully found user by id: {id}");
                Ok(user)
            }
            None => {
                error!("User with id {id} not found");
                Err(AppError::NotFound(format!("User with id {id} not found")))
            }
        }
    }

    async fn create_user(&self, input: &CreateUserRequest) -> Result<User, AppError> {
        let (sql, values) = Query::insert()
            .into_table(Users::Table)
            .columns([
                Users::Firstname,
                Users::Lastname,
                Users::Email,
                Users::Password,
                Users::NocTransfer,
                Users::CreatedAt,
                Users::UpdatedAt,
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

        let user: User = sqlx::query_as_with(&sql, values)
            .fetch_one(&self.db_pool)
            .await?;

        Ok(user)
    }

    async fn update_user(&self, input: &UpdateUserRequest) -> Result<User, AppError> {
        info!("Updating user ID {}", input.id);

        let id = input.id;

        let mut update_query = Query::update();
        let mut query = update_query
            .table(Users::Table)
            .and_where(Expr::col(Users::UserId).eq(id));

        if let Some(firstname) = &input.firstname {
            query = query.value(Users::Firstname, firstname.clone());
        }

        if let Some(lastname) = &input.lastname {
            query = query.value(Users::Lastname, lastname.clone());
        }

        if let Some(email) = &input.email {
            query = query.value(Users::Email, email.clone());
        }

        query = query.returning_all();

        let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

        let user = sqlx::query_as_with(&sql, values)
            .fetch_one(&self.db_pool)
            .await?;

        info!("User updated with ID: {id}");

        Ok(user)
    }

    async fn delete_user(&self, id: i32) -> Result<(), AppError> {
        let (sql, values) = Query::delete()
            .from_table(Users::Table)
            .and_where(Expr::col(Users::UserId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let result = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("User ID {id} not found")));
        }

        Ok(())
    }
}
