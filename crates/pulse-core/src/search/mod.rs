use crate::error::StorageError;
use crate::storage::DbHandle;
use crate::storage::queries::search_items;
use crate::types::FeedItemView;

const DEFAULT_SEARCH_LIMIT: usize = 50;

/// Service for full-text search over feed items using SQLite FTS5
pub struct SearchService {
    db: DbHandle,
}

impl SearchService {
    pub fn new(db: DbHandle) -> Self {
        Self { db }
    }

    /// Search for items matching the query string using FTS5.
    /// `sort` is `relevance` (rank), `newest`, or `oldest`; anything else falls back to relevance.
    pub async fn search(
        &self,
        query: &str,
        limit: Option<usize>,
        sort: &str,
    ) -> Result<Vec<FeedItemView>, StorageError> {
        let limit = limit.unwrap_or(DEFAULT_SEARCH_LIMIT);
        let query = query.to_string();
        let sort = sort.to_string();

        self.db
            .with_reader(|pool| async move { search_items(&pool, &query, limit, &sort).await })
            .await
    }

    /// Search with a prefix query (appends `*` for autocomplete-style matching)
    pub async fn search_prefix(
        &self,
        prefix: &str,
        limit: Option<usize>,
    ) -> Result<Vec<FeedItemView>, StorageError> {
        let query = format!("{}*", escape_fts5_query(prefix));
        self.search(&query, limit, "relevance").await
    }
}

/// Escape special FTS5 characters in a query string
fn escape_fts5_query(q: &str) -> String {
    let escaped = q.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_fts5() {
        assert_eq!(escape_fts5_query("hello world"), "\"hello world\"");
    }
}
