//! Application use-cases live here.
//! Domain rules stay independent from Axum, SQLx, Tauri, and UI code.

pub mod r2;
pub mod storage;

pub use school_collect_domain::{CollectId, TenantId, UserId};
