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
        let rows = client
            .query(sql, &[])
            .map_err(|e| DbError(e.to_string()))?;

        if rows.is_empty() {
            return Ok(TableData {
                columns: vec![],
                rows: Arc::new(vec![]),
            });
        }

        let columns: Vec<String> = rows[0]
            .columns()
            .iter()
            .map(|c| c.name().to_string())
            .collect();

        let data_rows: Vec<Vec<String>> = rows
            .iter()
            .map(|row| (0..columns.len()).map(|i| pg_value_to_string(row, i)).collect())
            .collect();

        Ok(TableData {
            columns,
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

/// Converts a single Postgres cell to a display string, handling common OIDs.
fn pg_value_to_string(row: &postgres::Row, i: usize) -> String {
    let type_name = row.columns()[i].type_().name();
    match type_name {
        "bool" => row
            .try_get::<_, Option<bool>>(i)
            .ok()
            .flatten()
            .map(|v| v.to_string())
            .unwrap_or_default(),
        "int2" => row
            .try_get::<_, Option<i16>>(i)
            .ok()
            .flatten()
            .map(|v| v.to_string())
            .unwrap_or_default(),
        "int4" | "oid" => row
            .try_get::<_, Option<i32>>(i)
            .ok()
            .flatten()
            .map(|v| v.to_string())
            .unwrap_or_default(),
        "int8" => row
            .try_get::<_, Option<i64>>(i)
            .ok()
            .flatten()
            .map(|v| v.to_string())
            .unwrap_or_default(),
        "float4" => row
            .try_get::<_, Option<f32>>(i)
            .ok()
            .flatten()
            .map(|v| format!("{v}"))
            .unwrap_or_default(),
        "float8" => row
            .try_get::<_, Option<f64>>(i)
            .ok()
            .flatten()
            .map(|v| format!("{v}"))
            .unwrap_or_default(),
        // text, varchar, bpchar, name, uuid, numeric (as text), json, etc.
        _ => row
            .try_get::<_, Option<String>>(i)
            .ok()
            .flatten()
            .unwrap_or_default(),
    }
}
