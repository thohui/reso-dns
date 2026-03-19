pub mod activity_log;
pub mod client_metrics;
pub mod config;
pub mod domain_metrics;
pub mod domain_rule;
pub mod list_subscription;
pub mod local_record;
pub mod user;
pub mod user_session;

pub struct Page<T> {
    pub items: Vec<T>,
    pub total: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListAction {
    Block,
    Allow,
}

impl rusqlite::types::ToSql for ListAction {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(match self {
            ListAction::Block => "block",
            ListAction::Allow => "allow",
        }
        .into())
    }
}

impl rusqlite::types::FromSql for ListAction {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str()? {
            "block" => Ok(ListAction::Block),
            "allow" => Ok(ListAction::Allow),
            other => Err(rusqlite::types::FromSqlError::Other(
                format!("unknown list action: {other}").into(),
            )),
        }
    }
}
