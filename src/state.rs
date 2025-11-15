use std::sync::Arc;
use crate::db::DbDriver;

// ─── Table data ───────────────────────────────────────────────────────────────

pub struct TableData {
    pub columns: Vec<String>,
    /// Wrapped in Arc so the render closure can hold a cheap reference each frame
    /// without cloning the full dataset.
    pub rows: Arc<Vec<Vec<String>>>,
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
    pub table_data: Option<TableData>,
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
            table_data: None,
            query_error: None,
            connection_error: None,
            query_in_progress: false,
        }
    }
}
