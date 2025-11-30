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

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".db-client").join("connections.json")
}

pub fn load() -> Vec<SavedConnection> {
    let path = config_path();
    let Ok(bytes) = std::fs::read(&path) else {
        return vec![];
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

pub fn save(connections: &[SavedConnection]) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(connections) {
        let _ = std::fs::write(&path, json);
    }
}

/// Upserts a connection by name, then persists the full list.
pub fn upsert(connections: &mut Vec<SavedConnection>, conn: SavedConnection) {
    if let Some(existing) = connections.iter_mut().find(|c| c.name == conn.name) {
        *existing = conn;
    } else {
        connections.push(conn);
    }
    save(connections);
}

/// Removes the connection at the given index and persists.
pub fn remove(connections: &mut Vec<SavedConnection>, index: usize) {
    if index < connections.len() {
        connections.remove(index);
        save(connections);
    }
}
