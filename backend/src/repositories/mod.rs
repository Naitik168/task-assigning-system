//! Data-access layer. Functions accept any `PgExecutor` so they work with a
//! pool or inside a transaction (`&mut *tx`).

pub mod challenges;
pub mod email_logs;
pub mod tasks;
pub mod users;
