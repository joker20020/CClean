//! SQLite 持久化：磁盘快照、快照分项、文件系统事件、清理日志、设置。
//! 数据库位于应用数据目录 cclean.db。

use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("数据库错误: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("数据库错误: {0}")]
    Msg(String),
}

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub id: i64,
    pub created_at: i64,
    pub disk_name: String,
    pub disk_total: i64,
    pub disk_used: i64,
    pub disk_free: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SnapshotItem {
    pub snapshot_id: i64,
    pub rule_id: String,
    pub size_bytes: i64,
    pub file_count: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SnapshotInputItem {
    pub rule_id: String,
    pub size_bytes: i64,
    pub file_count: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct TrendData {
    pub snapshots: Vec<Snapshot>,
    pub items: Vec<SnapshotItem>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CleanLog {
    pub id: i64,
    pub ts: i64,
    pub rule_id: String,
    pub path: String,
    pub size_bytes: i64,
    pub status: String,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct FsEvent {
    pub id: i64,
    pub ts: i64,
    pub kind: String,
    pub rule_id: String,
    pub path: String,
    pub size_delta: Option<i64>,
}

pub struct Storage {
    conn: Mutex<Connection>,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at INTEGER NOT NULL,
                disk_name TEXT NOT NULL,
                disk_total INTEGER NOT NULL,
                disk_used INTEGER NOT NULL,
                disk_free INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS snapshot_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                rule_id TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                file_count INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_snapshot_items_sid ON snapshot_items(snapshot_id);
            CREATE TABLE IF NOT EXISTS fs_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts INTEGER NOT NULL,
                kind TEXT NOT NULL,
                rule_id TEXT NOT NULL,
                path TEXT NOT NULL,
                size_delta INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_fs_events_ts ON fs_events(ts);
            CREATE TABLE IF NOT EXISTS clean_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts INTEGER NOT NULL,
                rule_id TEXT NOT NULL,
                path TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                status TEXT NOT NULL,
                detail TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_clean_logs_ts ON clean_logs(ts);
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 写入一次快照（磁盘 + 各规则占用），返回快照 id。
    pub fn insert_snapshot(
        &self,
        disk_name: &str,
        disk_total: u64,
        disk_used: u64,
        disk_free: u64,
        items: &[SnapshotInputItem],
    ) -> Result<i64> {
        let conn = self.conn.lock().map_err(|e| StorageError::Msg(e.to_string()))?;
        let ts = Utc::now().timestamp();
        conn.execute(
            "INSERT INTO snapshots (created_at, disk_name, disk_total, disk_used, disk_free)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![ts, disk_name, disk_total as i64, disk_used as i64, disk_free as i64],
        )?;
        let id = conn.last_insert_rowid();
        {
            let mut stmt = conn.prepare(
                "INSERT INTO snapshot_items (snapshot_id, rule_id, size_bytes, file_count)
                 VALUES (?1, ?2, ?3, ?4)",
            )?;
            for it in items {
                stmt.execute(rusqlite::params![id, it.rule_id, it.size_bytes, it.file_count])?;
            }
        }
        Ok(id)
    }

    pub fn list_snapshots(&self, limit: u32) -> Result<Vec<Snapshot>> {
        let conn = self.conn.lock().map_err(|e| StorageError::Msg(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, created_at, disk_name, disk_total, disk_used, disk_free
             FROM snapshots ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                Ok(Snapshot {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    disk_name: row.get(2)?,
                    disk_total: row.get(3)?,
                    disk_used: row.get(4)?,
                    disk_free: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        // 时间正序返回，便于画趋势线
        Ok(rows.into_iter().rev().collect())
    }

    /// 返回最近 limit 个快照及其分项。
    pub fn trend(&self, limit: u32) -> Result<TrendData> {
        let snapshots = self.list_snapshots(limit)?;
        let conn = self.conn.lock().map_err(|e| StorageError::Msg(e.to_string()))?;
        let ids: Vec<i64> = snapshots.iter().map(|s| s.id).collect();
        let mut items = Vec::new();
        if !ids.is_empty() {
            let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let sql = format!(
                "SELECT snapshot_id, rule_id, size_bytes, file_count
                 FROM snapshot_items WHERE snapshot_id IN ({placeholders})
                 ORDER BY snapshot_id"
            );
            let mut stmt = conn.prepare(&sql)?;
            let params: Vec<&dyn rusqlite::ToSql> =
                ids.iter().map(|i| i as &dyn rusqlite::ToSql).collect();
            let rows = stmt
                .query_map(params.as_slice(), |row| {
                    Ok(SnapshotItem {
                        snapshot_id: row.get(0)?,
                        rule_id: row.get(1)?,
                        size_bytes: row.get(2)?,
                        file_count: row.get(3)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            items = rows;
        }
        Ok(TrendData { snapshots, items })
    }

    pub fn insert_clean_log(
        &self,
        rule_id: &str,
        path: &str,
        size_bytes: i64,
        status: &str,
        detail: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| StorageError::Msg(e.to_string()))?;
        conn.execute(
            "INSERT INTO clean_logs (ts, rule_id, path, size_bytes, status, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                Utc::now().timestamp(),
                rule_id,
                path,
                size_bytes,
                status,
                detail
            ],
        )?;
        Ok(())
    }

    pub fn clean_logs(&self, limit: u32) -> Result<Vec<CleanLog>> {
        let conn = self.conn.lock().map_err(|e| StorageError::Msg(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, ts, rule_id, path, size_bytes, status, detail
             FROM clean_logs ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                Ok(CleanLog {
                    id: row.get(0)?,
                    ts: row.get(1)?,
                    rule_id: row.get(2)?,
                    path: row.get(3)?,
                    size_bytes: row.get(4)?,
                    status: row.get(5)?,
                    detail: row.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn insert_fs_event(
        &self,
        kind: &str,
        rule_id: &str,
        path: &str,
        size_delta: Option<i64>,
    ) -> Result<i64> {
        let conn = self.conn.lock().map_err(|e| StorageError::Msg(e.to_string()))?;
        conn.execute(
            "INSERT INTO fs_events (ts, kind, rule_id, path, size_delta)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![Utc::now().timestamp(), kind, rule_id, path, size_delta],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn fs_events(&self, limit: u32) -> Result<Vec<FsEvent>> {
        let conn = self.conn.lock().map_err(|e| StorageError::Msg(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, ts, kind, rule_id, path, size_delta
             FROM fs_events ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                Ok(FsEvent {
                    id: row.get(0)?,
                    ts: row.get(1)?,
                    kind: row.get(2)?,
                    rule_id: row.get(3)?,
                    path: row.get(4)?,
                    size_delta: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn delete_clean_log(&self, id: i64) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| StorageError::Msg(e.to_string()))?;
        Ok(conn.execute("DELETE FROM clean_logs WHERE id = ?1", rusqlite::params![id])?)
    }

    pub fn clear_clean_logs(&self) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| StorageError::Msg(e.to_string()))?;
        Ok(conn.execute("DELETE FROM clean_logs", [])?)
    }

    pub fn delete_fs_event(&self, id: i64) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| StorageError::Msg(e.to_string()))?;
        Ok(conn.execute("DELETE FROM fs_events WHERE id = ?1", rusqlite::params![id])?)
    }

    pub fn clear_fs_events(&self) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| StorageError::Msg(e.to_string()))?;
        Ok(conn.execute("DELETE FROM fs_events", [])?)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().map_err(|e| StorageError::Msg(e.to_string()))?;
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query_map(rusqlite::params![key], |row| row.get::<_, String>(0))?;
        match rows.next() {
            Some(v) => Ok(Some(v?)),
            None => Ok(None),
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| StorageError::Msg(e.to_string()))?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db(name: &str) -> Storage {
        let dir = std::env::temp_dir().join(format!("cclean_test_{}_{}", std::process::id(), name));
        std::fs::create_dir_all(&dir).unwrap();
        Storage::open(&dir.join("test.db")).unwrap()
    }

    #[test]
    fn snapshot_roundtrip_and_trend() {
        let s = temp_db("snap");
        let id1 = s
            .insert_snapshot(
                "C:\\",
                1000,
                500,
                500,
                &[
                    SnapshotInputItem {
                        rule_id: "dev.npm".into(),
                        size_bytes: 100,
                        file_count: 10,
                    },
                    SnapshotInputItem {
                        rule_id: "gpu.nvidia".into(),
                        size_bytes: 200,
                        file_count: 5,
                    },
                ],
            )
            .unwrap();
        let id2 = s
            .insert_snapshot(
                "C:\\",
                1000,
                600,
                400,
                &[SnapshotInputItem {
                    rule_id: "dev.npm".into(),
                    size_bytes: 150,
                    file_count: 12,
                }],
            )
            .unwrap();
        assert!(id2 > id1);

        let snaps = s.list_snapshots(10).unwrap();
        assert_eq!(snaps.len(), 2);
        assert_eq!(snaps[0].disk_free, 500); // 时间正序
        let trend = s.trend(10).unwrap();
        assert_eq!(trend.snapshots.len(), 2);
        assert_eq!(trend.items.len(), 3);
    }

    #[test]
    fn logs_events_and_settings() {
        let s = temp_db("logs");
        s.insert_clean_log("dev.npm", "/tmp/x", 123, "ok", "").unwrap();
        assert_eq!(s.clean_logs(10).unwrap().len(), 1);

        let id = s.insert_fs_event("write", "dev.npm", "/tmp/x/y", Some(10)).unwrap();
        let evs = s.fs_events(10).unwrap();
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].id, id);

        assert_eq!(s.get_setting("missing").unwrap(), None);
        s.set_setting("interval_hours", "6").unwrap();
        assert_eq!(s.get_setting("interval_hours").unwrap().as_deref(), Some("6"));
        s.set_setting("interval_hours", "12").unwrap();
        assert_eq!(s.get_setting("interval_hours").unwrap().as_deref(), Some("12"));
    }
}
