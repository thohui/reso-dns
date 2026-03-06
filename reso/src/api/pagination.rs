use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct PagedQuery {
    skip: Option<i64>,
    top: Option<i64>,
}

impl PagedQuery {
    pub fn skip(&self) -> i64 {
        self.skip.unwrap_or(0)
    }
    pub fn top(&self) -> i64 {
        const MAX_TOP: i64 = 1000;
        self.top.unwrap_or(25).min(MAX_TOP)
    }
}

#[derive(Serialize, Debug)]
pub struct PagedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total: i64,
    pub top: i64,
    pub skip: i64,
    pub has_more: bool,
    pub next_offset: i64,
}

impl<T: Serialize> PagedResponse<T> {
    pub fn new(items: Vec<T>, total: i64, top: i64, skip: i64) -> Self {
        let next_offset = skip.saturating_add(items.len() as i64);
        let has_more = next_offset < total;

        Self {
            items,
            total,
            top,
            skip,
            next_offset,
            has_more,
        }
    }
}
