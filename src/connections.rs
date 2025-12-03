use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedConnection {
    pub name: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub database: String,
    // Password is intentionally omitted from persistence for security.
}

pub struct ConnectionStore {
    connections: Vec<SavedConnection>,
}

impl ConnectionStore {
    pub fn load() -> Self {
        let path = config_path();
        let connections = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(_) => vec![],
        };
        Self { connections }
    }

    pub fn connections(&self) -> &[SavedConnection] {
        &self.connections
    }

    /// Upserts a connection by name, then persists the full list.
    pub fn upsert(&mut self, conn: SavedConnection) {
        if let Some(existing) = self.connections.iter_mut().find(|c| c.name == conn.name) {
            *existing = conn;
        } else {
            self.connections.push(conn);
        }
        self.persist();
    }

    /// Removes the connection at the given index and persists.
    pub fn remove(&mut self, index: usize) {
        if index < self.connections.len() {
            self.connections.remove(index);
            self.persist();
        }
    }

    fn persist(&self) {
        let path = config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.connections) {
            let _ = std::fs::write(&path, json);
        }
    }
}

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".db-client").join("connections.json")
}
