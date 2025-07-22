use sqlx::{Pool, Postgres, postgres::PgPoolOptions};

pub type ConnectionPool = Pool<Postgres>;

pub struct ConnectionManager;

impl ConnectionManager {
    pub async fn new_pool(
        connection_string: &str,
        run_migrations: bool,
    ) -> anyhow::Result<ConnectionPool> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(connection_string)
            .await
            .map_err(|err| anyhow::anyhow!("Failed to create database connection pool: {}", err))?;

        if run_migrations {
            sqlx::migrate!("../../migrations")
                .run(&pool)
                .await
                .map_err(|err| anyhow::anyhow!("Failed to run database migrations: {}", err))?;
        }

        Ok(pool)
    }
}
