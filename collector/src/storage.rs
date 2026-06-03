use std::path::Path;

use anyhow::{Result, ensure};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, Transaction, params, types::Type};
use uuid::Uuid;

use crate::models::{
    ActivityCategory, AppScreenshotCount, BlockerHit, CaptureStatus, LifecycleEvent, LifecycleType,
    ScreenshotMeta, ScreenshotSkippedReasonCount, ScreenshotSummary, StoredWindowEvent,
    VisualSummary, WindowSnapshot,
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
              ended_reason TEXT,
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

            CREATE TABLE IF NOT EXISTS lifecycle_events (
              raw_event_id INTEGER PRIMARY KEY,
              lifecycle_type TEXT NOT NULL,
              reason TEXT,
              active_session_id TEXT,
              payload_json TEXT NOT NULL,
              FOREIGN KEY(raw_event_id) REFERENCES raw_events(id),
              FOREIGN KEY(active_session_id) REFERENCES capture_sessions(id)
            );
            CREATE INDEX IF NOT EXISTS idx_lifecycle_events_type ON lifecycle_events(lifecycle_type);

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

            CREATE TABLE IF NOT EXISTS visual_summaries (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              screenshot_id INTEGER NOT NULL,
              captured_at TEXT NOT NULL,
              model_provider TEXT NOT NULL,
              model_name TEXT NOT NULL,
              prompt_version TEXT NOT NULL,
              summary_text TEXT NOT NULL,
              activity_category TEXT NOT NULL,
              project_hints_json TEXT NOT NULL,
              visible_apps_json TEXT NOT NULL,
              visible_text_hints_json TEXT NOT NULL,
              risk_flags_json TEXT NOT NULL,
              confidence REAL NOT NULL,
              created_at TEXT NOT NULL,
              error TEXT,
              FOREIGN KEY(screenshot_id) REFERENCES screenshot_thumbnails(id)
            );
            CREATE INDEX IF NOT EXISTS idx_visual_summaries_at ON visual_summaries(captured_at);
            CREATE INDEX IF NOT EXISTS idx_visual_summaries_screenshot ON visual_summaries(screenshot_id);

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
        self.ensure_column("capture_sessions", "ended_reason", "TEXT")?;
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

    pub fn close_session(
        &mut self,
        session_id: &str,
        ended_at: DateTime<Utc>,
        reason: &str,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        let changed = tx.execute(
            r#"
            UPDATE capture_sessions
            SET ended_at = ?2, ended_reason = ?3
            WHERE id = ?1 AND ended_at IS NULL
            "#,
            params![session_id, ended_at.to_rfc3339(), reason],
        )?;
        ensure!(changed == 1, "session is already closed or missing");

        insert_lifecycle_event_tx(
            &tx,
            session_id,
            ended_at,
            LifecycleType::SessionStop,
            Some(reason),
            serde_json::json!({ "reason": reason }),
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn close_stale_sessions(
        &mut self,
        ended_at: DateTime<Utc>,
        reason: &str,
    ) -> Result<Vec<String>> {
        let sessions = {
            let mut stmt = self.conn.prepare(
                r#"
                SELECT
                  s.id,
                  COALESCE(MAX(r.event_ts), s.started_at) AS last_recorded_at
                FROM capture_sessions s
                LEFT JOIN raw_events r ON r.session_id = s.id
                WHERE s.ended_at IS NULL
                GROUP BY s.id, s.started_at
                ORDER BY s.started_at ASC, s.id ASC
                "#,
            )?;
            let rows = stmt.query_map([], |row| {
                let last_recorded_at: String = row.get(1)?;
                Ok((row.get::<_, String>(0)?, parse_ts(&last_recorded_at)?))
            })?;

            let mut sessions = Vec::new();
            for row in rows {
                sessions.push(row?);
            }
            sessions
        };

        let tx = self.conn.transaction()?;
        let mut closed_session_ids = Vec::new();

        for (session_id, boundary_at) in sessions {
            let changed = tx.execute(
                r#"
                UPDATE capture_sessions
                SET ended_at = ?2, ended_reason = ?3
                WHERE id = ?1 AND ended_at IS NULL
                "#,
                params![session_id, boundary_at.to_rfc3339(), reason],
            )?;

            if changed == 1 {
                insert_lifecycle_event_tx(
                    &tx,
                    &session_id,
                    boundary_at,
                    LifecycleType::CollectorGap,
                    Some(reason),
                    serde_json::json!({
                        "reason": reason,
                        "detectedAt": ended_at.to_rfc3339(),
                    }),
                )?;
                closed_session_ids.push(session_id);
            }
        }
        tx.commit()?;

        Ok(closed_session_ids)
    }

    pub fn insert_lifecycle_event(
        &mut self,
        session_id: &str,
        event_ts: DateTime<Utc>,
        lifecycle_type: LifecycleType,
        reason: Option<&str>,
        payload: serde_json::Value,
    ) -> Result<i64> {
        let tx = self.conn.transaction()?;
        ensure_session_open_tx(&tx, session_id)?;
        let raw_event_id =
            insert_lifecycle_event_tx(&tx, session_id, event_ts, lifecycle_type, reason, payload)?;
        tx.commit()?;

        Ok(raw_event_id)
    }

    pub fn list_lifecycle_events(&self, limit: usize) -> Result<Vec<LifecycleEvent>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT *
            FROM (
              SELECT
                r.id AS raw_event_id,
                r.session_id,
                r.event_ts,
                l.lifecycle_type,
                l.reason,
                l.active_session_id,
                l.payload_json
              FROM raw_events r
              JOIN lifecycle_events l ON l.raw_event_id = r.id
              WHERE r.event_type = 'lifecycle'
              ORDER BY r.event_ts DESC, r.id DESC
              LIMIT ?1
            )
            ORDER BY event_ts ASC, raw_event_id ASC
            "#,
        )?;

        let rows = statement.query_map([limit as i64], |row| {
            let event_ts: String = row.get(2)?;
            let lifecycle_type: String = row.get(3)?;
            let payload_json: String = row.get(6)?;
            let lifecycle_type = LifecycleType::from_db(&lifecycle_type).ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unknown lifecycle_type: {lifecycle_type}"),
                    )),
                )
            })?;
            Ok(LifecycleEvent {
                raw_event_id: row.get(0)?,
                session_id: row.get(1)?,
                event_ts: parse_ts(&event_ts)?,
                lifecycle_type,
                reason: row.get(4)?,
                active_session_id: row.get(5)?,
                payload: parse_json(&payload_json)?,
            })
        })?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    pub fn insert_window_focus(
        &mut self,
        session_id: &str,
        snapshot: &WindowSnapshot,
    ) -> Result<i64> {
        let payload_json = serde_json::to_string(snapshot)?;
        let tx = self.conn.transaction()?;
        ensure_session_open_tx(&tx, session_id)?;

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

    pub fn insert_screenshot(&mut self, session_id: &str, meta: &ScreenshotMeta) -> Result<i64> {
        let tx = self.conn.transaction()?;
        ensure_session_open_tx(&tx, session_id)?;
        tx.execute(
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
                session_id,
            ],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(id)
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
            WHERE captured_at LIKE ?1 AND capture_status = 'ok'
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

    pub fn get_screenshot(&self, id: i64) -> Result<Option<ScreenshotMeta>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, captured_at, file_path, width, height, process_name, window_title, capture_status
            FROM screenshot_thumbnails
            WHERE id = ?1
            "#,
        )?;

        let mut rows = statement.query_map(params![id], |row| {
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

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn get_screenshot_summary(&self, date: &str) -> Result<ScreenshotSummary> {
        let pattern = format!("{date}%");
        let total: usize = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM screenshot_thumbnails WHERE captured_at LIKE ?1 AND capture_status = 'ok'",
                params![&pattern],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let hours: usize = self
            .conn
            .query_row(
                "SELECT COUNT(DISTINCT substr(captured_at, 12, 2)) FROM screenshot_thumbnails WHERE captured_at LIKE ?1 AND capture_status = 'ok'",
                params![&pattern],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name, COUNT(*) as cnt
            FROM screenshot_thumbnails
            WHERE captured_at LIKE ?1 AND capture_status = 'ok' AND process_name IS NOT NULL
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

        let mut stmt = self.conn.prepare(
            r#"
            SELECT capture_status, COUNT(*) as cnt
            FROM screenshot_thumbnails
            WHERE captured_at LIKE ?1 AND capture_status <> 'ok'
            GROUP BY capture_status
            ORDER BY capture_status ASC
            "#,
        )?;

        let skipped_reasons: Vec<ScreenshotSkippedReasonCount> = stmt
            .query_map(params![&pattern], |row| {
                Ok(ScreenshotSkippedReasonCount {
                    reason: row.get(0)?,
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
            skipped_reasons,
        })
    }

    pub fn insert_visual_summary(&mut self, summary: &VisualSummary) -> Result<i64> {
        self.conn.execute(
            r#"
            INSERT INTO visual_summaries
              (screenshot_id, captured_at, model_provider, model_name, prompt_version,
               summary_text, activity_category, project_hints_json, visible_apps_json,
               visible_text_hints_json, risk_flags_json, confidence, created_at, error)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            "#,
            params![
                summary.screenshot_id,
                summary.captured_at.to_rfc3339(),
                &summary.model_provider,
                &summary.model_name,
                &summary.prompt_version,
                &summary.summary_text,
                summary.activity_category.as_str(),
                serde_json::to_string(&summary.project_hints)?,
                serde_json::to_string(&summary.visible_apps)?,
                serde_json::to_string(&summary.visible_text_hints)?,
                serde_json::to_string(&summary.risk_flags)?,
                summary.confidence,
                summary.created_at.to_rfc3339(),
                summary.error.as_deref(),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_visual_summaries_by_date(
        &self,
        date: &str,
        limit: usize,
    ) -> Result<Vec<VisualSummary>> {
        let pattern = format!("{date}%");
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, screenshot_id, captured_at, model_provider, model_name, prompt_version,
                   summary_text, activity_category, project_hints_json, visible_apps_json,
                   visible_text_hints_json, risk_flags_json, confidence, created_at, error
            FROM visual_summaries
            WHERE captured_at LIKE ?1
            ORDER BY captured_at ASC, id ASC
            LIMIT ?2
            "#,
        )?;

        let rows = statement.query_map(params![pattern, limit as i64], map_visual_summary_row)?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
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
             FROM (
               SELECT id, event_ts, event_type, vk_code, scan_code, character, segment_id,
                      foreground_hwnd, foreground_pid, process_name, window_title
               FROM input_events
               ORDER BY event_ts DESC, id DESC
               LIMIT ?1
             )
             ORDER BY event_ts ASC, id ASC"
        };

        let mut stmt = self.conn.prepare(query)?;

        let rows = if let Some(sid) = segment_id {
            stmt.query_map(params![limit as i64, sid], map_input_event_row)?
        } else {
            stmt.query_map(params![limit as i64], map_input_event_row)?
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

        let total: usize = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM input_events WHERE event_ts LIKE ?1",
                params![&pattern],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let keydown: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM input_events WHERE event_ts LIKE ?1 AND event_type = 'keydown'",
            params![&pattern],
            |row| row.get(0),
        ).unwrap_or(0);

        let keyup: usize = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM input_events WHERE event_ts LIKE ?1 AND event_type = 'keyup'",
                params![&pattern],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let segments: usize = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM text_segments WHERE started_at LIKE ?1",
                params![&pattern],
                |row| row.get(0),
            )
            .unwrap_or(0);

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
        let window_events: usize =
            self.conn
                .query_row("SELECT COUNT(*) FROM window_events", [], |r| r.get(0))?;
        let lifecycle_events: usize =
            self.conn
                .query_row("SELECT COUNT(*) FROM lifecycle_events", [], |r| r.get(0))?;
        let input_events: usize =
            self.conn
                .query_row("SELECT COUNT(*) FROM input_events", [], |r| r.get(0))?;
        let text_segments: usize =
            self.conn
                .query_row("SELECT COUNT(*) FROM text_segments", [], |r| r.get(0))?;
        let screenshots: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM screenshot_thumbnails WHERE capture_status = 'ok'",
            [],
            |r| r.get(0),
        )?;
        let blocker_hits: usize =
            self.conn
                .query_row("SELECT COUNT(*) FROM blocker_hits", [], |r| r.get(0))?;

        Ok(crate::models::DbStats {
            window_events,
            lifecycle_events,
            input_events,
            text_segments,
            screenshots,
            blocker_hits,
        })
    }

    fn ensure_column(&self, table: &str, column: &str, column_type: &str) -> Result<()> {
        let mut stmt = self.conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;

        for row in rows {
            if row? == column {
                return Ok(());
            }
        }

        self.conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {column_type}"),
            [],
        )?;
        Ok(())
    }
}

fn map_input_event_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<crate::models::InputEvent> {
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
}

fn map_visual_summary_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<VisualSummary> {
    let captured_at: String = row.get(2)?;
    let activity_category: String = row.get(7)?;
    let project_hints_json: String = row.get(8)?;
    let visible_apps_json: String = row.get(9)?;
    let visible_text_hints_json: String = row.get(10)?;
    let risk_flags_json: String = row.get(11)?;
    let created_at: String = row.get(13)?;
    let activity_category = ActivityCategory::from_db(&activity_category).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            7,
            Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown activity_category: {activity_category}"),
            )),
        )
    })?;

    Ok(VisualSummary {
        id: row.get(0)?,
        screenshot_id: row.get(1)?,
        captured_at: parse_ts(&captured_at)?,
        model_provider: row.get(3)?,
        model_name: row.get(4)?,
        prompt_version: row.get(5)?,
        summary_text: row.get(6)?,
        activity_category,
        project_hints: parse_string_vec(&project_hints_json)?,
        visible_apps: parse_string_vec(&visible_apps_json)?,
        visible_text_hints: parse_string_vec(&visible_text_hints_json)?,
        risk_flags: parse_string_vec(&risk_flags_json)?,
        confidence: row.get(12)?,
        created_at: parse_ts(&created_at)?,
        error: row.get(14)?,
    })
}

pub(crate) fn parse_ts(value: &str) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))
}

fn parse_json(value: &str) -> rusqlite::Result<serde_json::Value> {
    serde_json::from_str(value)
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err)))
}

fn parse_string_vec(value: &str) -> rusqlite::Result<Vec<String>> {
    serde_json::from_str(value)
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err)))
}

fn insert_lifecycle_event_tx(
    tx: &Transaction<'_>,
    session_id: &str,
    event_ts: DateTime<Utc>,
    lifecycle_type: LifecycleType,
    reason: Option<&str>,
    payload: serde_json::Value,
) -> Result<i64> {
    let payload_json = serde_json::to_string(&payload)?;

    tx.execute(
        r#"
        INSERT INTO raw_events
          (session_id, event_ts, event_type, source, target_window_id, payload_json)
        VALUES (?1, ?2, 'lifecycle', 'lifecycle_collector', NULL, ?3)
        "#,
        params![session_id, event_ts.to_rfc3339(), payload_json],
    )?;
    let raw_event_id = tx.last_insert_rowid();

    tx.execute(
        r#"
        INSERT INTO lifecycle_events
          (raw_event_id, lifecycle_type, reason, active_session_id, payload_json)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        params![
            raw_event_id,
            lifecycle_type.as_str(),
            reason,
            session_id,
            payload_json
        ],
    )?;

    Ok(raw_event_id)
}

fn ensure_session_open_tx(tx: &Transaction<'_>, session_id: &str) -> Result<()> {
    let is_open: bool = tx.query_row(
        r#"
        SELECT EXISTS(
          SELECT 1
          FROM capture_sessions
          WHERE id = ?1 AND ended_at IS NULL
        )
        "#,
        params![session_id],
        |row| row.get(0),
    )?;
    ensure!(is_open, "session is closed or missing");
    Ok(())
}
