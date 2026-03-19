use rusqlite::types::Value;

/// Reusable builder for SQL WHERE clauses with parameter binding.
/// Mostly used in complex queries with optional filters.
pub struct WhereBuilder {
    /// Offset for parameter indices.
    param_offset: usize,
    /// Accumulated WHERE clauses (e.g. "AND col = ?1").
    clauses: Vec<String>,
    /// Parameters corresponding to the clauses.
    params: Vec<Value>,
}

impl WhereBuilder {
    pub fn new(param_offset: usize) -> Self {
        Self {
            param_offset,
            clauses: Vec::new(),
            params: Vec::new(),
        }
    }

    fn escape_like(s: &str) -> String {
        s.replace('\\', r"\\").replace('%', r"\%").replace('_', r"\_")
    }

    pub fn like(&mut self, col: &str, val: &str) {
        self.params.push(Value::Text(format!("%{}%", Self::escape_like(val))));
        self.clauses.push(format!(
            "AND {col} LIKE ?{} ESCAPE '\\'",
            self.param_offset + self.params.len(),
        ));
    }

    pub fn eq(&mut self, col: &str, val: Value) {
        self.params.push(val);
        self.clauses
            .push(format!("AND {col} = ?{}", self.param_offset + self.params.len(),));
    }

    pub fn raw(&mut self, clause: &str) {
        self.clauses.push(clause.to_string());
    }

    pub fn build(self) -> (String, Vec<Value>) {
        (self.clauses.join(" "), self.params)
    }
}
