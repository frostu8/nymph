//! API routes.

use std::cmp::{max, min};

use crate::app::AppError;
use crate::request::validate::{Validator as _, ValidatorExt as _, value};

pub mod card;
pub mod user;

/// Pagination helper.
pub struct Pagination<T> {
    results: Vec<T>,
    limit: u32,
}

impl<T> Pagination<T> {
    /// Creates a new pagination with a count limit.
    pub fn new(results: impl Into<Vec<T>>) -> Pagination<T> {
        Pagination {
            results: results.into(),
            limit: 25,
        }
    }

    /// Changes the limit of the pagination.
    ///
    /// By default, it is `25`.
    pub fn limit(self, limit: u32) -> Pagination<T> {
        Pagination { limit, ..self }
    }

    /// Paginates the results.
    pub fn paginate(&self, page: u32, count: u32) -> Result<&[T], AppError> {
        // limit results
        let count = value("count", count as usize)
            .in_range(1..=(self.limit as usize))
            .validate()?;

        let max_page = self.results.len() / count;
        let page = value("page", page as usize)
            .in_range(1..=max(max_page, 1))
            .validate()?;

        if self.results.len() > 0 {
            let start = (page - 1) * count;
            let end = min(self.results.len(), start + count);

            Ok(&self.results[start..end])
        } else {
            Ok(&[])
        }
    }
}
