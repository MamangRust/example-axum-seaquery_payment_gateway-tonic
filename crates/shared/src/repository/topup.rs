use crate::model::topup::Topup;
use crate::schema::topup::Topups as TopupSchema;
use crate::utils::AppError;
use crate::{
    abstract_trait::TopupRepositoryTrait,
    config::ConnectionPool,
    domain::request::topup::{CreateTopupRequest, UpdateTopupAmount, UpdateTopupRequest},
};
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use sea_query::{Expr, Func, Order, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use tracing::{error, info};

pub struct TopupRepository {
    db_pool: ConnectionPool,
}

impl TopupRepository {
    pub fn new(db_pool: ConnectionPool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl TopupRepositoryTrait for TopupRepository {
    async fn find_all(
        &self,
        page: i32,
        page_size: i32,
        search: Option<String>,
    ) -> Result<(Vec<Topup>, i64), AppError> {
        info!(
            "💳 [Topups] Fetching all topups - page: {page}, page_size: {page_size}, search: {:?}",
            search
        );

        let page = if page > 0 { page } else { 1 };
        let page_size = if page_size > 0 { page_size } else { 10 };
        let offset = (page - 1) * page_size;

        info!("🔢 [Topups] Using pagination: LIMIT={page_size} OFFSET={offset}",);

        let mut select_query = Query::select();
        select_query
            .columns([
                TopupSchema::TopupId,
                TopupSchema::UserId,
                TopupSchema::TopupNo,
                TopupSchema::TopupAmount,
                TopupSchema::TopupMethod,
                TopupSchema::TopupTime,
                TopupSchema::CreatedAt,
                TopupSchema::UpdatedAt,
            ])
            .from(TopupSchema::Table)
            .order_by(TopupSchema::TopupId, Order::Asc)
            .limit(page_size as u64)
            .offset(offset as u64);

        if let Some(ref term) = search {
            select_query.and_where(Expr::col(TopupSchema::TopupNo).like(format!("{term}%")));
            info!("🔍 [Topups] Filtering by topup_no prefix: {term}%");
        }

        let (sql, values) = select_query.build_sqlx(PostgresQueryBuilder);
        info!("🧾 [Topups] Generated SQL: {sql} | Values: {:?}", values);

        let topups_result = sqlx::query_as_with::<_, Topup, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await;

        let topups = match topups_result {
            Ok(rows) => {
                info!("✅ [Topups] Successfully fetched {} topup(s)", rows.len());
                rows
            }
            Err(e) => {
                error!("❌ [Topups] Failed to fetch topups: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        let mut count_query = Query::select();
        count_query
            .expr(Func::count(Expr::col(TopupSchema::TopupId)))
            .from(TopupSchema::Table);

        if let Some(ref term) = search {
            count_query.and_where(Expr::col(TopupSchema::TopupNo).like(format!("{term}%")));
        }

        let (count_sql, count_values) = count_query.build_sqlx(PostgresQueryBuilder);
        info!(
            "📊 [Topups] Count query: {count_sql} | Values: {:?}",
            count_values
        );

        let total_result = sqlx::query_as_with::<_, (i64,), _>(&count_sql, count_values)
            .fetch_one(&self.db_pool)
            .await;

        let total = match total_result {
            Ok((count,)) => {
                info!("📈 [Topups] Total matching topups: {count}");
                count
            }
            Err(e) => {
                error!("❌ [Topups] Failed to count total topups: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        info!(
            "🎉 [Topups] Pagination completed: {} of {total} topup(s) returned",
            topups.len(),
        );

        Ok((topups, total))
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<Topup>, AppError> {
        info!("🆔 [Topups] Finding topup by ID: {id}");

        let (sql, values) = Query::select()
            .from(TopupSchema::Table)
            .columns([
                TopupSchema::TopupId,
                TopupSchema::UserId,
                TopupSchema::TopupNo,
                TopupSchema::TopupAmount,
                TopupSchema::TopupMethod,
                TopupSchema::TopupTime,
                TopupSchema::CreatedAt,
                TopupSchema::UpdatedAt,
            ])
            .and_where(Expr::col(TopupSchema::TopupId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        info!("🧾 [Topups] Executing query: {sql} | Values: {:?}", values);

        let row = sqlx::query_as_with::<_, Topup, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ [Topups] Failed to execute query for topup_id={id}: {e}");
                AppError::SqlxError(e)
            })?;

        match &row {
            Some(topup) => {
                info!(
                    "✅ [Topups] Found! ID: {}, User ID: {}, Amount: {}, Method: {}",
                    topup.topup_id, topup.user_id, topup.topup_amount, topup.topup_method
                );
            }
            None => {
                info!("🟡 [Topups] Not found for topup_id={id}");
            }
        }

        Ok(row)
    }

    async fn find_by_users(&self, id: i32) -> Result<Vec<Topup>, AppError> {
        info!("👥 [Topups] Fetching all topups for user_id: {id}");

        let (sql, values) = Query::select()
            .from(TopupSchema::Table)
            .columns([
                TopupSchema::TopupId,
                TopupSchema::UserId,
                TopupSchema::TopupNo,
                TopupSchema::TopupAmount,
                TopupSchema::TopupMethod,
                TopupSchema::TopupTime,
                TopupSchema::CreatedAt,
                TopupSchema::UpdatedAt,
            ])
            .and_where(Expr::col(TopupSchema::UserId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        info!("🧾 [Topups] Executing query: {sql} | Values: {:?}", values);

        let rows = sqlx::query_as_with::<_, Topup, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ [Topups] Failed to fetch topups for user_id={id}: {e}",);
                AppError::SqlxError(e)
            })?;

        info!(
            "✅ [Topups] Successfully retrieved {} topup(s) for user_id={id}",
            rows.len(),
        );

        Ok(rows)
    }

    async fn find_by_user(&self, id: i32) -> Result<Option<Topup>, AppError> {
        info!("👤 [Topups] Finding one topup for user_id: {id}");

        let (sql, values) = Query::select()
            .from(TopupSchema::Table)
            .columns([
                TopupSchema::TopupId,
                TopupSchema::UserId,
                TopupSchema::TopupNo,
                TopupSchema::TopupAmount,
                TopupSchema::TopupMethod,
                TopupSchema::TopupTime,
                TopupSchema::CreatedAt,
                TopupSchema::UpdatedAt,
            ])
            .and_where(Expr::col(TopupSchema::UserId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        info!("🧾 [Topups] Executing query: {sql} | Values: {:?}", values);

        let row = sqlx::query_as_with::<_, Topup, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ [Topups] Failed to execute query for user_id={id}: {e}",);
                AppError::SqlxError(e)
            })?;

        match &row {
            Some(topup) => {
                info!(
                    "✅ [Topups] Found: topup_id={}, amount={}, method={}",
                    topup.topup_id, topup.topup_amount, topup.topup_method
                );
            }
            None => {
                info!("🟡 [Topups] No topup found for user_id={id}");
            }
        }

        Ok(row)
    }

    async fn create(&self, input: &CreateTopupRequest) -> Result<Topup, AppError> {
        info!(
            "💳 [Topups] Creating new topup: user_id={}, amount={}, method={}",
            input.user_id, input.topup_amount, input.topup_method
        );

        let now = Utc::now().naive_utc();

        let (sql, values) = Query::insert()
            .into_table(TopupSchema::Table)
            .columns([
                TopupSchema::UserId,
                TopupSchema::TopupNo,
                TopupSchema::TopupAmount,
                TopupSchema::TopupMethod,
                TopupSchema::TopupTime,
            ])
            .values([
                input.user_id.into(),
                input.topup_no.clone().into(),
                input.topup_amount.into(),
                input.topup_method.clone().into(),
                now.into(),
            ])
            .unwrap()
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        info!("🧾 [Topups] Executing INSERT: {sql} | Values: {:?}", values);

        let created = sqlx::query_as_with::<_, Topup, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| {
                error!(
                    "❌ [Topups] Failed to create topup for user_id={}: {e}",
                    input.user_id,
                );
                AppError::SqlxError(e)
            })?;

        info!(
            "✅ [Topups] Successfully created topup ID: {} | No: {}",
            created.topup_id, created.topup_no
        );

        Ok(created)
    }

    async fn update(&self, input: &UpdateTopupRequest) -> Result<Topup, AppError> {
        info!(
            "🔄 [Topups] Updating full topup with ID: {}",
            input.topup_id
        );

        let now = Utc::now().naive_utc();

        let (sql, values) = Query::update()
            .table(TopupSchema::Table)
            .values([
                (TopupSchema::TopupAmount, input.topup_amount.into()),
                (TopupSchema::TopupMethod, input.topup_method.clone().into()),
                (TopupSchema::TopupTime, now.into()),
            ])
            .and_where(Expr::col(TopupSchema::TopupId).eq(input.topup_id))
            .build_sqlx(PostgresQueryBuilder);

        info!("🧾 [Topups] Executing UPDATE: {sql} | Values: {:?}", values);

        let updated = sqlx::query_as_with::<_, Topup, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => {
                    error!(
                        "🟡 [Topups] Update failed: Topup with ID {} not found",
                        input.topup_id
                    );
                    AppError::NotFound(format!("Topup with ID {} not found", input.topup_id))
                }
                _ => {
                    error!(
                        "❌ [Topups] Database error updating topup ID {}: {e}",
                        input.topup_id,
                    );
                    AppError::SqlxError(e)
                }
            })?;

        info!(
            "✅ [Topups] Successfully updated topup ID {}: amount={}, method={}",
            updated.topup_id, updated.topup_amount, updated.topup_method
        );

        Ok(updated)
    }

    async fn update_amount(&self, input: &UpdateTopupAmount) -> Result<Topup, AppError> {
        info!(
            "💵 [Topups] Updating amount for topup ID {}: {} → {}",
            input.topup_id, "current", input.topup_amount
        );

        let (sql, values) = Query::update()
            .table(TopupSchema::Table)
            .values([(TopupSchema::TopupAmount, input.topup_amount.into())])
            .and_where(Expr::col(TopupSchema::TopupId).eq(input.topup_id))
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        info!(
            "🧾 [Topups] Executing UPDATE amount: {sql} | Values: {:?}",
            values
        );

        let updated = sqlx::query_as_with::<_, Topup, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => {
                    error!(
                        "🟡 [Topups] Amount update failed: Topup with ID {} not found",
                        input.topup_id
                    );
                    AppError::NotFound(format!("Topup with ID {} not found", input.topup_id))
                }
                _ => {
                    error!(
                        "❌ [Topups] Database error updating amount for topup ID {}: {e}",
                        input.topup_id,
                    );
                    AppError::SqlxError(e)
                }
            })?;

        info!(
            "✅ [Topups] Successfully updated amount for topup ID {}: {}",
            updated.topup_id, updated.topup_amount
        );

        Ok(updated)
    }

    async fn delete(&self, id: i32) -> Result<(), AppError> {
        info!("🗑️ [Topups] Deleting topup with ID: {id}");

        let (sql, values) = Query::delete()
            .from_table(TopupSchema::Table)
            .and_where(Expr::col(TopupSchema::TopupId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        info!("🧾 [Topups] Executing DELETE: {sql} | Values: {:?}", values);

        let result = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ [Topups] Failed to delete topup ID {id}: {e}");
                AppError::SqlxError(e)
            })?;

        if result.rows_affected() == 0 {
            error!("🟡 [Topups] Deletion failed: No topup found with ID {id}");
            return Err(AppError::NotFound(format!("Topup with ID {id} not found",)));
        }

        info!("✅ [Topups] Successfully deleted topup ID: {id}");
        Ok(())
    }
}
