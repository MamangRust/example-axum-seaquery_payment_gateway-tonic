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
use chrono::{DateTime, Utc};
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
            "📄 [Withdraw] Fetching all records - page: {}, page_size: {}, search: {:?}",
            page, page_size, search
        );

        let page = if page > 0 { page } else { 1 };
        let page_size = if page_size > 0 { page_size } else { 10 };
        let offset = (page - 1) * page_size;

        info!(
            "🔢 [Withdraw] Using pagination: LIMIT={} OFFSET={}",
            page_size, offset
        );

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

        if let Some(ref term) = search {
            let search_id: i32 = term.parse().unwrap_or(0);
            select_query.and_where(Expr::col(WithdrawSchema::WithdrawId).eq(search_id));
            info!("🔍 [Withdraw] Filtering by withdraw_id = {}", search_id);
        }

        let (sql, values) = select_query.build_sqlx(PostgresQueryBuilder);
        info!(
            "🧾 [Withdraw] Generated SQL: {} | Values: {:?}",
            sql, values
        );

        let withdraws_result = sqlx::query_as_with::<_, Withdraw, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await;

        let withdraws = match withdraws_result {
            Ok(rows) => {
                info!("✅ [Withdraw] Successfully fetched {} records", rows.len());
                rows
            }
            Err(e) => {
                error!(
                    "❌ [Withdraw] Failed to fetch withdraws from database: {}",
                    e
                );
                return Err(AppError::SqlxError(e));
            }
        };

        let mut count_query = Query::select();
        count_query
            .expr(Func::count(Expr::col(WithdrawSchema::WithdrawId)))
            .from(WithdrawSchema::Table);

        if let Some(ref term) = search {
            let search_id: i32 = term.parse().unwrap_or(0);
            count_query.and_where(Expr::col(WithdrawSchema::WithdrawId).eq(search_id));
        }

        let (count_sql, count_values) = count_query.build_sqlx(PostgresQueryBuilder);
        info!(
            "🧮 [Withdraw] Count query: {} | Values: {:?}",
            count_sql, count_values
        );

        let total_result = sqlx::query_as_with::<_, (i64,), _>(&count_sql, count_values)
            .fetch_one(&self.db_pool)
            .await;

        let total = match total_result {
            Ok((count,)) => {
                info!("📊 [Withdraw] Total matching records: {}", count);
                count
            }
            Err(e) => {
                error!("❌ [Withdraw] Failed to count total withdraws: {}", e);
                return Err(AppError::SqlxError(e));
            }
        };

        info!(
            "🎉 [Withdraw] Pagination completed: {} of {} records returned",
            withdraws.len(),
            total
        );

        Ok((withdraws, total))
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<Withdraw>, AppError> {
        info!("🆔 [Withdraw] Finding withdraw by ID: {}", id);

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

        info!(
            "🧾 [Withdraw] Executing query: {} | Values: {:?}",
            sql, values
        );

        let row = sqlx::query_as_with::<_, Withdraw, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| {
                error!(
                    "❌ [Withdraw] Failed to execute query for withdraw_id={}: {}",
                    id, e
                );
                AppError::SqlxError(e)
            })?;

        match &row {
            Some(withdraw) => {
                info!(
                    "✅ [Withdraw] Found! ID: {}, User ID: {}, Amount: {}",
                    withdraw.withdraw_id, withdraw.user_id, withdraw.withdraw_amount
                );
            }
            None => {
                info!("🟡 [Withdraw] Not found for withdraw_id={}", id);
            }
        }

        Ok(row)
    }

    async fn find_by_users(&self, id: i32) -> Result<Vec<Withdraw>, AppError> {
        info!("👥 [Withdraw] Finding all withdraws for user_id: {}", id);

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

        info!(
            "🧾 [Withdraw] Executing query: {} | Values: {:?}",
            sql, values
        );

        let rows = sqlx::query_as_with::<_, Withdraw, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await
            .map_err(|e| {
                error!(
                    "❌ [Withdraw] Failed to fetch withdraws for user_id={}: {}",
                    id, e
                );
                AppError::SqlxError(e)
            })?;

        info!(
            "✅ [Withdraw] Successfully retrieved {} record(s) for user_id={}",
            rows.len(),
            id
        );

        Ok(rows)
    }

    async fn find_by_user(&self, id: i32) -> Result<Option<Withdraw>, AppError> {
        info!("👤 [Withdraw] Finding one withdraw for user_id: {}", id);

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

        info!(
            "🧾 [Withdraw] Executing query: {} | Values: {:?}",
            sql, values
        );

        let row = sqlx::query_as_with::<_, Withdraw, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| {
                error!(
                    "❌ [Withdraw] Failed to execute query for user_id={}: {}",
                    id, e
                );
                AppError::SqlxError(e)
            })?;

        match &row {
            Some(withdraw) => {
                info!(
                    "✅ [Withdraw] Found: withdraw_id={}, amount={}, time={}",
                    withdraw.withdraw_id, withdraw.withdraw_amount, withdraw.withdraw_time
                );
            }
            None => {
                info!("🟡 [Withdraw] No data found for user_id={}", id);
            }
        }

        Ok(row)
    }

    async fn create(&self, input: &CreateWithdrawRequest) -> Result<Withdraw, AppError> {
        info!(
            "💸 [Withdraw] Creating new withdrawal: user_id={}, amount={}, time={}",
            input.user_id, input.withdraw_amount, input.withdraw_time
        );

        let withdraw_time = DateTime::parse_from_rfc3339(&input.withdraw_time)
            .map_err(|e| {
                error!(
                    "❌ [Withdraw] Invalid datetime string '{}': {}",
                    input.withdraw_time, e
                );
                AppError::Custom(
                    "Invalid datetime format. Use RFC3339 (e.g., 2024-01-01T12:00:00Z)."
                        .to_string(),
                )
            })?
            .with_timezone(&Utc);

        let withdraw_time_naive = withdraw_time.naive_utc().into();

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
                withdraw_time_naive,
            ])
            .unwrap()
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        info!(
            "🧾 [Withdraw] Executing INSERT: {sql} | Values: {:?}",
            values
        );

        let row = sqlx::query_as_with::<_, Withdraw, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ [Withdraw] Failed to create withdrawal: {e}");
                AppError::SqlxError(e)
            })?;

        info!(
            "✅ [Withdraw] Successfully created! withdraw_id={} for user_id={}",
            row.withdraw_id, row.user_id
        );

        Ok(row)
    }

    async fn update(&self, input: &UpdateWithdrawRequest) -> Result<Withdraw, AppError> {
        info!(
            "🔄 [Withdraw] Updating withdrawal: id={}, amount={}, time={}",
            input.withdraw_id, input.withdraw_amount, input.withdraw_time
        );

        let withdraw_time = DateTime::parse_from_rfc3339(&input.withdraw_time)
            .map_err(|e| {
                error!(
                    "❌ [Withdraw] Invalid datetime string '{}': {}",
                    input.withdraw_time, e
                );
                AppError::Custom(
                    "Invalid datetime format. Use RFC3339 (e.g., 2024-01-01T12:00:00Z)."
                        .to_string(),
                )
            })?
            .with_timezone(&Utc);

        let withdraw_time_naive = withdraw_time.naive_utc().into();

        let (sql, values) = Query::update()
            .table(WithdrawSchema::Table)
            .values([
                (WithdrawSchema::WithdrawAmount, input.withdraw_amount.into()),
                (WithdrawSchema::WithdrawTime, withdraw_time_naive),
            ])
            .and_where(Expr::col(WithdrawSchema::WithdrawId).eq(input.withdraw_id))
            .build_sqlx(PostgresQueryBuilder);

        info!(
            "🧾 [Withdraw] Executing UPDATE: {} | Values: {:?}",
            sql, values
        );

        let row = sqlx::query_as_with::<_, Withdraw, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => {
                    error!(
                        "🟡 [Withdraw] Not found: Withdraw with ID {} does not exist",
                        input.withdraw_id
                    );
                    AppError::NotFound(format!("Withdraw with ID {} not found", input.withdraw_id))
                }
                other => {
                    error!(
                        "❌ [Withdraw] Failed to update withdraw ID {}: {}",
                        input.withdraw_id, other
                    );
                    AppError::SqlxError(other)
                }
            })?;

        info!(
            "✅ [Withdraw] Successfully updated: withdraw_id={} → amount={}, time={}",
            row.withdraw_id, row.withdraw_amount, row.withdraw_time
        );

        Ok(row)
    }

    async fn delete(&self, id: i32) -> Result<(), AppError> {
        info!("🗑️ [Withdraw] Deleting withdrawal with ID: {}", id);

        let (sql, values) = Query::delete()
            .from_table(WithdrawSchema::Table)
            .and_where(Expr::col(WithdrawSchema::WithdrawId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        info!(
            "🧾 [Withdraw] Executing DELETE: {} | Values: {:?}",
            sql, values
        );

        let result = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ [Withdraw] Failed to delete withdraw ID {}: {}", id, e);
                AppError::SqlxError(e)
            })?;

        if result.rows_affected() == 0 {
            error!(
                "🟡 [Withdraw] No data deleted: Withdraw with ID {} not found",
                id
            );
            return Err(AppError::NotFound(format!(
                "Withdraw with ID {id} not found",
            )));
        }

        info!("✅ [Withdraw] Successfully deleted: ID={}", id);
        Ok(())
    }
}
