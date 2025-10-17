/// Holds the raw data for a queried table: column names and rows.
/// All cell values are stored as Strings for simple rendering in Phase 1.
pub struct TableData {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Global application state passed down to UI components.
pub struct AppState {
    pub db_path: Option<String>,
    pub tables: Vec<String>,
    pub active_table: Option<String>,
    pub table_data: Option<TableData>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            db_path: None,
            tables: Vec::new(),
            active_table: None,
            table_data: None,
        }
    }
}
