use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    config::Config,
    services::{
        cache::Cache,
        mailer::{DevEmailLogger, EmailSender},
    },
};

/// Shared application state, cheap to clone (everything is behind `Arc` / pool handles).
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub cache: Arc<dyn Cache>,
    pub mailer: Arc<dyn EmailSender>,
}

impl AppState {
    /// Builds state with the development mailer (writes to `email_logs` + console).
    pub fn new(db: PgPool, config: Config, cache: Arc<dyn Cache>) -> Self {
        let mailer = Arc::new(DevEmailLogger::new(db.clone()));
        Self {
            db,
            config: Arc::new(config),
            cache,
            mailer,
        }
    }
}
