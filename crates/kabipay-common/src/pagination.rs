//! Reusable pagination types for GraphQL list queries.
//!
//! Every list query follows the Connection pattern: nodes + pageInfo + totalCount.
//! Clamped at `MAX_PER_PAGE = 100` to protect the DB from runaway queries.

use async_graphql::{InputObject, SimpleObject};

pub const MAX_PER_PAGE: u64 = 100;
pub const DEFAULT_PER_PAGE: u64 = 25;

/// Page + per_page input. Clamp with `clamp()` before use.
#[derive(Debug, Clone, Copy, InputObject)]
pub struct PageInput {
    #[graphql(default = 1)]
    pub page: u64,
    #[graphql(default = 25)]
    pub per_page: u64,
}

impl Default for PageInput {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: DEFAULT_PER_PAGE,
        }
    }
}

impl PageInput {
    /// Returns a normalised PageInput: page ≥ 1, 1 ≤ per_page ≤ MAX_PER_PAGE.
    pub fn clamp(self) -> Self {
        Self {
            page: self.page.max(1),
            per_page: self.per_page.clamp(1, MAX_PER_PAGE),
        }
    }

    pub fn offset(&self) -> u64 {
        let c = self.clamp();
        (c.page - 1) * c.per_page
    }

    pub fn limit(&self) -> u64 {
        self.clamp().per_page
    }
}

#[derive(Debug, Clone, Copy, SimpleObject)]
pub struct PageInfo {
    pub total_count: u64,
    pub total_pages: u64,
    pub current_page: u64,
    pub per_page: u64,
    pub has_next_page: bool,
    pub has_prev_page: bool,
}

impl PageInfo {
    pub fn compute(page_input: PageInput, total_count: u64) -> Self {
        let c = page_input.clamp();
        let total_pages = if c.per_page == 0 {
            0
        } else {
            total_count.div_ceil(c.per_page)
        };
        Self {
            total_count,
            total_pages,
            current_page: c.page,
            per_page: c.per_page,
            has_next_page: c.page < total_pages,
            has_prev_page: c.page > 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_per_page() {
        let p = PageInput {
            page: 0,
            per_page: 9999,
        }
        .clamp();
        assert_eq!(p.page, 1);
        assert_eq!(p.per_page, MAX_PER_PAGE);
    }

    #[test]
    fn computes_page_info() {
        let info = PageInfo::compute(
            PageInput {
                page: 2,
                per_page: 25,
            },
            120,
        );
        assert_eq!(info.total_pages, 5);
        assert_eq!(info.current_page, 2);
        assert!(info.has_next_page);
        assert!(info.has_prev_page);
    }
}
