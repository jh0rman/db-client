use std::sync::{Arc, Mutex};
use postgres::{Client, NoTls};

use super::{DbDriver, DbError};
use crate::state::TableData;

pub struct PostgresDriver {
    client: Mutex<Client>,
}

impl PostgresDriver {
    /// Opens a synchronous connection. Call from a background thread.
    pub fn connect(
        host: &str,
        port: &str,
        user: &str,
        password: &str,
        database: &str,
    ) -> Result<Self, DbError> {
        let port_u16: u16 = port.parse().unwrap_or(5432);

        let mut config = postgres::Config::new();
        config.host(host).port(port_u16).user(user).dbname(database);
        if !password.is_empty() {
            config.password(password);
        }

        let client = config.connect(NoTls).map_err(|e| DbError(e.to_string()))?;
        Ok(Self {
            client: Mutex::new(client),
        })
    }

    /// Shared inner query runner used by both public methods.
    fn run_query(&self, sql: &str) -> Result<TableData, DbError> {
        let mut client = self.client.lock().unwrap();

        // prepare() gives us column names + types without executing the query.
        let stmt = client.prepare(sql).map_err(|e| DbError(e.to_string()))?;
        let columns: Vec<String> = stmt.columns().iter().map(|c| c.name().to_string()).collect();
        let column_types: Vec<String> =
            stmt.columns().iter().map(|c| c.type_().name().to_string()).collect();

        // simple_query returns every value as Option<&str> — works for all Postgres types.
        let messages = client.simple_query(sql).map_err(|e| DbError(e.to_string()))?;
        let col_count = columns.len();
        let data_rows: Vec<Vec<Option<String>>> = messages
            .into_iter()
            .filter_map(|msg| {
                if let postgres::SimpleQueryMessage::Row(row) = msg {
                    Some((0..col_count).map(|i| row.get(i).map(str::to_string)).collect())
                } else {
                    None
                }
            })
            .collect();

        Ok(TableData {
            columns,
            column_types,
            rows: Arc::new(data_rows),
        })
    }
}

impl DbDriver for PostgresDriver {
    fn get_tables(&self) -> Result<Vec<String>, DbError> {
        let mut client = self.client.lock().unwrap();
        let rows = client
            .query(
                "SELECT table_schema, table_name \
                 FROM information_schema.tables \
                 WHERE table_schema NOT IN ('pg_catalog','information_schema','pg_toast') \
                 AND table_type = 'BASE TABLE' \
                 ORDER BY table_schema, table_name",
                &[],
            )
            .map_err(|e| DbError(e.to_string()))?;
        let tables: Vec<String> = rows
            .iter()
            .map(|r| {
                let schema: String = r.get(0);
                let table: String = r.get(1);
                if schema == "public" {
                    table
                } else {
                    format!("{schema}.{table}")
                }
            })
            .collect();
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
        // Wrap the user query as a subquery so their ORDER BY / WHERE are preserved,
        // then apply our pagination envelope on top.
        let sql = format!(
            "SELECT * FROM ({query}) AS _paged_q LIMIT {limit} OFFSET {offset}"
        );
        self.run_query(&sql)
    }
}

