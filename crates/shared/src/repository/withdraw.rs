use crate::model::withdraw::Withdraw;
use crate::schema::withdraw::Withdraws as WithdrawSchema;
use crate::utils::AppError;
use crate::{
    abstract_trait::WithdrawRepositoryTrait,
    config::ConnectionPool,
    domain::request::withdraw::{CreateWithdrawRequest, UpdateWithdrawRequest},
};
use anyhow::Result;
use async_trait::async_trait;
use sea_query::{Expr, Func, Order, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use tracing::{error, info};

pub struct WithdrawRepository {
    db_pool: ConnectionPool,
}

impl WithdrawRepository {
    pub fn new(db_pool: ConnectionPool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl WithdrawRepositoryTrait for WithdrawRepository {
    async fn find_all(
        &self,
        page: i32,
        page_size: i32,
        search: Option<String>,
    ) -> Result<(Vec<Withdraw>, i64), AppError> {
        info!(
            "Getting all withdraws - page: {page}, page_size: {page_size}, search: {:?}",
            search
        );

        let page = if page > 0 { page } else { 1 };
        let page_size = if page_size > 0 { page_size } else { 10 };
        let offset = (page - 1) * page_size;

        let mut select_query = Query::select();
        select_query
            .columns([
                WithdrawSchema::WithdrawId,
                WithdrawSchema::UserId,
                WithdrawSchema::WithdrawAmount,
                WithdrawSchema::WithdrawTime,
                WithdrawSchema::CreatedAt,
                WithdrawSchema::UpdatedAt,
            ])
            .from(WithdrawSchema::Table)
            .order_by(WithdrawSchema::WithdrawId, Order::Asc)
            .limit(page_size as u64)
            .offset(offset as u64);

        if let Some(term) = &search {
            select_query.and_where(
                Expr::col(WithdrawSchema::WithdrawId).eq(term.parse::<i32>().unwrap_or(0)),
            );
        }

        let (sql, values) = select_query.build_sqlx(PostgresQueryBuilder);

        let withdraws_result = sqlx::query_as_with::<_, Withdraw, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await;

        let withdraws = match withdraws_result {
            Ok(rows) => rows,
            Err(e) => {
                error!("Error fetching withdraws: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        info!("Found {} withdraws", withdraws.len());

        let mut count_query = Query::select();
        count_query
            .expr(Func::count(Expr::col(WithdrawSchema::WithdrawId)))
            .from(WithdrawSchema::Table);

        if let Some(term) = &search {
            count_query.and_where(
                Expr::col(WithdrawSchema::WithdrawId).eq(term.parse::<i32>().unwrap_or(0)),
            );
        }

        let (count_sql, count_values) = count_query.build_sqlx(PostgresQueryBuilder);

        let total_result = sqlx::query_as_with::<_, (i64,), _>(&count_sql, count_values)
            .fetch_one(&self.db_pool)
            .await;

        let total = match total_result {
            Ok((count,)) => count,
            Err(e) => {
                error!("Error counting withdraws: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        info!("Found {} withdraws out of total {total}", withdraws.len());

        Ok((withdraws, total))
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<Withdraw>, AppError> {
        let (sql, values) = Query::select()
            .from(WithdrawSchema::Table)
            .columns([
                WithdrawSchema::WithdrawId,
                WithdrawSchema::UserId,
                WithdrawSchema::WithdrawAmount,
                WithdrawSchema::WithdrawTime,
                WithdrawSchema::CreatedAt,
                WithdrawSchema::UpdatedAt,
            ])
            .and_where(Expr::col(WithdrawSchema::WithdrawId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_as_with::<_, Withdraw, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await?;

        Ok(row)
    }

    async fn find_by_users(&self, id: i32) -> Result<Vec<Withdraw>, AppError> {
        let (sql, values) = Query::select()
            .from(WithdrawSchema::Table)
            .columns([
                WithdrawSchema::WithdrawId,
                WithdrawSchema::UserId,
                WithdrawSchema::WithdrawAmount,
                WithdrawSchema::WithdrawTime,
                WithdrawSchema::CreatedAt,
                WithdrawSchema::UpdatedAt,
            ])
            .and_where(Expr::col(WithdrawSchema::UserId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let rows = sqlx::query_as_with::<_, Withdraw, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await?;

        Ok(rows)
    }

    async fn find_by_user(&self, id: i32) -> Result<Option<Withdraw>, AppError> {
        let (sql, values) = Query::select()
            .from(WithdrawSchema::Table)
            .columns([
                WithdrawSchema::WithdrawId,
                WithdrawSchema::UserId,
                WithdrawSchema::WithdrawAmount,
                WithdrawSchema::WithdrawTime,
                WithdrawSchema::CreatedAt,
                WithdrawSchema::UpdatedAt,
            ])
            .and_where(Expr::col(WithdrawSchema::UserId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_as_with::<_, Withdraw, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await?;

        Ok(row)
    }

    async fn create(&self, input: &CreateWithdrawRequest) -> Result<Withdraw, AppError> {
        let withdraw_time_naive = input.withdraw_time.naive_utc();

        let (sql, values) = Query::insert()
            .into_table(WithdrawSchema::Table)
            .columns([
                WithdrawSchema::UserId,
                WithdrawSchema::WithdrawAmount,
                WithdrawSchema::WithdrawTime,
            ])
            .values([
                input.user_id.into(),
                input.withdraw_amount.into(),
                withdraw_time_naive.into(),
            ])
            .unwrap()
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_as_with::<_, Withdraw, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

        Ok(row)
    }

    async fn update(&self, input: &UpdateWithdrawRequest) -> Result<Withdraw, AppError> {
        let withdraw_time_naive = input.withdraw_time.naive_utc();

        let (sql, values) = Query::update()
            .table(WithdrawSchema::Table)
            .values([
                (WithdrawSchema::WithdrawAmount, input.withdraw_amount.into()),
                (WithdrawSchema::WithdrawTime, withdraw_time_naive.into()),
            ])
            .and_where(Expr::col(WithdrawSchema::WithdrawId).eq(input.withdraw_id))
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_as_with::<_, Withdraw, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => {
                    AppError::NotFound(format!("Withdraw with ID {} not found", input.withdraw_id))
                }
                other => AppError::SqlxError(other),
            })?;

        Ok(row)
    }

    async fn delete(&self, id: i32) -> Result<(), AppError> {
        let (sql, values) = Query::delete()
            .from_table(WithdrawSchema::Table)
            .and_where(Expr::col(WithdrawSchema::WithdrawId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let result = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

        if result.rows_affected() == 0 {
            error!("No Withdraw found to delete with ID: {id}");
            return Err(AppError::NotFound(format!(
                "Withdraw with ID {id} not found",
            )));
        }

        info!("Withdraw ID: {id} deleted successfully");
        Ok(())
    }
}
