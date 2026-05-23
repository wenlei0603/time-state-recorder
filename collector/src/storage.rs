use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use uuid::Uuid;

use crate::models::{CaptureStatus, StoredWindowEvent, WindowSnapshot};

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Self { conn })
    }

    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Self { conn })
    }

    pub fn init(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS capture_sessions (
              id TEXT PRIMARY KEY,
              started_at TEXT NOT NULL,
              ended_at TEXT,
              host_id TEXT NOT NULL,
              app_version TEXT NOT NULL,
              config_hash TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS raw_events (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              session_id TEXT NOT NULL,
              event_ts TEXT NOT NULL,
              event_type TEXT NOT NULL,
              source TEXT NOT NULL,
              target_window_id INTEGER,
              payload_json TEXT NOT NULL,
              privacy_level TEXT NOT NULL DEFAULT 'normal',
              FOREIGN KEY(session_id) REFERENCES capture_sessions(id)
            );

            CREATE TABLE IF NOT EXISTS window_events (
              raw_event_id INTEGER PRIMARY KEY,
              hwnd INTEGER,
              pid INTEGER,
              process_name TEXT,
              exe_path_hash TEXT,
              window_title TEXT,
              capture_status TEXT NOT NULL,
              FOREIGN KEY(raw_event_id) REFERENCES raw_events(id)
            );

            CREATE INDEX IF NOT EXISTS idx_raw_events_ts ON raw_events(event_ts);
            CREATE INDEX IF NOT EXISTS idx_raw_events_session ON raw_events(session_id);
            "#,
        )?;
        Ok(())
    }

    pub fn create_session(&self, app_version: &str, config_hash: &str) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let host_id = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown-host".to_string());

        self.conn.execute(
            r#"
            INSERT INTO capture_sessions
              (id, started_at, host_id, app_version, config_hash)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                session_id,
                Utc::now().to_rfc3339(),
                host_id,
                app_version,
                config_hash
            ],
        )?;

        Ok(session_id)
    }

    pub fn insert_window_focus(
        &mut self,
        session_id: &str,
        snapshot: &WindowSnapshot,
    ) -> Result<i64> {
        let payload_json = serde_json::to_string(snapshot)?;
        let tx = self.conn.transaction()?;

        tx.execute(
            r#"
            INSERT INTO raw_events
              (session_id, event_ts, event_type, source, target_window_id, payload_json)
            VALUES (?1, ?2, 'window_focus', 'window_collector', ?3, ?4)
            "#,
            params![
                session_id,
                snapshot.captured_at.to_rfc3339(),
                snapshot.hwnd,
                payload_json
            ],
        )?;
        let raw_event_id = tx.last_insert_rowid();

        tx.execute(
            r#"
            INSERT INTO window_events
              (raw_event_id, hwnd, pid, process_name, exe_path_hash, window_title, capture_status)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                raw_event_id,
                snapshot.hwnd,
                snapshot.pid,
                snapshot.process_name,
                snapshot.exe_path_hash,
                snapshot.window_title,
                snapshot.capture_status.as_str()
            ],
        )?;
        tx.commit()?;

        Ok(raw_event_id)
    }

    pub fn list_window_events(&self, limit: usize) -> Result<Vec<StoredWindowEvent>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT *
            FROM (
              SELECT
                r.id AS raw_event_id,
                r.session_id,
                r.event_ts,
                w.hwnd,
                w.pid,
                w.process_name,
                w.exe_path_hash,
                w.window_title,
                w.capture_status
              FROM raw_events r
              JOIN window_events w ON w.raw_event_id = r.id
              WHERE r.event_type = 'window_focus'
              ORDER BY r.event_ts DESC, r.id DESC
              LIMIT ?1
            )
            ORDER BY event_ts ASC, raw_event_id ASC
            "#,
        )?;

        let rows = statement.query_map([limit as i64], |row| {
            let event_ts: String = row.get(2)?;
            let capture_status: String = row.get(8)?;
            Ok(StoredWindowEvent {
                raw_event_id: row.get(0)?,
                session_id: row.get(1)?,
                event_ts: parse_ts(&event_ts)?,
                hwnd: row.get(3)?,
                pid: row.get(4)?,
                process_name: row.get(5)?,
                exe_path_hash: row.get(6)?,
                window_title: row.get(7)?,
                capture_status: CaptureStatus::from_db(&capture_status),
            })
        })?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }

        Ok(events)
    }
}

fn parse_ts(value: &str) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))
}
