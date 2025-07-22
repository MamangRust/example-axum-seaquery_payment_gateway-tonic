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
            "Getting all Transfer - page: {page}, page_size: {page_size}, search: {:?}",
            search
        );

        let page = if page > 0 { page } else { 1 };
        let page_size = if page_size > 0 { page_size } else { 10 };
        let offset = (page - 1) * page_size;

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

        if let Some(term) = &search {
            select_query
                .and_where(Expr::col(TransferSchema::TransferFrom).like(format!("{term}%")));
        }

        let (sql, values) = select_query.build_sqlx(PostgresQueryBuilder);

        let transfer_schema_result = sqlx::query_as_with::<_, Transfer, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await;

        let transfer_schema = match transfer_schema_result {
            Ok(rows) => rows,
            Err(e) => {
                error!("Error fetching Transfer: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        info!("Found {} Transfer", transfer_schema.len());

        let mut count_query = Query::select();
        count_query
            .expr(Func::count(Expr::col(TransferSchema::TransferId)))
            .from(TransferSchema::Table);

        if let Some(term) = &search {
            count_query.and_where(Expr::col(TransferSchema::TransferFrom).like(format!("{term}%")));
        }

        let (count_sql, count_values) = count_query.build_sqlx(PostgresQueryBuilder);

        let total_result = sqlx::query_as_with::<_, (i64,), _>(&count_sql, count_values)
            .fetch_one(&self.db_pool)
            .await;

        let total = match total_result {
            Ok((count,)) => count,
            Err(e) => {
                error!("Error counting Transfer: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        info!(
            "Found {} Transfer out of total {total}",
            transfer_schema.len()
        );

        Ok((transfer_schema, total))
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<Transfer>, AppError> {
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

        let row = sqlx::query_as_with::<_, Transfer, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await?;

        Ok(row)
    }

    async fn find_by_users(&self, id: i32) -> Result<Vec<Transfer>, AppError> {
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

        let rows = sqlx::query_as_with::<_, Transfer, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await?;

        Ok(rows)
    }

    async fn find_by_user(&self, user_id: i32) -> Result<Option<Transfer>, AppError> {
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

        let row = sqlx::query_as_with::<_, Transfer, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await?;

        Ok(row)
    }

    async fn create(&self, input: &CreateTransferRequest) -> Result<Transfer, AppError> {
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

        let created = sqlx::query_as_with::<_, Transfer, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

        Ok(created)
    }

    async fn update(&self, input: &UpdateTransferRequest) -> Result<Transfer, AppError> {
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

        let updated = sqlx::query_as_with::<_, Transfer, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

        Ok(updated)
    }

    async fn update_amount(
        &self,
        input: &UpdateTransferAmountRequest,
    ) -> Result<Transfer, AppError> {
        let now = Utc::now().naive_utc();

        let (sql, values) = Query::update()
            .table(TransferSchema::Table)
            .values([
                (TransferSchema::TransferAmount, input.transfer_amount.into()),
                (TransferSchema::UpdatedAt, now.into()),
            ])
            .and_where(Expr::col(TransferSchema::TransferId).eq(input.transfer_id))
            .build_sqlx(PostgresQueryBuilder);

        let updated = sqlx::query_as_with::<_, Transfer, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

        Ok(updated)
    }

    async fn delete(&self, id: i32) -> Result<(), AppError> {
        let (sql, values) = Query::delete()
            .from_table(TransferSchema::Table)
            .and_where(Expr::col(TransferSchema::TransferId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let result = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

        if result.rows_affected() == 0 {
            error!("No Transfer found to delete with ID: {id}");
            return Err(AppError::NotFound(format!(
                "Transfer with ID {id} not found",
            )));
        }

        info!("Transfer ID: {id} deleted successfully");
        Ok(())
    }
}
