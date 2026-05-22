use crate::error::StorageError;
use crate::storage::DbHandle;
use crate::storage::queries::get_timeline;
use crate::types::{TimelineCursor, TimelineFilter, TimelinePage};

/// Service for timeline queries with cursor-based pagination
pub struct TimelineService {
    db: DbHandle,
}

impl TimelineService {
    pub fn new(db: DbHandle) -> Self {
        Self { db }
    }

    /// Fetch a page of timeline items matching the given filter.
    pub async fn get_page(
        &self,
        filter: TimelineFilter,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<TimelinePage, StorageError> {
        self.db
            .with_reader(|pool| async move {
                get_timeline(&pool, &filter, cursor.as_ref(), limit).await
            })
            .await
    }

    /// Fetch the first page of the timeline (no cursor)
    pub async fn get_first_page(
        &self,
        filter: TimelineFilter,
        limit: usize,
    ) -> Result<TimelinePage, StorageError> {
        self.get_page(filter, None, limit).await
    }

    /// Fetch all unread items count
    pub async fn unread_count(&self) -> Result<i64, StorageError> {
        self.db
            .with_reader(|pool| async move {
                let count: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM item_states WHERE is_read = 0 AND is_hidden = 0",
                )
                .fetch_one(&pool)
                .await
                .map_err(StorageError::Sqlite)?;
                Ok(count)
            })
            .await
    }
}
