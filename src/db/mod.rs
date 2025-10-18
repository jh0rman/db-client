use rusqlite::{Connection, Result};
use crate::state::TableData;

/// Creates a sample SQLite database at `db_path` with test tables if it doesn't exist.
pub fn seed_test_db(db_path: &str) -> Result<()> {
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

/// Returns the names of all tables and views in the database, sorted alphabetically.
pub fn get_tables(db_path: &str) -> Result<Vec<String>> {
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type IN ('table','view') ORDER BY name",
    )?;
    let tables = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>>>()?;
    Ok(tables)
}

/// Phase 3: Returns the first 100 rows of `table` as strings.
pub fn get_table_data(_db_path: &str, _table: &str) -> Result<TableData> {
    unimplemented!("Phase 3")
}
