//! Task Manager API library crate.
//!
//! Layering: `routes` (HTTP) -> `services` (business rules) -> `repositories` (SQL).
//! `main.rs` only wires configuration, infrastructure and the router together,
//! which lets integration tests build the exact same router in-process.

pub mod config;
pub mod dto;
pub mod error;
pub mod extract;
pub mod middleware;
pub mod models;
pub mod openapi;
pub mod repositories;
pub mod routes;
pub mod services;
pub mod state;

pub use routes::build_router;
pub use state::AppState;
