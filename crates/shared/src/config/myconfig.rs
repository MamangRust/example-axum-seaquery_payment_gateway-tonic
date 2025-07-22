use anyhow::{Context, Result, anyhow};

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub run_migrations: bool,
    pub port: u16,
}

impl Config {
    pub fn init() -> Result<Self> {
        let database_url =
            std::env::var("DATABASE_URL").context("Missing environment variable: DATABASE_URL")?;

        let jwt_secret =
            std::env::var("JWT_SECRET").context("Missing environment variable: JWT_SECRET")?;

        let run_migrations_str = std::env::var("RUN_MIGRATIONS")
            .context("Missing environment variable: RUN_MIGRATIONS")?;

        let port_str = std::env::var("PORT").context("Missing environment variable: PORT")?;

        let run_migrations = match run_migrations_str.as_str() {
            "true" => true,
            "false" => false,
            other => {
                return Err(anyhow!(
                    "RUN_MIGRATIONS must be 'true' or 'false', got '{}'",
                    other
                ));
            }
        };

        let port = port_str
            .parse::<u16>()
            .context("PORT must be a valid u16 integer")?;

        Ok(Self {
            database_url,
            jwt_secret,
            run_migrations,
            port,
        })
    }
}
