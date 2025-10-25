pub mod postgres;

use rusqlite::Connection;

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
    fn execute_query(&self, query: &str) -> Result<crate::state::TableData, DbError>;
}

// ─── Mock driver (Phase 1) ────────────────────────────────────────────────────

pub struct MockDriver;

impl DbDriver for MockDriver {
    fn get_tables(&self) -> Result<Vec<String>, DbError> {
        Ok(vec![
            "mock_orders".to_string(),
            "mock_products".to_string(),
            "mock_users".to_string(),
        ])
    }

    fn execute_query(&self, _query: &str) -> Result<crate::state::TableData, DbError> {
        use crate::state::TableData;
        Ok(TableData {
            columns: vec!["id".to_string(), "name".to_string(), "status".to_string()],
            rows: vec![
                vec!["1".to_string(), "Alice".to_string(), "active".to_string()],
                vec!["2".to_string(), "Bob".to_string(), "inactive".to_string()],
                vec!["3".to_string(), "Carol".to_string(), "active".to_string()],
            ],
        })
    }
}

// ─── SQLite helpers (kept from v1, Phase 2 will wrap in SqliteDriver) ─────────

pub fn seed_test_db(db_path: &str) -> Result<(), rusqlite::Error> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS users (
            id      INTEGER PRIMARY KEY,
            name    TEXT NOT NULL,
            email   TEXT NOT NULL,
            created TEXT
        );
        CREATE TABLE IF NOT EXISTS products (
            id          INTEGER PRIMARY KEY,
            name        TEXT NOT NULL,
            price       REAL,
            stock       INTEGER
        );
        CREATE TABLE IF NOT EXISTS orders (
            id          INTEGER PRIMARY KEY,
            user_id     INTEGER,
            product_id  INTEGER,
            quantity    INTEGER,
            total       REAL
        );
        INSERT OR IGNORE INTO users VALUES (1,'Alice','alice@example.com','2024-01-01');
        INSERT OR IGNORE INTO users VALUES (2,'Bob','bob@example.com','2024-02-15');
        INSERT OR IGNORE INTO products VALUES (1,'Widget',9.99,100);
        INSERT OR IGNORE INTO products VALUES (2,'Gadget',49.99,25);
        INSERT OR IGNORE INTO orders VALUES (1,1,2,1,49.99);
        INSERT OR IGNORE INTO orders VALUES (2,2,1,3,29.97);
    ")?;
    Ok(())
}

pub fn get_tables(db_path: &str) -> Result<Vec<String>, rusqlite::Error> {
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type IN ('table','view') ORDER BY name",
    )?;
    let tables = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    Ok(tables)
}

pub fn get_table_data(db_path: &str, table: &str) -> Result<crate::state::TableData, rusqlite::Error> {
    let conn = Connection::open(db_path)?;
    let query = format!("SELECT * FROM \"{}\" LIMIT 100", table);
    let mut stmt = conn.prepare(&query)?;

    let col_count = stmt.column_count();
    let columns: Vec<String> = stmt.column_names().into_iter().map(str::to_string).collect();

    let rows = stmt
        .query_map([], |row| {
            let cells = (0..col_count)
                .map(|i| {
                    use rusqlite::types::Value;
                    let val: Value = row.get(i)?;
                    Ok(match val {
                        Value::Null => String::new(),
                        Value::Integer(n) => n.to_string(),
                        Value::Real(f) => format!("{:.4}", f),
                        Value::Text(s) => s,
                        Value::Blob(b) => format!("<blob {} bytes>", b.len()),
                    })
                })
                .collect::<rusqlite::Result<Vec<String>>>()?;
            Ok(cells)
        })?
        .collect::<Result<Vec<Vec<String>>, _>>()?;

    Ok(crate::state::TableData { columns, rows })
}
