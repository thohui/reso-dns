use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct PagedQuery {
    skip: Option<u64>,
    top: Option<u64>,
}

impl PagedQuery {
    pub fn skip(&self) -> u64 {
        self.skip.unwrap_or(0)
    }
    pub fn top(&self) -> u64 {
        const MAX_TOP: u64 = 1000;
        self.top.unwrap_or(25).min(MAX_TOP)
    }
}

#[derive(Serialize, Debug)]
pub struct PagedResponse<T: Serialize> {
    pub items: Vec<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    pub top: u64,
    pub skip: u64,
    pub has_more: bool,
    pub next_offset: u64,
}

impl<T: Serialize> PagedResponse<T> {
    pub fn new(items: Vec<T>, total: Option<u64>, top: u64, skip: u64) -> Self {
        let next_offset = skip.saturating_add(items.len() as u64);
        let has_more = match total {
            Some(t) => next_offset < t,
            None => items.len() as u64 == top,
        };

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
