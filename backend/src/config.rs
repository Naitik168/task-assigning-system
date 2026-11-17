use std::{env, time::Duration};

use anyhow::{bail, Context};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppEnv {
    Development,
    Production,
}

/// Runtime configuration, loaded from environment variables (see `.env.example`).
///
/// `Debug` is intentionally not derived so secrets never end up in logs.
#[derive(Clone)]
pub struct Config {
    pub app_env: AppEnv,
    pub bind_addr: String,
    pub database_url: String,
    /// `None` => use the in-memory cache.
    pub redis_url: Option<String>,
    pub jwt_secret: String,
    pub jwt_ttl: Duration,
    /// Server-side key used to HMAC one-time codes before they are stored.
    pub otp_secret: String,
    pub otp_ttl: Duration,
    pub otp_max_attempts: i32,
    pub cache_ttl: Duration,
    pub cors_origins: Vec<String>,
    pub run_migrations: bool,
    pub seed_admin_password: String,
    pub seed_staff_password: String,
}

const DEV_JWT_SECRET: &str = "dev-only-jwt-secret-change-me-please-0123456789";
const DEV_OTP_SECRET: &str = "dev-only-otp-secret-change-me-please-0123456789";

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let app_env = match var_or("APP_ENV", "development").to_lowercase().as_str() {
            "development" | "dev" | "local" => AppEnv::Development,
            "production" | "prod" => AppEnv::Production,
            other => bail!("APP_ENV must be 'development' or 'production', got '{other}'"),
        };

        let config = Self {
            app_env,
            bind_addr: var_or("BIND_ADDR", "0.0.0.0:8080"),
            database_url: env::var("DATABASE_URL").context("DATABASE_URL must be set")?,
            redis_url: env::var("REDIS_URL").ok().filter(|v| !v.trim().is_empty()),
            jwt_secret: var_or("JWT_SECRET", DEV_JWT_SECRET),
            jwt_ttl: Duration::from_secs(parse_var("JWT_TTL_MINUTES", 60u64)? * 60),
            otp_secret: var_or("OTP_SECRET", DEV_OTP_SECRET),
            otp_ttl: Duration::from_secs(parse_var("OTP_TTL_SECONDS", 300u64)?),
            otp_max_attempts: parse_var("OTP_MAX_ATTEMPTS", 5i32)?,
            cache_ttl: Duration::from_secs(parse_var("CACHE_TTL_SECONDS", 300u64)?),
            cors_origins: var_or(
                "CORS_ORIGINS",
                "http://localhost:5173,http://localhost:3000",
            )
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
            run_migrations: parse_var("RUN_MIGRATIONS", true)?,
            seed_admin_password: var_or("SEED_ADMIN_PASSWORD", "Admin@12345"),
            seed_staff_password: var_or("SEED_STAFF_PASSWORD", "Bond@007007"),
        };

        if config.app_env == AppEnv::Production {
            if config.jwt_secret == DEV_JWT_SECRET || config.jwt_secret.len() < 32 {
                bail!("JWT_SECRET must be set to a random value of at least 32 characters in production");
            }
            if config.otp_secret == DEV_OTP_SECRET || config.otp_secret.len() < 32 {
                bail!("OTP_SECRET must be set to a random value of at least 32 characters in production");
            }
        }
        Ok(config)
    }

    pub fn is_development(&self) -> bool {
        self.app_env == AppEnv::Development
    }

    /// Deterministic configuration for tests (in-memory cache, dev routes enabled).
    pub fn for_tests(database_url: impl Into<String>) -> Self {
        Self {
            app_env: AppEnv::Development,
            bind_addr: "127.0.0.1:0".into(),
            database_url: database_url.into(),
            redis_url: None,
            jwt_secret: "test-jwt-secret-0123456789-0123456789".into(),
            jwt_ttl: Duration::from_secs(3600),
            otp_secret: "test-otp-secret-0123456789-0123456789".into(),
            otp_ttl: Duration::from_secs(300),
            otp_max_attempts: 5,
            cache_ttl: Duration::from_secs(300),
            cors_origins: vec!["http://localhost:5173".into()],
            run_migrations: false,
            seed_admin_password: "Admin@12345".into(),
            seed_staff_password: "Bond@007007".into(),
        }
    }
}

fn var_or(key: &str, default: &str) -> String {
    env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn parse_var<T>(key: &str, default: T) -> anyhow::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(raw) if !raw.trim().is_empty() => raw
            .trim()
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid value for {key}: {e}")),
        _ => Ok(default),
    }
}
