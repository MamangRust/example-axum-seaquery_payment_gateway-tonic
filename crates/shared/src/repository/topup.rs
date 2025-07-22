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
            "Getting all topups - page: {page}, page_size: {page_size}, search: {:?}",
            search
        );

        let page = if page > 0 { page } else { 1 };
        let page_size = if page_size > 0 { page_size } else { 10 };
        let offset = (page - 1) * page_size;

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

        if let Some(term) = &search {
            select_query.and_where(Expr::col(TopupSchema::TopupNo).like(format!("{term}%")));
        }

        let (sql, values) = select_query.build_sqlx(PostgresQueryBuilder);

        let topups_result = sqlx::query_as_with::<_, Topup, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await;

        let topups = match topups_result {
            Ok(rows) => rows,
            Err(e) => {
                error!("Error fetching topups: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        info!("Found {} topups", topups.len());

        let mut count_query = Query::select();
        count_query
            .expr(Func::count(Expr::col(TopupSchema::TopupId)))
            .from(TopupSchema::Table);

        if let Some(term) = &search {
            count_query.and_where(Expr::col(TopupSchema::TopupNo).like(format!("{term}%")));
        }

        let (count_sql, count_values) = count_query.build_sqlx(PostgresQueryBuilder);

        let total_result = sqlx::query_as_with::<_, (i64,), _>(&count_sql, count_values)
            .fetch_one(&self.db_pool)
            .await;

        let total = match total_result {
            Ok((count,)) => count,
            Err(e) => {
                error!("Error counting topups: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        info!("Found {} topups out of total {total}", topups.len());

        Ok((topups, total))
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<Topup>, AppError> {
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

        let row = sqlx::query_as_with::<_, Topup, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await?;

        Ok(row)
    }

    async fn find_by_users(&self, id: i32) -> Result<Vec<Topup>, AppError> {
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

        let rows = sqlx::query_as_with::<_, Topup, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await?;

        Ok(rows)
    }

    async fn find_by_user(&self, id: i32) -> Result<Option<Topup>, AppError> {
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

        let row = sqlx::query_as_with::<_, Topup, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await?;

        Ok(row)
    }
    async fn create(&self, input: &CreateTopupRequest) -> Result<Topup, AppError> {
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
                Utc::now().naive_utc().into(),
            ])
            .unwrap()
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        let created = sqlx::query_as_with(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

        Ok(created)
    }

    async fn update(&self, input: &UpdateTopupRequest) -> Result<Topup, AppError> {
        let (sql, values) = Query::update()
            .table(TopupSchema::Table)
            .values([
                (TopupSchema::TopupAmount, input.topup_amount.into()),
                (TopupSchema::TopupMethod, input.topup_method.clone().into()),
                (TopupSchema::TopupTime, Utc::now().naive_utc().into()),
            ])
            .and_where(Expr::col(TopupSchema::TopupId).eq(input.topup_id))
            .build_sqlx(PostgresQueryBuilder);

        let updated = sqlx::query_as_with(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

        Ok(updated)
    }

    async fn update_amount(&self, input: &UpdateTopupAmount) -> Result<Topup, AppError> {
        let (sql, values) = Query::update()
            .table(TopupSchema::Table)
            .values([(TopupSchema::TopupAmount, input.topup_amount.into())])
            .and_where(Expr::col(TopupSchema::TopupId).eq(input.topup_id))
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        let updated = sqlx::query_as_with::<_, Topup, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

        Ok(updated)
    }

    async fn delete(&self, id: i32) -> Result<(), AppError> {
        let (sql, values) = Query::delete()
            .from_table(TopupSchema::Table)
            .and_where(Expr::col(TopupSchema::TopupId).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let result = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

        if result.rows_affected() == 0 {
            error!("No Topup found to delete with ID: {id}");
            return Err(AppError::NotFound(format!("Topup with ID {id} not found",)));
        }

        info!("Topup ID: {id} deleted successfully");
        Ok(())
    }
}
