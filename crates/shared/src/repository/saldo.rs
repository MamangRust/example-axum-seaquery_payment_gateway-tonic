use crate::model::saldo::Saldo;
use crate::schema::saldo::Saldo as SaldoSchema;
use crate::utils::AppError;
use crate::{
    abstract_trait::SaldoRepositoryTrait,
    config::ConnectionPool,
    domain::request::saldo::{
        CreateSaldoRequest, UpdateSaldoBalance, UpdateSaldoRequest, UpdateSaldoWithdraw,
    },
};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use sea_query::{Expr, Func, Order, PostgresQueryBuilder, Query, SimpleExpr, Value};
use sea_query_binder::SqlxBinder;
use sqlx::Row;
use tracing::{error, info};

pub struct SaldoRepository {
    db_pool: ConnectionPool,
}

impl SaldoRepository {
    pub fn new(db_pool: ConnectionPool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl SaldoRepositoryTrait for SaldoRepository {
    async fn find_all(
        &self,
        page: i32,
        page_size: i32,
        search: Option<String>,
    ) -> Result<(Vec<Saldo>, i64), AppError> {
        info!(
            "💰 [Saldos] Fetching all saldo records - page: {page}, page_size: {page_size}, search: {:?}",
            search
        );

        let page = if page > 0 { page } else { 1 };
        let page_size = if page_size > 0 { page_size } else { 10 };
        let offset = (page - 1) * page_size;

        info!("🔢 [Saldos] Using pagination: LIMIT={page_size} OFFSET={offset}",);

        let mut select_query = Query::select();
        select_query
            .columns([
                SaldoSchema::SaldoId,
                SaldoSchema::UserId,
                SaldoSchema::TotalBalance,
                SaldoSchema::WithdrawAmount,
                SaldoSchema::WithdrawTime,
                SaldoSchema::CreatedAt,
                SaldoSchema::UpdatedAt,
            ])
            .from(SaldoSchema::Table)
            .order_by(SaldoSchema::SaldoId, Order::Asc)
            .limit(page_size as u64)
            .offset(offset as u64);

        if let Some(ref term) = search {
            select_query.and_where(Expr::col(SaldoSchema::UserId).like(format!("{term}%")));
            info!("🔍 [Saldos] Filtering by user_id prefix: {term}%");
        }

        let (sql, values) = select_query.build_sqlx(PostgresQueryBuilder);
        info!("🧾 [Saldos] Generated SQL: {sql} | Values: {:?}", values);

        let saldos_result = sqlx::query_as_with::<_, Saldo, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await;

        let saldos = match saldos_result {
            Ok(rows) => {
                info!(
                    "✅ [Saldos] Successfully fetched {} saldo record(s)",
                    rows.len()
                );
                rows
            }
            Err(e) => {
                error!("❌ [Saldos] Failed to fetch saldo records: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        let mut count_query = Query::select();
        count_query
            .expr(Func::count(Expr::col(SaldoSchema::SaldoId)))
            .from(SaldoSchema::Table);

        if let Some(ref term) = search {
            count_query.and_where(Expr::col(SaldoSchema::UserId).like(format!("{term}%")));
        }

        let (count_sql, count_values) = count_query.build_sqlx(PostgresQueryBuilder);
        info!(
            "📊 [Saldos] Count query: {count_sql} | Values: {:?}",
            count_values
        );

        let total_result = sqlx::query_as_with::<_, (i64,), _>(&count_sql, count_values)
            .fetch_one(&self.db_pool)
            .await;

        let total = match total_result {
            Ok((count,)) => {
                info!("📈 [Saldos] Total matching records: {count}");
                count
            }
            Err(e) => {
                error!("❌ [Saldos] Failed to count total saldo records: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        info!(
            "🎉 [Saldos] Pagination completed: {} of {total} record(s) returned",
            saldos.len(),
        );

        Ok((saldos, total))
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<Saldo>, AppError> {
        info!("🔍 [Saldo] Finding saldo by ID: {id}");

        let (sql, values) = Query::select()
            .from(SaldoSchema::Table)
            .columns([
                SaldoSchema::SaldoId,
                SaldoSchema::UserId,
                SaldoSchema::TotalBalance,
                SaldoSchema::WithdrawAmount,
                SaldoSchema::WithdrawTime,
                SaldoSchema::CreatedAt,
                SaldoSchema::UpdatedAt,
            ])
            .and_where(Expr::col(SaldoSchema::SaldoId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        info!("🧾 [Saldo] Executing query: {sql} | Values: {:?}", values);

        let row = sqlx::query_as_with::<_, Saldo, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ [Saldo] Failed to execute query for saldo_id={id}: {e}",);
                AppError::SqlxError(e)
            })?;

        match &row {
            Some(saldo) => {
                info!(
                    "✅ [Saldo] Found! ID: {}, User ID: {}, Balance: {}",
                    saldo.saldo_id, saldo.user_id, saldo.total_balance
                );
            }
            None => {
                info!("🟡 [Saldo] Not found for saldo_id={id}");
            }
        }

        Ok(row)
    }

    async fn find_by_user_id(&self, user_id: i32) -> Result<Option<Saldo>, AppError> {
        info!("👤 [Saldo] Finding saldo for user_id: {user_id}");

        let (sql, values) = Query::select()
            .from(SaldoSchema::Table)
            .columns([
                SaldoSchema::SaldoId,
                SaldoSchema::UserId,
                SaldoSchema::TotalBalance,
                SaldoSchema::WithdrawAmount,
                SaldoSchema::WithdrawTime,
                SaldoSchema::CreatedAt,
                SaldoSchema::UpdatedAt,
            ])
            .and_where(Expr::col(SaldoSchema::UserId).eq(user_id))
            .build_sqlx(PostgresQueryBuilder);

        info!("🧾 [Saldo] Executing query: {sql} | Values: {:?}", values);

        let row = sqlx::query_as_with::<_, Saldo, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ [Saldo] Failed to fetch saldo for user_id={user_id}: {e}");
                AppError::SqlxError(e)
            })?;

        match &row {
            Some(saldo) => {
                info!(
                    "✅ [Saldo] Found saldo for user_id={user_id}: saldo_id={}, balance={}",
                    saldo.saldo_id, saldo.total_balance
                );
            }
            None => {
                info!("🟡 [Saldo] No saldo found for user_id={user_id}");
            }
        }

        Ok(row)
    }

    async fn find_by_users_id(&self, user_id: i32) -> Result<Vec<Saldo>, AppError> {
        info!("👥 [Saldo] Finding all saldos for user_id: {user_id}");

        let (sql, values) = Query::select()
            .from(SaldoSchema::Table)
            .columns([
                SaldoSchema::SaldoId,
                SaldoSchema::UserId,
                SaldoSchema::TotalBalance,
                SaldoSchema::WithdrawAmount,
                SaldoSchema::WithdrawTime,
                SaldoSchema::CreatedAt,
                SaldoSchema::UpdatedAt,
            ])
            .and_where(Expr::col(SaldoSchema::UserId).eq(user_id))
            .order_by(SaldoSchema::SaldoId, Order::Asc)
            .build_sqlx(PostgresQueryBuilder);

        info!("🧾 [Saldo] Executing query: {sql} | Values: {:?}", values);

        let rows = sqlx::query_as_with::<_, Saldo, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ [Saldo] Failed to fetch saldos for user_id={user_id}: {e}",);
                AppError::SqlxError(e)
            })?;

        info!(
            "✅ [Saldo] Retrieved {} saldo record(s) for user_id={user_id}",
            rows.len(),
        );

        Ok(rows)
    }

    async fn create(&self, input: &CreateSaldoRequest) -> Result<Saldo, AppError> {
        info!(
            "➕ [Saldo] Creating new saldo for user_id={} with balance={}",
            input.user_id, input.total_balance
        );

        let now = chrono::Utc::now();

        let (sql, values) = Query::insert()
            .into_table(SaldoSchema::Table)
            .columns([
                SaldoSchema::UserId,
                SaldoSchema::TotalBalance,
                SaldoSchema::CreatedAt,
                SaldoSchema::UpdatedAt,
            ])
            .values([
                input.user_id.into(),
                input.total_balance.into(),
                now.into(),
                now.into(),
            ])
            .unwrap()
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        info!("🧾 [Saldo] INSERT query: {sql} | Values: {:?}", values);

        let inserted: Saldo = sqlx::query_as_with::<_, Saldo, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| {
                error!(
                    "❌ [Saldo] Failed to create saldo for user_id={}: {e}",
                    input.user_id,
                );
                AppError::SqlxError(e)
            })?;

        info!(
            "✅ [Saldo] Successfully created saldo ID: {} for user_id={}",
            inserted.saldo_id, inserted.user_id
        );

        Ok(inserted)
    }

    async fn update(&self, input: &UpdateSaldoRequest) -> Result<Saldo, AppError> {
        info!("🔄 [Saldo] Updating saldo with ID: {}", input.saldo_id);

        let (sql, values) = Query::select()
            .from(SaldoSchema::Table)
            .columns([SaldoSchema::SaldoId, SaldoSchema::TotalBalance])
            .and_where(Expr::col(SaldoSchema::SaldoId).eq(input.saldo_id))
            .build_sqlx(PostgresQueryBuilder);

        let saldo_record: Option<(i32, i64)> = sqlx::query_with(&sql, values)
            .map(|row: sqlx::postgres::PgRow| (row.get("saldo_id"), row.get("total_balance")))
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| {
                error!(
                    "❌ [Saldo] Database error while fetching current balance for saldo_id={}: {e}",
                    input.saldo_id,
                );
                AppError::SqlxError(e)
            })?;

        let (saldo_id, current_balance) =
            saldo_record.ok_or_else(|| AppError::NotFound("Saldo not found".into()))?;

        let withdraw_amount = input.withdraw_amount.unwrap_or(0);
        let updated_balance = current_balance - withdraw_amount as i64;

        if updated_balance < 50000 {
            error!(
                "⚠️ [Saldo] Insufficient balance after withdrawal: {current_balance} - {withdraw_amount} = {updated_balance} < 50000",
            );
            return Err(AppError::Custom(
                "Insufficient balance: Saldo cannot be less than 50000".into(),
            ));
        }

        let withdraw_time: NaiveDateTime = input
            .withdraw_time
            .unwrap_or_else(|| Utc::now().naive_utc());

        let (update_sql, update_values) = Query::update()
            .table(SaldoSchema::Table)
            .values([
                (SaldoSchema::TotalBalance, updated_balance.into()),
                (SaldoSchema::WithdrawAmount, withdraw_amount.into()),
                (SaldoSchema::WithdrawTime, withdraw_time.into()),
            ])
            .and_where(Expr::col(SaldoSchema::SaldoId).eq(saldo_id))
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        info!(
            "🧾 [Saldo] Executing UPDATE: {update_sql} | Values: {:?}",
            update_values
        );

        let updated: Saldo = sqlx::query_as_with::<_, Saldo, _>(&update_sql, update_values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ [Saldo] Failed to update saldo ID {saldo_id}: {e}");
                AppError::SqlxError(e)
            })?;

        info!(
            "✅ [Saldo] Successfully updated saldo ID {}: new balance={}, withdraw_amount={}",
            updated.saldo_id,
            updated.total_balance,
            updated.withdraw_amount.unwrap().clone()
        );

        Ok(updated)
    }

    async fn update_balance(&self, input: &UpdateSaldoBalance) -> Result<Saldo, AppError> {
        info!(
            "💵 [Saldo] Updating balance for user_id={} to {}",
            input.user_id, input.total_balance
        );

        let (sql, values) = Query::select()
            .from(SaldoSchema::Table)
            .column(SaldoSchema::SaldoId)
            .and_where(Expr::col(SaldoSchema::UserId).eq(input.user_id))
            .build_sqlx(PostgresQueryBuilder);

        let saldo_id: Option<i32> = sqlx::query_with(&sql, values)
            .map(|row: sqlx::postgres::PgRow| row.get("saldo_id"))
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| {
                error!(
                    "❌ [Saldo] Database error while fetching saldo_id for user_id={}: {e}",
                    input.user_id,
                );
                AppError::SqlxError(e)
            })?;

        let saldo_id = saldo_id.ok_or(AppError::NotFound("Saldo not found".into()))?;

        let (update_sql, update_values) = Query::update()
            .table(SaldoSchema::Table)
            .values([(SaldoSchema::TotalBalance, input.total_balance.into())])
            .and_where(Expr::col(SaldoSchema::SaldoId).eq(saldo_id))
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        info!(
            "🧾 [Saldo] Executing balance update: {update_sql} | Values: {:?}",
            update_values
        );

        let updated: Saldo = sqlx::query_as_with::<_, Saldo, _>(&update_sql, update_values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ [Saldo] Failed to update balance for saldo_id={saldo_id}: {e}");
                AppError::SqlxError(e)
            })?;

        info!(
            "✅ [Saldo] Balance updated successfully: saldo_id={} → {}",
            updated.saldo_id, updated.total_balance
        );

        Ok(updated)
    }

    async fn update_saldo_withdraw(&self, input: &UpdateSaldoWithdraw) -> Result<Saldo, AppError> {
        info!(
            "💸 [Saldo] Processing withdrawal for user_id={} | Amount: {}",
            input.user_id,
            input.withdraw_amount.unwrap_or(0)
        );

        let (select_sql, select_values) = Query::select()
            .from(SaldoSchema::Table)
            .columns([SaldoSchema::SaldoId, SaldoSchema::TotalBalance])
            .and_where(Expr::col(SaldoSchema::UserId).eq(input.user_id))
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_with(&select_sql, select_values)
            .fetch_optional(&self.db_pool)
            .await?
            .ok_or(AppError::NotFound("Saldo not found".into()))?;

        let saldo_id: i32 = row.get("saldo_id");
        let current_balance: i32 = row.get("total_balance");

        let withdraw_amount: i32 = input.withdraw_amount.unwrap_or(0);
        if current_balance < withdraw_amount {
            error!(
                "❌ [Saldo] Insufficient balance: {} < {} for user_id={}",
                current_balance, withdraw_amount, input.user_id
            );
            return Err(AppError::Custom("Insufficient balance".into()));
        }

        let new_balance = current_balance - withdraw_amount;

        let (update_sql, update_values) = Query::update()
            .table(SaldoSchema::Table)
            .values([
                (SaldoSchema::TotalBalance, new_balance.into()),
                (SaldoSchema::WithdrawAmount, withdraw_amount.into()),
                (
                    SaldoSchema::WithdrawTime,
                    input.withdraw_time.map(Into::into).unwrap_or_else(|| {
                        SimpleExpr::Value(Value::ChronoDateTime(Some(Box::new(
                            Utc::now().naive_utc(),
                        ))))
                    }),
                ),
            ])
            .and_where(Expr::col(SaldoSchema::SaldoId).eq(saldo_id))
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        let updated: Saldo = sqlx::query_as_with::<_, Saldo, _>(&update_sql, update_values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| {
                error!(
                    "❌ [Saldo] Failed to update saldo (withdraw) for user_id={}: {e}",
                    input.user_id,
                );
                AppError::SqlxError(e)
            })?;

        info!(
            "✅ [Saldo] Withdraw processed: user_id={} | Old: {}, New: {}",
            input.user_id, current_balance, new_balance
        );

        Ok(updated)
    }

    async fn delete(&self, id: i32) -> Result<(), AppError> {
        info!("🗑️ [Saldo] Deleting saldo with ID: {id}");

        let (sql, values) = Query::delete()
            .from_table(SaldoSchema::Table)
            .and_where(Expr::col(SaldoSchema::SaldoId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        info!("🧾 [Saldo] DELETE query: {sql} | Values: {:?}", values);

        let result = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await
            .map_err(|e| {
                error!("❌ [Saldo] Failed to delete saldo ID {id}: {e}");
                AppError::SqlxError(e)
            })?;

        if result.rows_affected() == 0 {
            error!("❌ [Saldo] Deletion failed: No saldo found with ID {id}");
            return Err(AppError::NotFound(format!("Saldo with ID {id} not found",)));
        }

        info!("✅ [Saldo] Successfully deleted saldo ID: {id}");
        Ok(())
    }
}
