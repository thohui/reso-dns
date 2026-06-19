pub mod activity_log;
pub mod api_key;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchType {
    /// Matches only this exact domain.
    Exact,
    /// Matches all subdomains but not the domain itself (e.g. `*.example.com`).
    Wildcard,
    /// Matches the domain and all its subdomains (adblock `||domain^` semantics).
    Domain,
}

impl rusqlite::types::ToSql for MatchType {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(match self {
            MatchType::Exact => "exact",
            MatchType::Wildcard => "wildcard",
            MatchType::Domain => "domain",
        }
        .into())
    }
}

impl rusqlite::types::FromSql for MatchType {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str()? {
            "exact" => Ok(MatchType::Exact),
            "wildcard" => Ok(MatchType::Wildcard),
            "domain" => Ok(MatchType::Domain),
            other => Err(rusqlite::types::FromSqlError::Other(
                format!("unknown match type: {other}").into(),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListAction {
    Block,
    Allow,
}

impl From<reso_list::parser::RuleType> for ListAction {
    fn from(value: reso_list::parser::RuleType) -> Self {
        match value {
            reso_list::parser::RuleType::Allow => Self::Allow,
            reso_list::parser::RuleType::Block => Self::Block,
        }
    }
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
