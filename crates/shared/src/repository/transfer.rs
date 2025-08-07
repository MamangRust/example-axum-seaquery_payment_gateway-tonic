use crate::model::transfer::Transfer;
use crate::schema::transfer::Transfers as TransferSchema;
use crate::utils::AppError;
use crate::{
    abstract_trait::TransferRepositoryTrait,
    config::ConnectionPool,
    domain::request::transfer::{
        CreateTransferRequest, UpdateTransferAmountRequest, UpdateTransferRequest,
    },
};
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use sea_query::{Expr, Func, Order, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use tracing::{error, info};

pub struct TransferRepository {
    db_pool: ConnectionPool,
}

impl TransferRepository {
    pub fn new(db_pool: ConnectionPool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl TransferRepositoryTrait for TransferRepository {
    async fn find_all(
        &self,
        page: i32,
        page_size: i32,
        search: Option<String>,
    ) -> Result<(Vec<Transfer>, i64), AppError> {
        info!(
            "🔄 [Transfers] Fetching transfers - page: {page}, page_size: {page_size}, search: {:?}",
            search
        );

        let page = if page > 0 { page } else { 1 };
        let page_size = if page_size > 0 { page_size } else { 10 };
        let offset = (page - 1) * page_size;

        info!("🔢 [Transfers] Using pagination: LIMIT={page_size} OFFSET={offset}",);

        let mut select_query = Query::select();
        select_query
            .columns([
                TransferSchema::TransferId,
                TransferSchema::TransferFrom,
                TransferSchema::TransferTo,
                TransferSchema::TransferAmount,
                TransferSchema::TransferTime,
                TransferSchema::CreatedAt,
                TransferSchema::UpdatedAt,
            ])
            .from(TransferSchema::Table)
            .order_by(TransferSchema::TransferId, Order::Asc)
            .limit(page_size as u64)
            .offset(offset as u64);

        if let Some(ref term) = search {
            select_query
                .and_where(Expr::col(TransferSchema::TransferFrom).like(format!("{term}%")));
            info!("🔍 [Transfers] Filtering by sender (transfer_from) like: {term}%");
        }

        let (sql, values) = select_query.build_sqlx(PostgresQueryBuilder);
        info!("🧾 [Transfers] Generated SQL: {sql} | Values: {:?}", values);

        let transfer_result = sqlx::query_as_with::<_, Transfer, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await;

        let transfers = match transfer_result {
            Ok(rows) => {
                info!(
                    "✅ [Transfers] Successfully fetched {} transfer(s)",
                    rows.len()
                );
                rows
            }
            Err(e) => {
                error!("❌ [Transfers] Failed to fetch transfers: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        let mut count_query = Query::select();
        count_query
            .expr(Func::count(Expr::col(TransferSchema::TransferId)))
            .from(TransferSchema::Table);

        if let Some(ref term) = search {
            count_query.and_where(Expr::col(TransferSchema::TransferFrom).like(format!("{term}%")));
        }

        let (count_sql, count_values) = count_query.build_sqlx(PostgresQueryBuilder);
        info!(
            "📊 [Transfers] Count query: {count_sql} | Values: {:?}",
            count_values
        );

        let total_result = sqlx::query_as_with::<_, (i64,), _>(&count_sql, count_values)
            .fetch_one(&self.db_pool)
            .await;

        let total = match total_result {
            Ok((count,)) => {
                info!("📈 [Transfers] Total matching transfers: {count}");
                count
            }
            Err(e) => {
                error!("❌ [Transfers] Failed to count total transfers: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        info!(
            "🎉 [Transfers] Pagination complete: {} of {total} transfer(s) returned",
            transfers.len(),
        );

        Ok((transfers, total))
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<Transfer>, AppError> {
        info!("🆔 [Transfers] Finding transfer by ID: {}", id);

        let (sql, values) = Query::select()
            .from(TransferSchema::Table)
            .columns([
                TransferSchema::TransferId,
                TransferSchema::TransferFrom,
                TransferSchema::TransferTo,
                TransferSchema::TransferAmount,
                TransferSchema::TransferTime,
                TransferSchema::CreatedAt,
                TransferSchema::UpdatedAt,
            ])
            .and_where(Expr::col(TransferSchema::TransferId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        info!(
            "🧾 [Transfers] Executing query: {sql} | Values: {:?}",
            values
        );

        let row = sqlx::query_as_with::<_, Transfer, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ [Transfers] Database error while fetching transfer ID {id}: {e}",);
                AppError::SqlxError(e)
            })?;

        match &row {
            Some(transfer) => {
                info!(
                    "✅ [Transfers] Found transfer: ID={}, From={}, To={}, Amount={}",
                    transfer.transfer_id,
                    transfer.transfer_from,
                    transfer.transfer_to,
                    transfer.transfer_amount
                );
            }
            None => {
                info!("🟡 [Transfers] No transfer found with ID: {id}");
            }
        }

        Ok(row)
    }

    async fn find_by_users(&self, id: i32) -> Result<Vec<Transfer>, AppError> {
        info!("👥 [Transfers] Fetching all transfers sent by user ID: {id}",);

        let (sql, values) = Query::select()
            .from(TransferSchema::Table)
            .columns([
                TransferSchema::TransferId,
                TransferSchema::TransferFrom,
                TransferSchema::TransferTo,
                TransferSchema::TransferAmount,
                TransferSchema::TransferTime,
                TransferSchema::CreatedAt,
                TransferSchema::UpdatedAt,
            ])
            .and_where(Expr::col(TransferSchema::TransferFrom).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        info!(
            "🧾 [Transfers] Executing query: {sql} | Values: {:?}",
            values
        );

        let rows = sqlx::query_as_with::<_, Transfer, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ [Transfers] Failed to fetch transfers for sender user ID {id}: {e}",);
                AppError::SqlxError(e)
            })?;

        info!(
            "✅ [Transfers] Successfully fetched {} transfer(s) for sender user ID: {id}",
            rows.len(),
        );

        Ok(rows)
    }

    async fn find_by_user(&self, user_id: i32) -> Result<Option<Transfer>, AppError> {
        info!(
            "👤 [Transfers] Finding one transfer sent by user ID: {}",
            user_id
        );

        let (sql, values) = Query::select()
            .from(TransferSchema::Table)
            .columns([
                TransferSchema::TransferId,
                TransferSchema::TransferFrom,
                TransferSchema::TransferTo,
                TransferSchema::TransferAmount,
                TransferSchema::TransferTime,
                TransferSchema::CreatedAt,
                TransferSchema::UpdatedAt,
            ])
            .and_where(Expr::col(TransferSchema::TransferFrom).eq(user_id))
            .build_sqlx(PostgresQueryBuilder);

        info!(
            "🧾 [Transfers] Executing query: {sql} | Values: {:?}",
            values
        );

        let row = sqlx::query_as_with::<_, Transfer, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| {
                error!(
                    "❌ [Transfers] Database error while finding transfer for user ID {user_id}: {e}",
                );
                AppError::SqlxError(e)
            })?;

        match &row {
            Some(transfer) => {
                info!(
                    "✅ [Transfers] Found transfer for user ID {}: ID={}, Amount={}",
                    user_id, transfer.transfer_id, transfer.transfer_amount
                );
            }
            None => {
                info!("🟡 [Transfers] No transfer found for user ID: {user_id}");
            }
        }

        Ok(row)
    }

    async fn create(&self, input: &CreateTransferRequest) -> Result<Transfer, AppError> {
        info!(
            "💸 [Transfers] Creating new transfer: {} → {} | Amount: {}",
            input.transfer_from, input.transfer_to, input.transfer_amount
        );

        let now = Utc::now().naive_utc();

        let (sql, values) = Query::insert()
            .into_table(TransferSchema::Table)
            .columns([
                TransferSchema::TransferFrom,
                TransferSchema::TransferTo,
                TransferSchema::TransferAmount,
                TransferSchema::TransferTime,
            ])
            .values([
                input.transfer_from.into(),
                input.transfer_to.into(),
                input.transfer_amount.into(),
                now.into(),
            ])
            .unwrap()
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        info!("🧾 [Transfers] INSERT query: {sql} | Values: {:?}", values);

        let created = sqlx::query_as_with::<_, Transfer, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| {
                error!(
                    "❌ [Transfers] Failed to create transfer ({} → {}): {e}",
                    input.transfer_from, input.transfer_to,
                );
                AppError::SqlxError(e)
            })?;

        info!(
            "✅ [Transfers] Successfully created transfer ID: {} | Amount: {}",
            created.transfer_id, created.transfer_amount
        );

        Ok(created)
    }

    async fn update(&self, input: &UpdateTransferRequest) -> Result<Transfer, AppError> {
        info!(
            "🔄 [Transfers] Updating full transfer with ID: {}",
            input.transfer_id
        );

        let now = Utc::now().naive_utc();

        let (sql, values) = Query::update()
            .table(TransferSchema::Table)
            .values([
                (TransferSchema::TransferFrom, input.transfer_from.into()),
                (TransferSchema::TransferTo, input.transfer_to.into()),
                (TransferSchema::TransferAmount, input.transfer_amount.into()),
                (TransferSchema::TransferTime, now.into()),
                (TransferSchema::UpdatedAt, now.into()),
            ])
            .and_where(Expr::col(TransferSchema::TransferId).eq(input.transfer_id))
            .build_sqlx(PostgresQueryBuilder);

        info!("🧾 [Transfers] UPDATE query: {sql} | Values: {:?}", values);

        let updated = sqlx::query_as_with::<_, Transfer, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => {
                    error!(
                        "❌ [Transfers] Update failed: Transfer with ID {} not found",
                        input.transfer_id
                    );
                    AppError::NotFound(format!("Transfer with ID {} not found", input.transfer_id))
                }
                _ => {
                    error!(
                        "❌ [Transfers] Database error updating transfer ID {}: {e}",
                        input.transfer_id,
                    );
                    AppError::SqlxError(e)
                }
            })?;

        info!(
            "✅ [Transfers] Updated transfer ID {}: {} → {} | Amount: {}",
            updated.transfer_id,
            updated.transfer_from,
            updated.transfer_to,
            updated.transfer_amount
        );

        Ok(updated)
    }

    async fn update_amount(
        &self,
        input: &UpdateTransferAmountRequest,
    ) -> Result<Transfer, AppError> {
        info!(
            "💱 [Transfers] Updating amount for transfer ID: {} to {}",
            input.transfer_id, input.transfer_amount
        );

        let now = Utc::now().naive_utc();

        let (sql, values) = Query::update()
            .table(TransferSchema::Table)
            .values([
                (TransferSchema::TransferAmount, input.transfer_amount.into()),
                (TransferSchema::UpdatedAt, now.into()),
            ])
            .and_where(Expr::col(TransferSchema::TransferId).eq(input.transfer_id))
            .build_sqlx(PostgresQueryBuilder);

        info!(
            "🧾 [Transfers] UPDATE amount query: {sql} | Values: {:?}",
            values
        );

        let updated = sqlx::query_as_with::<_, Transfer, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => {
                    error!(
                        "❌ [Transfers] Amount update failed: Transfer ID {} not found",
                        input.transfer_id
                    );
                    AppError::NotFound(format!("Transfer with ID {} not found", input.transfer_id))
                }
                _ => {
                    error!(
                        "❌ [Transfers] Database error updating amount for transfer ID {}: {e}",
                        input.transfer_id
                    );
                    AppError::SqlxError(e)
                }
            })?;

        info!(
            "✅ [Transfers] Successfully updated amount for transfer ID {}: {}",
            updated.transfer_id, updated.transfer_amount
        );

        Ok(updated)
    }

    async fn delete(&self, id: i32) -> Result<(), AppError> {
        info!("🗑️ [Transfers] Deleting transfer with ID: {id}");

        let (sql, values) = Query::delete()
            .from_table(TransferSchema::Table)
            .and_where(Expr::col(TransferSchema::TransferId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        info!("🧾 [Transfers] DELETE query: {sql} | Values: {:?}", values);

        let result = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ [Transfers] Failed to delete transfer ID {id}: {e}");
                AppError::SqlxError(e)
            })?;

        if result.rows_affected() == 0 {
            error!("❌ [Transfers] Deletion failed: No transfer found with ID {id}",);
            return Err(AppError::NotFound(format!(
                "Transfer with ID {id} not found"
            )));
        }

        info!("✅ [Transfers] Successfully deleted transfer ID: {id}");
        Ok(())
    }
}
