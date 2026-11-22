//! Durable scan/recovery history in SQLite.
//!
//! Every completed scan and recovery is recorded in a real database at
//! `data_local_dir/phoenixrecover/history.db` (override with
//! `PHOENIX_DB` for tests). The UI reads it back through the
//! `scan_history` command — no hardcoded arrays anywhere.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use rusqlite::Connection;
use serde::Serialize;

pub struct Store {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub kind: String,
    pub source: String,
    pub detail: String,
    pub files: i64,
    pub bytes: i64,
    pub at: String,
}

impl Store {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id     INTEGER PRIMARY KEY AUTOINCREMENT,
                kind   TEXT NOT NULL,
                source TEXT NOT NULL,
                detail TEXT NOT NULL,
                files  INTEGER NOT NULL DEFAULT 0,
                bytes  INTEGER NOT NULL DEFAULT 0,
                at     TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn record(&self, kind: &str, source: &str, detail: &str, files: i64, bytes: i64) -> anyhow::Result<()> {
        let conn = self.conn.lock().expect("history db lock");
        conn.execute(
            "INSERT INTO history (kind, source, detail, files, bytes) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![kind, source, detail, files, bytes],
        )?;
        Ok(())
    }

    pub fn list(&self, limit: i64) -> anyhow::Result<Vec<HistoryEntry>> {
        let conn = self.conn.lock().expect("history db lock");
        let mut stmt = conn.prepare(
            "SELECT id, kind, source, detail, files, bytes, at
             FROM history ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![limit], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                kind: row.get(1)?,
                source: row.get(2)?,
                detail: row.get(3)?,
                files: row.get(4)?,
                bytes: row.get(5)?,
                at: row.get(6)?,
            })
        })?;
        Ok(rows.flatten().collect())
    }
}

pub fn db_path() -> PathBuf {
    std::env::var("PHOENIX_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| crate::engine::data_local_dir().join("history.db"))
}

fn global() -> &'static Store {
    static STORE: OnceLock<Store> = OnceLock::new();
    STORE.get_or_init(|| Store::open(&db_path()).expect("open history database"))
}

pub fn record_scan(source: &str, deep: bool, files: usize, bytes: u64) {
    let _ = global().record(
        "scan",
        source,
        if deep { "deep scan" } else { "quick scan" },
        files as i64,
        bytes as i64,
    );
}

pub fn record_recovery(source: &str, recovered: usize, bytes: u64, output_dir: &str) {
    let _ = global().record("recovery", source, output_dir, recovered as i64, bytes as i64);
}

pub fn history(limit: i64) -> Vec<HistoryEntry> {
    global().list(limit).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_lists_scans_and_recoveries() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("history.db");
        let store = Store::open(&db).unwrap();

        store.record("scan", "/dev/sdb", "deep scan", 12, 345_678).unwrap();
        store.record("recovery", "/dev/sdb", "/home/u/out", 5, 123_456).unwrap();

        let all = store.list(10).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].kind, "recovery", "newest first");
        assert_eq!(all[0].files, 5);
        assert_eq!(all[1].kind, "scan");
        assert_eq!(all[1].bytes, 345_678);

        let limited = store.list(1).unwrap();
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].kind, "recovery");
    }
}
