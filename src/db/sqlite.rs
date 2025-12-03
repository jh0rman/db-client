use std::sync::Mutex;
use rusqlite::Connection;
use super::{DbDriver, DbError};
use crate::state::TableData;

pub struct SqliteDriver {
    conn: Mutex<Connection>,
}

impl SqliteDriver {
    pub fn connect(path: &str) -> Result<Self, DbError> {
        let conn = Connection::open(path).map_err(DbError::from)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    fn run_query(&self, sql: &str) -> Result<TableData, DbError> {
        use std::sync::Arc;
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql).map_err(DbError::from)?;

        let col_count = stmt.column_count();
        let columns: Vec<String> = stmt.column_names().into_iter().map(str::to_string).collect();
        let column_types: Vec<String> = vec![String::new(); col_count];

        let rows = stmt
            .query_map([], |row| {
                let cells = (0..col_count)
                    .map(|i| {
                        use rusqlite::types::Value;
                        let val: Value = row.get(i)?;
                        Ok(match val {
                            Value::Null => None,
                            Value::Integer(n) => Some(n.to_string()),
                            Value::Real(f) => Some(format!("{:.4}", f)),
                            Value::Text(s) => Some(s),
                            Value::Blob(b) => Some(format!("<blob {} bytes>", b.len())),
                        })
                    })
                    .collect::<rusqlite::Result<Vec<Option<String>>>>()?;
                Ok(cells)
            })
            .map_err(DbError::from)?
            .collect::<Result<Vec<Vec<Option<String>>>, _>>()
            .map_err(DbError::from)?;

        Ok(TableData { columns, column_types, rows: Arc::new(rows) })
    }
}

impl DbDriver for SqliteDriver {
    fn get_tables(&self) -> Result<Vec<String>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type IN ('table','view') ORDER BY name",
            )
            .map_err(DbError::from)?;
        let tables = stmt
            .query_map([], |row| row.get(0))
            .map_err(DbError::from)?
            .collect::<Result<Vec<String>, _>>()
            .map_err(DbError::from)?;
        Ok(tables)
    }

    fn execute_query(&self, query: &str) -> Result<TableData, DbError> {
        self.run_query(query)
    }

    fn execute_query_paged(
        &self,
        query: &str,
        offset: usize,
        limit: usize,
    ) -> Result<TableData, DbError> {
        let sql = format!("SELECT * FROM ({query}) AS _q LIMIT {limit} OFFSET {offset}");
        self.run_query(&sql)
    }
}

pub fn seed_test_db(path: &str) -> Result<(), DbError> {
    let conn = Connection::open(path).map_err(DbError::from)?;
    conn.execute_batch(
        "
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
    ",
    )
    .map_err(DbError::from)?;
    Ok(())
}
