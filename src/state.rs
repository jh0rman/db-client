use std::sync::Arc;
use crate::db::DbDriver;

// ─── Table data ───────────────────────────────────────────────────────────────

/// Flat result returned by the driver for a single query or page.
pub struct TableData {
    pub columns: Vec<String>,
    /// Postgres/SQLite type name per column (e.g. "int4", "text", "bool").
    pub column_types: Vec<String>,
    /// None = SQL NULL; Some(s) = cell value.
    pub rows: Arc<Vec<Vec<Option<String>>>>,
}

// ─── Paged result (used by the UI) ───────────────────────────────────────────

/// All rows loaded so far for the active query, growing as the user scrolls.
pub struct PagedTableData {
    pub columns: Vec<String>,
    pub column_types: Vec<String>,
    /// The bare query (no LIMIT/OFFSET) used to fetch subsequent chunks.
    pub query: String,
    /// Accumulated rows across all loaded chunks. None = SQL NULL.
    pub rows: Arc<Vec<Vec<Option<String>>>>,
    pub chunk_size: usize,
    /// True when the last chunk came back with fewer rows than chunk_size.
    pub all_loaded: bool,
    /// True while a background chunk fetch is in flight.
    pub loading_next: bool,
}

// ─── Connection ───────────────────────────────────────────────────────────────

#[derive(PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

pub struct ConnectionConfig {
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub database: String,
}

// ─── App state ────────────────────────────────────────────────────────────────

pub struct AppState {
    pub connection_status: ConnectionStatus,
    pub driver: Option<Arc<dyn DbDriver>>,
    pub sql_query: String,
    // Main view
    pub tables: Vec<String>,
    pub active_table: Option<String>,
    pub results: Option<PagedTableData>,
    pub query_error: Option<String>,
    pub connection_error: Option<String>,
    pub query_in_progress: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            connection_status: ConnectionStatus::Disconnected,
            driver: None,
            sql_query: String::new(),
            tables: Vec::new(),
            active_table: None,
            results: None,
            query_error: None,
            connection_error: None,
            query_in_progress: false,
        }
    }
}
