pub mod activity_log;
pub mod blocked_domain;
pub mod config;
pub mod error_log;
pub mod local_record;
pub mod query_log;
pub mod user;
pub mod user_session;

pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
}
