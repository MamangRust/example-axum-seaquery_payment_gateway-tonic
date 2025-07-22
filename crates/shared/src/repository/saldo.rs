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
            "Getting all saldos - page: {page}, page_size: {page_size}, search: {:?}",
            search
        );

        let page = if page > 0 { page } else { 1 };
        let page_size = if page_size > 0 { page_size } else { 10 };
        let offset = (page - 1) * page_size;

        let mut select_query = Query::select();
        select_query
            .columns([
                SaldoSchema::SaldoId,
                SaldoSchema::UserId,
                SaldoSchema::TotalBalance,
                SaldoSchema::CreatedAt,
                SaldoSchema::UpdatedAt,
            ])
            .from(SaldoSchema::Table)
            .order_by(SaldoSchema::SaldoId, Order::Asc)
            .limit(page_size as u64)
            .offset(offset as u64);

        if let Some(term) = &search {
            select_query.and_where(Expr::col(SaldoSchema::UserId).like(format!("{term}%")));
        }

        let (sql, values) = select_query.build_sqlx(PostgresQueryBuilder);

        let saldos_result = sqlx::query_as_with::<_, Saldo, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await;

        let saldos = match saldos_result {
            Ok(rows) => rows,
            Err(e) => {
                error!("Error fetching saldos: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        info!("Found {} saldos", saldos.len());

        let mut count_query = Query::select();
        count_query
            .expr(Func::count(Expr::col(SaldoSchema::SaldoId)))
            .from(SaldoSchema::Table);

        if let Some(term) = &search {
            count_query.and_where(Expr::col(SaldoSchema::UserId).like(format!("{term}%")));
        }

        let (count_sql, count_values) = count_query.build_sqlx(PostgresQueryBuilder);

        let total_result = sqlx::query_as_with::<_, (i64,), _>(&count_sql, count_values)
            .fetch_one(&self.db_pool)
            .await;

        let total = match total_result {
            Ok((count,)) => count,
            Err(e) => {
                error!("Error counting saldos: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        info!("Found {} saldos out of total {total}", saldos.len());

        Ok((saldos, total))
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<Saldo>, AppError> {
        let (sql, values) = Query::select()
            .from(SaldoSchema::Table)
            .columns([
                SaldoSchema::SaldoId,
                SaldoSchema::UserId,
                SaldoSchema::TotalBalance,
                SaldoSchema::CreatedAt,
                SaldoSchema::UpdatedAt,
            ])
            .and_where(Expr::col(SaldoSchema::SaldoId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_as_with::<_, Saldo, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await?;

        Ok(row)
    }

    async fn find_by_user_id(&self, user_id: i32) -> Result<Option<Saldo>, AppError> {
        let (sql, values) = Query::select()
            .from(SaldoSchema::Table)
            .columns([
                SaldoSchema::SaldoId,
                SaldoSchema::UserId,
                SaldoSchema::TotalBalance,
                SaldoSchema::CreatedAt,
                SaldoSchema::UpdatedAt,
            ])
            .and_where(Expr::col(SaldoSchema::UserId).eq(user_id))
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_as_with::<_, Saldo, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await?;

        Ok(row)
    }

    async fn find_by_users_id(&self, user_id: i32) -> Result<Vec<Saldo>, AppError> {
        let (sql, values) = Query::select()
            .from(SaldoSchema::Table)
            .columns([
                SaldoSchema::SaldoId,
                SaldoSchema::UserId,
                SaldoSchema::TotalBalance,
                SaldoSchema::CreatedAt,
                SaldoSchema::UpdatedAt,
            ])
            .and_where(Expr::col(SaldoSchema::UserId).eq(user_id))
            .order_by(SaldoSchema::SaldoId, Order::Asc)
            .build_sqlx(PostgresQueryBuilder);

        let rows = sqlx::query_as_with::<_, Saldo, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await?;

        Ok(rows)
    }

    async fn create(&self, input: &CreateSaldoRequest) -> Result<Saldo, AppError> {
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

        let inserted: Saldo = sqlx::query_as_with::<_, Saldo, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await?;

        Ok(inserted)
    }

    async fn update(&self, input: &UpdateSaldoRequest) -> Result<Saldo, AppError> {
        let (sql, values) = Query::select()
            .from(SaldoSchema::Table)
            .columns([SaldoSchema::SaldoId, SaldoSchema::TotalBalance])
            .and_where(Expr::col(SaldoSchema::SaldoId).eq(input.saldo_id))
            .build_sqlx(PostgresQueryBuilder);

        let saldo_record: Option<(i32, i64)> = sqlx::query_with(&sql, values)
            .map(|row: sqlx::postgres::PgRow| (row.get("saldo_id"), row.get("total_balance")))
            .fetch_optional(&self.db_pool)
            .await?;

        let (saldo_id, current_balance) =
            saldo_record.ok_or_else(|| AppError::NotFound("Saldo not found".into()))?;

        let withdraw_amount = input.withdraw_amount.unwrap_or(0);
        let updated_balance = current_balance - withdraw_amount as i64;

        if updated_balance < 50000 {
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

        let updated: Saldo = sqlx::query_as_with::<_, Saldo, _>(&update_sql, update_values)
            .fetch_one(&self.db_pool)
            .await?;

        Ok(updated)
    }

    async fn update_balance(&self, input: &UpdateSaldoBalance) -> Result<Saldo, AppError> {
        let (sql, values) = Query::select()
            .from(SaldoSchema::Table)
            .column(SaldoSchema::SaldoId)
            .and_where(Expr::col(SaldoSchema::UserId).eq(input.user_id))
            .build_sqlx(PostgresQueryBuilder);

        let saldo_id: Option<i32> = sqlx::query_with(&sql, values)
            .map(|row: sqlx::postgres::PgRow| row.get("saldo_id"))
            .fetch_optional(&self.db_pool)
            .await?;

        let saldo_id = saldo_id.ok_or(AppError::NotFound("Saldo not found".into()))?;

        let (update_sql, update_values) = Query::update()
            .table(SaldoSchema::Table)
            .values([(SaldoSchema::TotalBalance, input.total_balance.into())])
            .and_where(Expr::col(SaldoSchema::SaldoId).eq(saldo_id))
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        let updated: Saldo = sqlx::query_as_with::<_, Saldo, _>(&update_sql, update_values)
            .fetch_one(&self.db_pool)
            .await?;

        Ok(updated)
    }

    async fn update_saldo_withdraw(&self, input: &UpdateSaldoWithdraw) -> Result<Saldo, AppError> {
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
        let current_balance: i64 = row.get("total_balance");

        let withdraw_amount: i64 = input.withdraw_amount.unwrap_or(0) as i64;
        if current_balance < withdraw_amount {
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
                    input
                        .withdraw_time
                        .map(Into::into)
                        .unwrap_or(SimpleExpr::Value(Value::ChronoDateTime(Some(Box::new(
                            Utc::now().naive_utc(),
                        ))))),
                ),
            ])
            .and_where(Expr::col(SaldoSchema::SaldoId).eq(saldo_id))
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        let updated: Saldo = sqlx::query_as_with::<_, Saldo, _>(&update_sql, update_values)
            .fetch_one(&self.db_pool)
            .await?;

        Ok(updated)
    }

    async fn delete(&self, id: i32) -> Result<(), AppError> {
        info!("Deleting saldo with ID: {id}");

        let (sql, values) = Query::delete()
            .from_table(SaldoSchema::Table)
            .and_where(Expr::col(SaldoSchema::SaldoId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let result = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

        if result.rows_affected() == 0 {
            error!("No Saldo found to delete with ID: {id}");
            return Err(AppError::NotFound(format!("Saldo with ID {id} not found",)));
        }

        info!("Saldo ID: {id} deleted successfully");
        Ok(())
    }
}
