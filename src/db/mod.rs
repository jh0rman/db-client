pub mod postgres;
pub mod sqlite;

// ─── Shared types ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct DbError(pub String);

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<rusqlite::Error> for DbError {
    fn from(e: rusqlite::Error) -> Self {
        DbError(e.to_string())
    }
}

// ─── Driver trait ─────────────────────────────────────────────────────────────

/// Common interface implemented by every database backend.
pub trait DbDriver: Send + Sync {
    fn get_tables(&self) -> Result<Vec<String>, DbError>;
    /// Execute an arbitrary query and return all matching rows.
    fn execute_query(&self, query: &str) -> Result<crate::state::TableData, DbError>;
    /// Wrap `query` in LIMIT/OFFSET for lazy pagination. Implementations
    /// use a subquery so the user's own ORDER BY / WHERE clauses are preserved.
    fn execute_query_paged(
        &self,
        query: &str,
        offset: usize,
        limit: usize,
    ) -> Result<crate::state::TableData, DbError>;
}

// ─── Async blocking helper ────────────────────────────────────────────────────

/// Runs a blocking closure on a dedicated thread and returns its result via a
/// oneshot channel. Returns `None` if the channel was dropped before resolution.
pub async fn run_blocking<T: Send + 'static>(
    f: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (tx, rx) = futures::channel::oneshot::channel::<T>();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.await.ok()
}

// ─── Mock driver (tests only) ─────────────────────────────────────────────────

#[cfg(test)]
pub struct MockDriver;

#[cfg(test)]
impl DbDriver for MockDriver {
    fn get_tables(&self) -> Result<Vec<String>, DbError> {
        Ok(vec![
            "mock_orders".to_string(),
            "mock_products".to_string(),
            "mock_users".to_string(),
        ])
    }

    fn execute_query(&self, _query: &str) -> Result<crate::state::TableData, DbError> {
        use std::sync::Arc;
        use crate::state::TableData;
        Ok(TableData {
            columns: vec!["id".to_string(), "name".to_string(), "status".to_string()],
            column_types: vec!["int4".to_string(), "text".to_string(), "text".to_string()],
            rows: Arc::new(vec![
                vec![Some("1".to_string()), Some("Alice".to_string()), Some("active".to_string())],
                vec![Some("2".to_string()), Some("Bob".to_string()), Some("inactive".to_string())],
                vec![Some("3".to_string()), None, Some("active".to_string())],
            ]),
        })
    }

    fn execute_query_paged(
        &self,
        _query: &str,
        _offset: usize,
        _limit: usize,
    ) -> Result<crate::state::TableData, DbError> {
        use std::sync::Arc;
        use crate::state::TableData;
        Ok(TableData { columns: vec![], column_types: vec![], rows: Arc::new(vec![]) })
    }
}
