use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct PagedQuery {
    skip: Option<usize>,
    top: Option<usize>,
}

impl PagedQuery {
    pub fn skip(&self) -> usize {
        self.skip.unwrap_or(0)
    }
    pub fn top(&self) -> usize {
        self.top.unwrap_or(25)
    }
}

#[derive(Serialize, Debug)]
pub struct PagedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total: usize,
    pub top: usize,
    pub skip: usize,
    pub has_more: bool,
    pub next_offset: usize,
}

impl<T: Serialize> PagedResponse<T> {
    pub fn new(items: Vec<T>, total: usize, top: usize, skip: usize) -> Self {
        let next_offset = skip.saturating_add(items.len());
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
