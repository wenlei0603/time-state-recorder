use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use uuid::Uuid;

use crate::models::{
    AppInputCount, AppScreenshotCount, BlockerHit, CaptureStatus, InputEvent,
    InputEventType, InputSummary, ScreenshotMeta, ScreenshotSummary,
    StoredWindowEvent, TextSegment, WindowSnapshot,
};

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

            CREATE TABLE IF NOT EXISTS blocker_hits (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              hit_at TEXT NOT NULL,
              capture_type TEXT NOT NULL,
              field TEXT NOT NULL,
              operator TEXT NOT NULL,
              rule_value TEXT NOT NULL,
              actual_value TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_blocker_hits_at ON blocker_hits(hit_at);

            CREATE TABLE IF NOT EXISTS screenshot_thumbnails (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              captured_at TEXT NOT NULL,
              file_path TEXT NOT NULL,
              width INTEGER NOT NULL,
              height INTEGER NOT NULL,
              process_name TEXT,
              window_title TEXT,
              capture_status TEXT NOT NULL DEFAULT 'ok',
              session_id TEXT NOT NULL,
              FOREIGN KEY(session_id) REFERENCES capture_sessions(id)
            );
            CREATE INDEX IF NOT EXISTS idx_screenshots_at ON screenshot_thumbnails(captured_at);

            CREATE TABLE IF NOT EXISTS input_events (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              event_ts TEXT NOT NULL,
              event_type TEXT NOT NULL,
              vk_code INTEGER NOT NULL,
              scan_code INTEGER NOT NULL,
              character TEXT,
              segment_id TEXT NOT NULL,
              foreground_hwnd INTEGER NOT NULL,
              foreground_pid INTEGER NOT NULL,
              process_name TEXT,
              window_title TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_input_events_ts ON input_events(event_ts);
            CREATE INDEX IF NOT EXISTS idx_input_events_segment ON input_events(segment_id);

            CREATE TABLE IF NOT EXISTS text_segments (
              id TEXT PRIMARY KEY,
              started_at TEXT NOT NULL,
              ended_at TEXT,
              text_content TEXT NOT NULL,
              key_count INTEGER NOT NULL DEFAULT 0,
              backspace_count INTEGER NOT NULL DEFAULT 0,
              delete_count INTEGER NOT NULL DEFAULT 0,
              foreground_hwnd INTEGER NOT NULL,
              foreground_pid INTEGER NOT NULL,
              process_name TEXT,
              window_title TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_text_segments_at ON text_segments(started_at);
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

    pub fn insert_blocker_hit(&mut self, hit: &BlockerHit) -> Result<i64> {
        self.conn.execute(
            r#"
            INSERT INTO blocker_hits
              (hit_at, capture_type, field, operator, rule_value, actual_value)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                hit.hit_at.to_rfc3339(),
                hit.capture_type,
                hit.field,
                hit.operator,
                hit.rule_value,
                hit.actual_value,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_blocker_hits(&self, limit: usize) -> Result<Vec<BlockerHit>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, hit_at, capture_type, field, operator, rule_value, actual_value
            FROM blocker_hits
            ORDER BY hit_at DESC, id DESC
            LIMIT ?1
            "#,
        )?;

        let rows = statement.query_map([limit as i64], |row| {
            let hit_at: String = row.get(1)?;
            Ok(BlockerHit {
                id: row.get(0)?,
                hit_at: parse_ts(&hit_at)?,
                capture_type: row.get(2)?,
                field: row.get(3)?,
                operator: row.get(4)?,
                rule_value: row.get(5)?,
                actual_value: row.get(6)?,
            })
        })?;

        let mut hits = Vec::new();
        for row in rows {
            hits.push(row?);
        }
        Ok(hits)
    }

    pub fn insert_screenshot(&mut self, meta: &ScreenshotMeta) -> Result<i64> {
        self.conn.execute(
            r#"
            INSERT INTO screenshot_thumbnails
              (captured_at, file_path, width, height, process_name, window_title, capture_status, session_id)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                meta.captured_at.to_rfc3339(),
                meta.file_path,
                meta.width,
                meta.height,
                meta.process_name,
                meta.window_title,
                meta.capture_status,
                "current",
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_screenshots_by_date(
        &self,
        date: &str,
        limit: usize,
    ) -> Result<Vec<ScreenshotMeta>> {
        let pattern = format!("{date}%");
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, captured_at, file_path, width, height, process_name, window_title, capture_status
            FROM screenshot_thumbnails
            WHERE captured_at LIKE ?1
            ORDER BY captured_at ASC
            LIMIT ?2
            "#,
        )?;

        let rows = statement.query_map(params![pattern, limit as i64], |row| {
            let captured_at: String = row.get(1)?;
            Ok(ScreenshotMeta {
                id: row.get(0)?,
                captured_at: parse_ts(&captured_at)?,
                file_path: row.get(2)?,
                width: row.get(3)?,
                height: row.get(4)?,
                process_name: row.get(5)?,
                window_title: row.get(6)?,
                capture_status: row.get(7)?,
            })
        })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn get_screenshot_summary(&self, date: &str) -> Result<ScreenshotSummary> {
        let pattern = format!("{date}%");
        let total: usize = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM screenshot_thumbnails WHERE captured_at LIKE ?1",
                params![&pattern],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let hours: usize = self
            .conn
            .query_row(
                "SELECT COUNT(DISTINCT substr(captured_at, 12, 2)) FROM screenshot_thumbnails WHERE captured_at LIKE ?1",
                params![&pattern],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name, COUNT(*) as cnt
            FROM screenshot_thumbnails
            WHERE captured_at LIKE ?1 AND process_name IS NOT NULL
            GROUP BY process_name
            ORDER BY cnt DESC
            LIMIT 10
            "#,
        )?;

        let top_apps: Vec<AppScreenshotCount> = stmt
            .query_map(params![&pattern], |row| {
                Ok(AppScreenshotCount {
                    process_name: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(ScreenshotSummary {
            date: date.to_string(),
            total_screenshots: total,
            hours_covered: hours,
            top_apps,
        })
    }

    pub fn insert_input_segment(
        &mut self,
        segment: &crate::models::TextSegment,
        events: &[crate::models::InputEvent],
    ) -> Result<()> {
        let tx = self.conn.transaction()?;

        tx.execute(
            r#"
            INSERT INTO text_segments
              (id, started_at, ended_at, text_content, key_count, backspace_count, delete_count,
               foreground_hwnd, foreground_pid, process_name, window_title)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                segment.id,
                segment.started_at.to_rfc3339(),
                segment.ended_at.map(|t| t.to_rfc3339()),
                segment.text_content,
                segment.key_count,
                segment.backspace_count,
                segment.delete_count,
                segment.foreground_hwnd,
                segment.foreground_pid,
                segment.process_name,
                segment.window_title,
            ],
        )?;

        for event in events {
            tx.execute(
                r#"
                INSERT INTO input_events
                  (event_ts, event_type, vk_code, scan_code, character, segment_id,
                   foreground_hwnd, foreground_pid, process_name, window_title)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
                params![
                    event.event_ts.to_rfc3339(),
                    event.event_type.as_str(),
                    event.vk_code,
                    event.scan_code,
                    event.character,
                    event.segment_id,
                    event.foreground_hwnd,
                    event.foreground_pid,
                    event.process_name,
                    event.window_title,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn list_input_events(
        &self,
        limit: usize,
        segment_id: Option<&str>,
    ) -> Result<Vec<crate::models::InputEvent>> {
        let query = if segment_id.is_some() {
            "SELECT id, event_ts, event_type, vk_code, scan_code, character, segment_id,
                    foreground_hwnd, foreground_pid, process_name, window_title
             FROM input_events
             WHERE segment_id = ?2
             ORDER BY event_ts ASC, id ASC
             LIMIT ?1"
        } else {
            "SELECT id, event_ts, event_type, vk_code, scan_code, character, segment_id,
                    foreground_hwnd, foreground_pid, process_name, window_title
             FROM input_events
             ORDER BY event_ts DESC, id DESC
             LIMIT ?1"
        };

        let mut stmt = self.conn.prepare(query)?;

        let rows = if let Some(sid) = segment_id {
            stmt.query_map(params![limit as i64, sid], |row| {
                let event_ts: String = row.get(1)?;
                let event_type: String = row.get(2)?;
                Ok(crate::models::InputEvent {
                    id: row.get(0)?,
                    event_ts: parse_ts(&event_ts)?,
                    event_type: crate::models::InputEventType::from_db(&event_type),
                    vk_code: row.get(3)?,
                    scan_code: row.get(4)?,
                    character: row.get(5)?,
                    segment_id: row.get(6)?,
                    foreground_hwnd: row.get(7)?,
                    foreground_pid: row.get(8)?,
                    process_name: row.get(9)?,
                    window_title: row.get(10)?,
                })
            })?
        } else {
            stmt.query_map(params![limit as i64], |row| {
                let event_ts: String = row.get(1)?;
                let event_type: String = row.get(2)?;
                Ok(crate::models::InputEvent {
                    id: row.get(0)?,
                    event_ts: parse_ts(&event_ts)?,
                    event_type: crate::models::InputEventType::from_db(&event_type),
                    vk_code: row.get(3)?,
                    scan_code: row.get(4)?,
                    character: row.get(5)?,
                    segment_id: row.get(6)?,
                    foreground_hwnd: row.get(7)?,
                    foreground_pid: row.get(8)?,
                    process_name: row.get(9)?,
                    window_title: row.get(10)?,
                })
            })?
        };

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    pub fn list_text_segments(
        &self,
        date: &str,
        limit: usize,
    ) -> Result<Vec<crate::models::TextSegment>> {
        let pattern = format!("{date}%");
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, started_at, ended_at, text_content, key_count, backspace_count, delete_count,
                   foreground_hwnd, foreground_pid, process_name, window_title
            FROM text_segments
            WHERE started_at LIKE ?1
            ORDER BY started_at DESC
            LIMIT ?2
            "#,
        )?;

        let rows = stmt.query_map(params![&pattern, limit as i64], |row| {
            let started_at: String = row.get(1)?;
            let ended_at: Option<String> = row.get(2)?;
            Ok(crate::models::TextSegment {
                id: row.get(0)?,
                started_at: parse_ts(&started_at)?,
                ended_at: match ended_at {
                    Some(s) => Some(parse_ts(&s)?),
                    None => None,
                },
                text_content: row.get(3)?,
                key_count: row.get(4)?,
                backspace_count: row.get(5)?,
                delete_count: row.get(6)?,
                foreground_hwnd: row.get(7)?,
                foreground_pid: row.get(8)?,
                process_name: row.get(9)?,
                window_title: row.get(10)?,
            })
        })?;

        let mut segments = Vec::new();
        for row in rows {
            segments.push(row?);
        }
        Ok(segments)
    }

    pub fn get_input_summary(&self, date: &str) -> Result<crate::models::InputSummary> {
        let pattern = format!("{date}%");

        let total: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM input_events WHERE event_ts LIKE ?1",
            params![&pattern],
            |row| row.get(0),
        ).unwrap_or(0);

        let keydown: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM input_events WHERE event_ts LIKE ?1 AND event_type = 'keydown'",
            params![&pattern],
            |row| row.get(0),
        ).unwrap_or(0);

        let keyup: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM input_events WHERE event_ts LIKE ?1 AND event_type = 'keyup'",
            params![&pattern],
            |row| row.get(0),
        ).unwrap_or(0);

        let segments: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM text_segments WHERE started_at LIKE ?1",
            params![&pattern],
            |row| row.get(0),
        ).unwrap_or(0);

        let total_chars: usize = self.conn.query_row(
            "SELECT COALESCE(SUM(LENGTH(text_content)), 0) FROM text_segments WHERE started_at LIKE ?1",
            params![&pattern],
            |row| row.get(0),
        ).unwrap_or(0);

        let last_activity: Option<String> = self.conn.query_row(
            "SELECT event_ts FROM input_events WHERE event_ts LIKE ?1 ORDER BY event_ts DESC LIMIT 1",
            params![&pattern],
            |row| row.get(0),
        ).ok();

        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name, SUM(LENGTH(text_content)) as total_chars
            FROM text_segments
            WHERE started_at LIKE ?1 AND process_name IS NOT NULL
            GROUP BY process_name
            ORDER BY total_chars DESC
            LIMIT 10
            "#,
        )?;

        let top_apps: Vec<crate::models::AppInputCount> = stmt
            .query_map(params![&pattern], |row| {
                Ok(crate::models::AppInputCount {
                    process_name: row.get(0)?,
                    char_count: row.get::<_, i64>(1)? as usize,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(crate::models::InputSummary {
            date: date.to_string(),
            total_events: total,
            keydown_count: keydown,
            keyup_count: keyup,
            segment_count: segments,
            total_chars,
            last_activity: last_activity.and_then(|s| parse_ts(&s).ok()),
            top_apps,
        })
    }

    pub fn get_db_stats(&self) -> Result<crate::models::DbStats> {
        let window_events: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM window_events", [], |r| r.get(0))?;
        let input_events: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM input_events", [], |r| r.get(0))?;
        let text_segments: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM text_segments", [], |r| r.get(0))?;
        let screenshots: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM screenshot_thumbnails", [], |r| r.get(0))?;
        let blocker_hits: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM blocker_hits", [], |r| r.get(0))?;

        Ok(crate::models::DbStats {
            window_events,
            input_events,
            text_segments,
            screenshots,
            blocker_hits,
        })
    }
}

pub(crate) fn parse_ts(value: &str) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))
}
