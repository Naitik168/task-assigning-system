//! Email delivery abstraction. The development implementation records every
//! message in the `email_logs` table and prints it to the console; a real SMTP
//! sender could implement the same trait without touching the auth service.

use async_trait::async_trait;
use sqlx::PgPool;

use crate::{error::AppResult, repositories::email_logs};

#[derive(Debug, Clone)]
pub struct OutgoingEmail {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[async_trait]
pub trait EmailSender: Send + Sync {
    async fn send(&self, email: OutgoingEmail) -> AppResult<()>;
}

pub struct DevEmailLogger {
    db: PgPool,
}

impl DevEmailLogger {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl EmailSender for DevEmailLogger {
    async fn send(&self, email: OutgoingEmail) -> AppResult<()> {
        email_logs::insert(&self.db, &email.to, &email.subject, &email.body).await?;
        tracing::info!(
            to = %email.to,
            subject = %email.subject,
            "[dev-email] {}",
            email.body
        );
        Ok(())
    }
}
