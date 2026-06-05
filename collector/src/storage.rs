use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::{Result, ensure};
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction, params, types::Type};
use uuid::Uuid;

use crate::models::{
    ActivityCategory, ActivityCategoryCount, AppScreenshotCount, BlockerHit, CaptureStatus,
    DailyActivityStats, DailyAppActivity, DailyBrief, DailyComparison, HighResScreenshotMeta,
    HourlyActivityMetric, ImageRetentionStats, InsightReport, LifecycleEvent, LifecycleType,
    ScreenshotMeta, ScreenshotSkippedReasonCount, ScreenshotSummary, StoredWindowEvent,
    VisualObservation, VisualSummary, VisualTrajectoryPoint, VisualWindowSummary, WindowSnapshot,
};

pub struct Store {
    conn: Connection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenshotStoreKind {
    Thumbnail,
    HighRes,
}

impl ScreenshotStoreKind {
    fn table_name(self) -> &'static str {
        match self {
            Self::Thumbnail => "screenshot_thumbnails",
            Self::HighRes => "high_res_screenshots",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredScreenshotFile {
    pub kind: ScreenshotStoreKind,
    pub id: i64,
    pub captured_at: DateTime<Utc>,
    pub file_path: String,
    pub file_size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActivitySlice {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    seconds: i64,
    app: String,
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
              file_size_bytes INTEGER NOT NULL DEFAULT 0,
              expired_at TEXT,
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

            CREATE TABLE IF NOT EXISTS high_res_screenshots (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              captured_at TEXT NOT NULL,
              file_path TEXT NOT NULL,
              width INTEGER NOT NULL,
              height INTEGER NOT NULL,
              process_name TEXT,
              window_title TEXT,
              capture_status TEXT NOT NULL DEFAULT 'ok',
              file_size_bytes INTEGER NOT NULL DEFAULT 0,
              expired_at TEXT,
              session_id TEXT NOT NULL,
              FOREIGN KEY(session_id) REFERENCES capture_sessions(id)
            );
            CREATE INDEX IF NOT EXISTS idx_high_res_screenshots_at ON high_res_screenshots(captured_at);

            CREATE TABLE IF NOT EXISTS visual_observations (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              high_res_screenshot_id INTEGER NOT NULL UNIQUE,
              captured_at TEXT NOT NULL,
              file_path TEXT NOT NULL,
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
              FOREIGN KEY(high_res_screenshot_id) REFERENCES high_res_screenshots(id)
            );
            CREATE INDEX IF NOT EXISTS idx_visual_observations_at ON visual_observations(captured_at);

            CREATE TABLE IF NOT EXISTS visual_window_summaries (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              window_start TEXT NOT NULL,
              window_end TEXT NOT NULL,
              sampled_screenshot_ids_json TEXT NOT NULL,
              previous_summary_id INTEGER,
              model_provider TEXT NOT NULL,
              model_name TEXT NOT NULL,
              prompt_version TEXT NOT NULL,
              summary_text TEXT NOT NULL,
              continuity TEXT NOT NULL,
              primary_activity TEXT NOT NULL,
              project_hints_json TEXT NOT NULL,
              task_intent TEXT NOT NULL,
              trajectory_json TEXT NOT NULL,
              switching_level TEXT NOT NULL,
              switching_evidence TEXT NOT NULL,
              loafing_level TEXT NOT NULL,
              loafing_evidence TEXT NOT NULL,
              visible_apps_json TEXT NOT NULL,
              visible_text_hints_json TEXT NOT NULL,
              risk_flags_json TEXT NOT NULL,
              confidence REAL NOT NULL,
              raw_summary_json TEXT NOT NULL,
              created_at TEXT NOT NULL,
              error TEXT,
              FOREIGN KEY(previous_summary_id) REFERENCES visual_window_summaries(id)
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_visual_window_summaries_window
              ON visual_window_summaries(window_start, window_end);
            CREATE INDEX IF NOT EXISTS idx_visual_window_summaries_start
              ON visual_window_summaries(window_start);

            CREATE TABLE IF NOT EXISTS insight_reports (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              period_start TEXT NOT NULL,
              period_end TEXT NOT NULL,
              generated_at TEXT NOT NULL,
              report_kind TEXT NOT NULL,
              model_provider TEXT NOT NULL,
              model_name TEXT NOT NULL,
              summary_text TEXT NOT NULL,
              category_mix_json TEXT NOT NULL,
              project_hints_json TEXT NOT NULL,
              evidence_count INTEGER NOT NULL,
              error TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_insight_reports_period ON insight_reports(period_start, period_end);
            CREATE INDEX IF NOT EXISTS idx_insight_reports_kind_period
              ON insight_reports(report_kind, period_start, period_end);

            CREATE TABLE IF NOT EXISTS daily_briefs (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              date TEXT NOT NULL,
              period_start TEXT NOT NULL,
              period_end TEXT NOT NULL,
              generated_at TEXT NOT NULL,
              scheduled_for_local TEXT NOT NULL,
              model_provider TEXT NOT NULL,
              model_name TEXT NOT NULL,
              prompt_version TEXT NOT NULL,
              status TEXT NOT NULL,
              descriptive_stats_json TEXT NOT NULL,
              hourly_metrics_json TEXT NOT NULL,
              comparison_json TEXT NOT NULL,
              five_hour_report_ids_json TEXT NOT NULL,
              daily_summary_text TEXT NOT NULL,
              action_trajectory TEXT NOT NULL,
              raw_summary_json TEXT NOT NULL,
              error TEXT
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_daily_briefs_date_schedule
              ON daily_briefs(date, scheduled_for_local);
            CREATE INDEX IF NOT EXISTS idx_daily_briefs_generated
              ON daily_briefs(generated_at);

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
        self.ensure_column(
            "screenshot_thumbnails",
            "file_size_bytes",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        self.ensure_column("screenshot_thumbnails", "expired_at", "TEXT")?;
        self.ensure_column(
            "high_res_screenshots",
            "file_size_bytes",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        self.ensure_column("high_res_screenshots", "expired_at", "TEXT")?;
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
        self.insert_screenshot_with_file_size(session_id, meta, 0)
    }

    pub fn insert_screenshot_with_file_size(
        &mut self,
        session_id: &str,
        meta: &ScreenshotMeta,
        file_size_bytes: u64,
    ) -> Result<i64> {
        let tx = self.conn.transaction()?;
        ensure_session_open_tx(&tx, session_id)?;
        tx.execute(
            r#"
            INSERT INTO screenshot_thumbnails
              (captured_at, file_path, width, height, process_name, window_title, capture_status,
               file_size_bytes, session_id)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                meta.captured_at.to_rfc3339(),
                meta.file_path,
                meta.width,
                meta.height,
                meta.process_name,
                meta.window_title,
                meta.capture_status,
                file_size_bytes as i64,
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
        let (start, end) = utc_day_bounds(date)?;
        self.list_screenshots_between(start, end, limit)
    }

    pub fn list_screenshots_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<ScreenshotMeta>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, captured_at, file_path, width, height, process_name, window_title, capture_status
            FROM screenshot_thumbnails
            WHERE captured_at >= ?1 AND captured_at < ?2 AND capture_status = 'ok'
            ORDER BY captured_at ASC
            LIMIT ?3
            "#,
        )?;

        let rows = statement.query_map(
            params![start.to_rfc3339(), end.to_rfc3339(), limit as i64],
            |row| {
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
            },
        )?;

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
        let (start, end) = utc_day_bounds(date)?;
        self.get_screenshot_summary_between(date, start, end)
    }

    pub fn get_screenshot_summary_between(
        &self,
        date: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<ScreenshotSummary> {
        let start = start.to_rfc3339();
        let end = end.to_rfc3339();
        let total: usize = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM screenshot_thumbnails WHERE captured_at >= ?1 AND captured_at < ?2 AND capture_status = 'ok'",
                params![&start, &end],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let hours: usize = self
            .conn
            .query_row(
                "SELECT COUNT(DISTINCT substr(captured_at, 1, 13)) FROM screenshot_thumbnails WHERE captured_at >= ?1 AND captured_at < ?2 AND capture_status = 'ok'",
                params![&start, &end],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name, COUNT(*) as cnt
            FROM screenshot_thumbnails
            WHERE captured_at >= ?1 AND captured_at < ?2 AND capture_status = 'ok' AND process_name IS NOT NULL
            GROUP BY process_name
            ORDER BY cnt DESC
            LIMIT 10
            "#,
        )?;

        let top_apps: Vec<AppScreenshotCount> = stmt
            .query_map(params![&start, &end], |row| {
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
            WHERE captured_at >= ?1 AND captured_at < ?2 AND capture_status <> 'ok'
            GROUP BY capture_status
            ORDER BY capture_status ASC
            "#,
        )?;

        let skipped_reasons: Vec<ScreenshotSkippedReasonCount> = stmt
            .query_map(params![&start, &end], |row| {
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

    pub fn insert_high_res_screenshot(
        &mut self,
        session_id: &str,
        meta: &HighResScreenshotMeta,
    ) -> Result<i64> {
        self.insert_high_res_screenshot_with_file_size(session_id, meta, 0)
    }

    pub fn insert_high_res_screenshot_with_file_size(
        &mut self,
        session_id: &str,
        meta: &HighResScreenshotMeta,
        file_size_bytes: u64,
    ) -> Result<i64> {
        let tx = self.conn.transaction()?;
        ensure_session_open_tx(&tx, session_id)?;
        tx.execute(
            r#"
            INSERT INTO high_res_screenshots
              (captured_at, file_path, width, height, process_name, window_title, capture_status,
               file_size_bytes, session_id)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                meta.captured_at.to_rfc3339(),
                meta.file_path,
                meta.width,
                meta.height,
                meta.process_name,
                meta.window_title,
                meta.capture_status,
                file_size_bytes as i64,
                session_id,
            ],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(id)
    }

    pub fn list_high_res_screenshots_by_date(
        &self,
        date: &str,
        limit: usize,
    ) -> Result<Vec<HighResScreenshotMeta>> {
        let (start, end) = utc_day_bounds(date)?;
        self.list_high_res_screenshots_between(start, end, limit)
    }

    pub fn list_high_res_screenshots_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<HighResScreenshotMeta>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, captured_at, file_path, width, height, process_name, window_title, capture_status
            FROM high_res_screenshots
            WHERE captured_at >= ?1 AND captured_at < ?2 AND capture_status = 'ok'
            ORDER BY captured_at ASC
            LIMIT ?3
            "#,
        )?;

        let rows = statement.query_map(
            params![start.to_rfc3339(), end.to_rfc3339(), limit as i64],
            |row| {
                let captured_at: String = row.get(1)?;
                Ok(HighResScreenshotMeta {
                    id: row.get(0)?,
                    captured_at: parse_ts(&captured_at)?,
                    file_path: row.get(2)?,
                    width: row.get(3)?,
                    height: row.get(4)?,
                    process_name: row.get(5)?,
                    window_title: row.get(6)?,
                    capture_status: row.get(7)?,
                })
            },
        )?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
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
        let (start, end) = utc_day_bounds(date)?;
        self.list_visual_summaries_between(start, end, limit)
    }

    pub fn list_visual_summaries_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<VisualSummary>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, screenshot_id, captured_at, model_provider, model_name, prompt_version,
                   summary_text, activity_category, project_hints_json, visible_apps_json,
                   visible_text_hints_json, risk_flags_json, confidence, created_at, error
            FROM visual_summaries
            WHERE captured_at >= ?1 AND captured_at < ?2
            ORDER BY captured_at ASC, id ASC
            LIMIT ?3
            "#,
        )?;

        let rows = statement.query_map(
            params![start.to_rfc3339(), end.to_rfc3339(), limit as i64],
            map_visual_summary_row,
        )?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn insert_visual_observation(&mut self, observation: &VisualObservation) -> Result<i64> {
        self.conn.execute(
            r#"
            INSERT INTO visual_observations
              (high_res_screenshot_id, captured_at, file_path, model_provider, model_name,
               prompt_version, summary_text, activity_category, project_hints_json,
               visible_apps_json, visible_text_hints_json, risk_flags_json, confidence,
               created_at, error)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            "#,
            params![
                observation.high_res_screenshot_id,
                observation.captured_at.to_rfc3339(),
                &observation.file_path,
                &observation.model_provider,
                &observation.model_name,
                &observation.prompt_version,
                &observation.summary_text,
                observation.activity_category.as_str(),
                serde_json::to_string(&observation.project_hints)?,
                serde_json::to_string(&observation.visible_apps)?,
                serde_json::to_string(&observation.visible_text_hints)?,
                serde_json::to_string(&observation.risk_flags)?,
                observation.confidence,
                observation.created_at.to_rfc3339(),
                observation.error.as_deref(),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_visual_observations_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<VisualObservation>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, high_res_screenshot_id, captured_at, file_path, model_provider, model_name,
                   prompt_version, summary_text, activity_category, project_hints_json,
                   visible_apps_json, visible_text_hints_json, risk_flags_json, confidence,
                   created_at, error
            FROM visual_observations
            WHERE captured_at >= ?1 AND captured_at < ?2
            ORDER BY captured_at ASC, id ASC
            LIMIT ?3
            "#,
        )?;

        let rows = statement.query_map(
            params![start.to_rfc3339(), end.to_rfc3339(), limit as i64],
            map_visual_observation_row,
        )?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn list_visual_observations(&self, limit: usize) -> Result<Vec<VisualObservation>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, high_res_screenshot_id, captured_at, file_path, model_provider, model_name,
                   prompt_version, summary_text, activity_category, project_hints_json,
                   visible_apps_json, visible_text_hints_json, risk_flags_json, confidence,
                   created_at, error
            FROM visual_observations
            ORDER BY captured_at DESC, id DESC
            LIMIT ?1
            "#,
        )?;

        let rows = statement.query_map(params![limit as i64], map_visual_observation_row)?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn list_unobserved_high_res_screenshots(
        &self,
        limit: usize,
    ) -> Result<Vec<HighResScreenshotMeta>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT h.id, h.captured_at, h.file_path, h.width, h.height,
                   h.process_name, h.window_title, h.capture_status
            FROM high_res_screenshots h
            LEFT JOIN visual_observations o ON o.high_res_screenshot_id = h.id
            WHERE h.capture_status = 'ok' AND o.id IS NULL
            ORDER BY h.captured_at ASC, h.id ASC
            LIMIT ?1
            "#,
        )?;

        let rows = statement.query_map(params![limit as i64], |row| {
            let captured_at: String = row.get(1)?;
            Ok(HighResScreenshotMeta {
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

    pub fn insert_visual_window_summary(&mut self, summary: &VisualWindowSummary) -> Result<i64> {
        self.conn.execute(
            r#"
            INSERT INTO visual_window_summaries
              (window_start, window_end, sampled_screenshot_ids_json, previous_summary_id,
               model_provider, model_name, prompt_version, summary_text, continuity,
               primary_activity, project_hints_json, task_intent, trajectory_json,
               switching_level, switching_evidence, loafing_level, loafing_evidence,
               visible_apps_json, visible_text_hints_json, risk_flags_json, confidence,
               raw_summary_json, created_at, error)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                    ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)
            "#,
            params![
                summary.window_start.to_rfc3339(),
                summary.window_end.to_rfc3339(),
                serde_json::to_string(&summary.sampled_screenshot_ids)?,
                summary.previous_summary_id,
                &summary.model_provider,
                &summary.model_name,
                &summary.prompt_version,
                &summary.summary_text,
                &summary.continuity,
                summary.primary_activity.as_str(),
                serde_json::to_string(&summary.project_hints)?,
                &summary.task_intent,
                serde_json::to_string(&summary.trajectory)?,
                &summary.switching_level,
                &summary.switching_evidence,
                &summary.loafing_level,
                &summary.loafing_evidence,
                serde_json::to_string(&summary.visible_apps)?,
                serde_json::to_string(&summary.visible_text_hints)?,
                serde_json::to_string(&summary.risk_flags)?,
                summary.confidence,
                serde_json::to_string(&summary.raw_summary_json)?,
                summary.created_at.to_rfc3339(),
                summary.error.as_deref(),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_visual_window_summaries_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<VisualWindowSummary>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, window_start, window_end, sampled_screenshot_ids_json, previous_summary_id,
                   model_provider, model_name, prompt_version, summary_text, continuity,
                   primary_activity, project_hints_json, task_intent, trajectory_json,
                   switching_level, switching_evidence, loafing_level, loafing_evidence,
                   visible_apps_json, visible_text_hints_json, risk_flags_json, confidence,
                   raw_summary_json, created_at, error
            FROM visual_window_summaries
            WHERE window_start >= ?1 AND window_start < ?2
            ORDER BY window_start ASC, id ASC
            LIMIT ?3
            "#,
        )?;

        let rows = statement.query_map(
            params![start.to_rfc3339(), end.to_rfc3339(), limit as i64],
            map_visual_window_summary_row,
        )?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn list_visual_window_summaries(&self, limit: usize) -> Result<Vec<VisualWindowSummary>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, window_start, window_end, sampled_screenshot_ids_json, previous_summary_id,
                   model_provider, model_name, prompt_version, summary_text, continuity,
                   primary_activity, project_hints_json, task_intent, trajectory_json,
                   switching_level, switching_evidence, loafing_level, loafing_evidence,
                   visible_apps_json, visible_text_hints_json, risk_flags_json, confidence,
                   raw_summary_json, created_at, error
            FROM visual_window_summaries
            ORDER BY window_start DESC, id DESC
            LIMIT ?1
            "#,
        )?;

        let rows = statement.query_map(params![limit as i64], map_visual_window_summary_row)?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn latest_visual_window_summary_before(
        &self,
        before: DateTime<Utc>,
    ) -> Result<Option<VisualWindowSummary>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, window_start, window_end, sampled_screenshot_ids_json, previous_summary_id,
                   model_provider, model_name, prompt_version, summary_text, continuity,
                   primary_activity, project_hints_json, task_intent, trajectory_json,
                   switching_level, switching_evidence, loafing_level, loafing_evidence,
                   visible_apps_json, visible_text_hints_json, risk_flags_json, confidence,
                   raw_summary_json, created_at, error
            FROM visual_window_summaries
            WHERE window_end <= ?1
            ORDER BY window_end DESC, id DESC
            LIMIT 1
            "#,
        )?;

        let mut rows =
            statement.query_map(params![before.to_rfc3339()], map_visual_window_summary_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn insert_insight_report(&mut self, report: &InsightReport) -> Result<i64> {
        self.conn.execute(
            r#"
            INSERT INTO insight_reports
              (period_start, period_end, generated_at, report_kind, model_provider, model_name,
               summary_text, category_mix_json, project_hints_json, evidence_count, error)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                report.period_start.to_rfc3339(),
                report.period_end.to_rfc3339(),
                report.generated_at.to_rfc3339(),
                &report.report_kind,
                &report.model_provider,
                &report.model_name,
                &report.summary_text,
                serde_json::to_string(&report.category_mix)?,
                serde_json::to_string(&report.project_hints)?,
                report.evidence_count as i64,
                report.error.as_deref(),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_insight_reports(&self, limit: usize) -> Result<Vec<InsightReport>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, period_start, period_end, generated_at, report_kind, model_provider,
                   model_name, summary_text, category_mix_json, project_hints_json,
                   evidence_count, error
            FROM insight_reports
            ORDER BY period_end DESC, id DESC
            LIMIT ?1
            "#,
        )?;

        let rows = statement.query_map(params![limit as i64], map_insight_report_row)?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn list_insight_reports_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        kind: Option<&str>,
        limit: usize,
    ) -> Result<Vec<InsightReport>> {
        let start = start.to_rfc3339();
        let end = end.to_rfc3339();
        let limit = limit as i64;
        let mut items = Vec::new();

        if let Some(kind) = kind {
            let mut statement = self.conn.prepare(
                r#"
                SELECT id, period_start, period_end, generated_at, report_kind, model_provider,
                       model_name, summary_text, category_mix_json, project_hints_json,
                       evidence_count, error
                FROM insight_reports
                WHERE period_start < ?2
                  AND period_end > ?1
                  AND report_kind = ?3
                ORDER BY period_start ASC, id ASC
                LIMIT ?4
                "#,
            )?;
            let rows =
                statement.query_map(params![start, end, kind, limit], map_insight_report_row)?;
            for row in rows {
                items.push(row?);
            }
        } else {
            let mut statement = self.conn.prepare(
                r#"
                SELECT id, period_start, period_end, generated_at, report_kind, model_provider,
                       model_name, summary_text, category_mix_json, project_hints_json,
                       evidence_count, error
                FROM insight_reports
                WHERE period_start < ?2
                  AND period_end > ?1
                ORDER BY period_start ASC, id ASC
                LIMIT ?3
                "#,
            )?;
            let rows = statement.query_map(params![start, end, limit], map_insight_report_row)?;
            for row in rows {
                items.push(row?);
            }
        }

        Ok(items)
    }

    pub fn insert_daily_brief(&mut self, brief: &DailyBrief) -> Result<i64> {
        if let Some(existing_id) = self
            .conn
            .query_row(
                "SELECT id FROM daily_briefs WHERE date = ?1 AND scheduled_for_local = ?2",
                params![&brief.date, &brief.scheduled_for_local],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
        {
            self.conn.execute(
                r#"
                UPDATE daily_briefs
                SET period_start = ?1,
                    period_end = ?2,
                    generated_at = ?3,
                    model_provider = ?4,
                    model_name = ?5,
                    prompt_version = ?6,
                    status = ?7,
                    descriptive_stats_json = ?8,
                    hourly_metrics_json = ?9,
                    comparison_json = ?10,
                    five_hour_report_ids_json = ?11,
                    daily_summary_text = ?12,
                    action_trajectory = ?13,
                    raw_summary_json = ?14,
                    error = ?15
                WHERE id = ?16
                "#,
                params![
                    brief.period_start.to_rfc3339(),
                    brief.period_end.to_rfc3339(),
                    brief.generated_at.to_rfc3339(),
                    &brief.model_provider,
                    &brief.model_name,
                    &brief.prompt_version,
                    &brief.status,
                    serde_json::to_string(&brief.descriptive_stats)?,
                    serde_json::to_string(&brief.hourly_metrics)?,
                    serde_json::to_string(&brief.comparison)?,
                    serde_json::to_string(&brief.five_hour_report_ids)?,
                    &brief.daily_summary_text,
                    &brief.action_trajectory,
                    serde_json::to_string(&brief.raw_summary_json)?,
                    brief.error.as_deref(),
                    existing_id,
                ],
            )?;
            return Ok(existing_id);
        }

        self.conn.execute(
            r#"
            INSERT INTO daily_briefs
              (date, period_start, period_end, generated_at, scheduled_for_local,
               model_provider, model_name, prompt_version, status, descriptive_stats_json,
               hourly_metrics_json, comparison_json, five_hour_report_ids_json,
               daily_summary_text, action_trajectory, raw_summary_json, error)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            "#,
            params![
                &brief.date,
                brief.period_start.to_rfc3339(),
                brief.period_end.to_rfc3339(),
                brief.generated_at.to_rfc3339(),
                &brief.scheduled_for_local,
                &brief.model_provider,
                &brief.model_name,
                &brief.prompt_version,
                &brief.status,
                serde_json::to_string(&brief.descriptive_stats)?,
                serde_json::to_string(&brief.hourly_metrics)?,
                serde_json::to_string(&brief.comparison)?,
                serde_json::to_string(&brief.five_hour_report_ids)?,
                &brief.daily_summary_text,
                &brief.action_trajectory,
                serde_json::to_string(&brief.raw_summary_json)?,
                brief.error.as_deref(),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn upsert_daily_brief_error(
        &mut self,
        mut brief: DailyBrief,
        error: impl Into<String>,
    ) -> Result<i64> {
        brief.status = "error".into();
        brief.error = Some(error.into());
        self.insert_daily_brief(&brief)
    }

    pub fn get_daily_brief_by_date(
        &self,
        date: &str,
        scheduled_for_local: &str,
    ) -> Result<Option<DailyBrief>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, date, period_start, period_end, generated_at, scheduled_for_local,
                   model_provider, model_name, prompt_version, status, descriptive_stats_json,
                   hourly_metrics_json, comparison_json, five_hour_report_ids_json,
                   daily_summary_text, action_trajectory, raw_summary_json, error
            FROM daily_briefs
            WHERE date = ?1 AND scheduled_for_local = ?2
            ORDER BY generated_at DESC, id DESC
            LIMIT 1
            "#,
        )?;
        statement
            .query_row(params![date, scheduled_for_local], map_daily_brief_row)
            .optional()
            .map_err(Into::into)
    }

    pub fn latest_daily_brief(&self) -> Result<Option<DailyBrief>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, date, period_start, period_end, generated_at, scheduled_for_local,
                   model_provider, model_name, prompt_version, status, descriptive_stats_json,
                   hourly_metrics_json, comparison_json, five_hour_report_ids_json,
                   daily_summary_text, action_trajectory, raw_summary_json, error
            FROM daily_briefs
            ORDER BY generated_at DESC, id DESC
            LIMIT 1
            "#,
        )?;
        statement
            .query_row([], map_daily_brief_row)
            .optional()
            .map_err(Into::into)
    }

    pub fn daily_brief_exists(&self, date: &str, scheduled_for_local: &str) -> Result<bool> {
        let exists = self
            .conn
            .query_row(
                "SELECT 1 FROM daily_briefs WHERE date = ?1 AND scheduled_for_local = ?2 LIMIT 1",
                params![date, scheduled_for_local],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        Ok(exists)
    }

    pub fn build_daily_activity_stats(
        &self,
        date: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        five_hour_reports: &[InsightReport],
    ) -> Result<DailyActivityStats> {
        let activity_end = end.min(Utc::now());
        let slices = self.window_activity_slices(start, activity_end)?;
        let mut app_seconds: HashMap<String, i64> = HashMap::new();
        let mut distinct_apps = HashSet::new();
        let mut first_activity_at: Option<DateTime<Utc>> = None;
        let mut last_activity_at: Option<DateTime<Utc>> = None;
        let mut active_seconds = 0;

        for slice in &slices {
            active_seconds += slice.seconds;
            distinct_apps.insert(slice.app.clone());
            *app_seconds.entry(slice.app.clone()).or_default() += slice.seconds;
            first_activity_at =
                Some(first_activity_at.map_or(slice.start, |value| value.min(slice.start)));
            last_activity_at =
                Some(last_activity_at.map_or(slice.end, |value| value.max(slice.end)));
        }

        let mut top_apps = app_seconds
            .into_iter()
            .map(|(process_name, seconds)| DailyAppActivity {
                process_name,
                active_seconds: seconds,
                share: if active_seconds > 0 {
                    seconds as f64 / active_seconds as f64
                } else {
                    0.0
                },
            })
            .collect::<Vec<_>>();
        top_apps.sort_by(|left, right| {
            right
                .active_seconds
                .cmp(&left.active_seconds)
                .then_with(|| left.process_name.cmp(&right.process_name))
        });
        top_apps.truncate(5);

        let screenshot_count =
            self.count_rows_between("screenshot_thumbnails", "captured_at", start, end)?;
        let high_res_screenshot_count =
            self.count_rows_between("high_res_screenshots", "captured_at", start, end)?;
        let visual_window_count =
            self.count_rows_between("visual_window_summaries", "window_start", start, end)?;
        let (input_events, input_chars) = self.input_counts_between(start, end)?;
        let category_mix = category_mix_from_reports(five_hour_reports);

        Ok(DailyActivityStats {
            date: date.into(),
            period_start: start,
            period_end: end,
            active_seconds,
            active_hours: active_seconds as f64 / 3600.0,
            window_event_count: self.count_window_events_between(start, end)?,
            switch_count: slices
                .windows(2)
                .filter(|pair| pair[0].app != pair[1].app)
                .count(),
            distinct_app_count: distinct_apps.len(),
            top_apps,
            category_mix,
            input_chars,
            input_events,
            screenshot_count,
            high_res_screenshot_count,
            visual_window_count,
            five_hour_report_count: five_hour_reports.len(),
            first_activity_at,
            last_activity_at,
        })
    }

    pub fn build_hourly_activity_metrics(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        five_hour_reports: &[InsightReport],
    ) -> Result<Vec<HourlyActivityMetric>> {
        let activity_end = end.min(Utc::now());
        let slices = self.window_activity_slices(start, activity_end)?;
        let mut metrics = Vec::with_capacity(24);
        for hour in 0..24 {
            let hour_start = start + chrono::Duration::hours(hour);
            let hour_end = (hour_start + chrono::Duration::hours(1)).min(end);
            let hour_slices = slices
                .iter()
                .filter(|slice| slice.start < hour_end && slice.end > hour_start)
                .collect::<Vec<_>>();
            let mut active_seconds = 0;
            let mut app_seconds: HashMap<String, i64> = HashMap::new();
            let mut distinct_apps = HashSet::new();
            for slice in &hour_slices {
                let overlap_start = slice.start.max(hour_start);
                let overlap_end = slice.end.min(hour_end);
                let seconds = (overlap_end - overlap_start).num_seconds().max(0);
                active_seconds += seconds;
                distinct_apps.insert(slice.app.clone());
                *app_seconds.entry(slice.app.clone()).or_default() += seconds;
            }
            let dominant_app = app_seconds
                .into_iter()
                .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
                .map(|(app, _)| app);
            let (_input_events, input_chars) = self.input_counts_between(hour_start, hour_end)?;
            let report_ids = five_hour_reports
                .iter()
                .filter(|report| report.period_start < hour_end && report.period_end > hour_start)
                .map(|report| report.id)
                .collect::<Vec<_>>();

            metrics.push(HourlyActivityMetric {
                hour: hour as u8,
                start_at: hour_start,
                end_at: hour_end,
                active_seconds: active_seconds.min(3600),
                active_ratio: (active_seconds.min(3600) as f64 / 3600.0).clamp(0.0, 1.0),
                window_event_count: self.count_window_events_between(hour_start, hour_end)?,
                switch_count: hour_slices
                    .windows(2)
                    .filter(|pair| pair[0].app != pair[1].app)
                    .count(),
                distinct_app_count: distinct_apps.len(),
                dominant_app,
                dominant_category: dominant_category_from_reports(five_hour_reports, &report_ids),
                input_chars,
                screenshot_count: self.count_rows_between(
                    "screenshot_thumbnails",
                    "captured_at",
                    hour_start,
                    hour_end,
                )?,
                high_res_screenshot_count: self.count_rows_between(
                    "high_res_screenshots",
                    "captured_at",
                    hour_start,
                    hour_end,
                )?,
                visual_window_count: self.count_rows_between(
                    "visual_window_summaries",
                    "window_start",
                    hour_start,
                    hour_end,
                )?,
                five_hour_report_ids: report_ids,
            });
        }
        Ok(metrics)
    }

    pub fn build_daily_comparison(
        &self,
        date: &str,
        stats: &DailyActivityStats,
    ) -> Result<DailyComparison> {
        Ok(DailyComparison {
            baseline_days: 0,
            compared_dates: Vec::new(),
            active_seconds_delta: stats.active_seconds,
            switches_per_hour_delta: if stats.active_hours > 0.0 {
                stats.switch_count as f64 / stats.active_hours
            } else {
                0.0
            },
            input_chars_delta: stats.input_chars as i64,
            screenshot_coverage_delta: stats.screenshot_count as f64,
            dominant_category_shift: stats
                .category_mix
                .first()
                .map(|category| format!("unknown -> {}", category.activity_category.as_str())),
            start_time_shift_minutes: None,
            end_time_shift_minutes: None,
            explanation: format!("{date} 暂无足够历史基线，当前显示当日指标本身。"),
        })
    }

    fn window_activity_slices(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<ActivitySlice>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT
              r.id AS raw_event_id,
              r.session_id,
              r.event_ts,
              w.hwnd,
              w.pid,
              w.process_name,
              w.exe_path_hash,
              w.window_title,
              w.capture_status,
              c.ended_at
            FROM raw_events r
            JOIN window_events w ON w.raw_event_id = r.id
            JOIN capture_sessions c ON c.id = r.session_id
            WHERE r.event_type = 'window_focus'
              AND r.event_ts < ?1
            ORDER BY r.session_id ASC, r.event_ts ASC, r.id ASC
            "#,
        )?;
        let rows = statement.query_map(params![end.to_rfc3339()], |row| {
            let event_ts: String = row.get(2)?;
            let capture_status: String = row.get(8)?;
            let session_ended_at: Option<String> = row.get(9)?;
            Ok((
                StoredWindowEvent {
                    raw_event_id: row.get(0)?,
                    session_id: row.get(1)?,
                    event_ts: parse_ts(&event_ts)?,
                    hwnd: row.get(3)?,
                    pid: row.get(4)?,
                    process_name: row.get(5)?,
                    exe_path_hash: row.get(6)?,
                    window_title: row.get(7)?,
                    capture_status: CaptureStatus::from_db(&capture_status),
                },
                session_ended_at.as_deref().map(parse_ts).transpose()?,
            ))
        })?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }

        let mut slices = Vec::new();
        for (index, (event, session_ended_at)) in events.iter().enumerate() {
            if event.capture_status != CaptureStatus::Ok {
                continue;
            }
            let next_at = events
                .iter()
                .skip(index + 1)
                .map(|(candidate, _)| candidate)
                .find(|candidate| candidate.session_id == event.session_id)
                .map(|candidate| candidate.event_ts);
            let inferred_end = match (next_at, *session_ended_at) {
                (Some(next_at), Some(ended_at)) => next_at.min(ended_at),
                (Some(next_at), None) => next_at,
                (None, Some(ended_at)) => ended_at,
                (None, None) => end,
            };
            let slice_start = event.event_ts.max(start);
            let slice_end = inferred_end.min(end);
            if slice_end > slice_start {
                slices.push(ActivitySlice {
                    start: slice_start,
                    end: slice_end,
                    seconds: (slice_end - slice_start).num_seconds().max(0),
                    app: event.process_name.clone(),
                });
            }
        }
        slices.sort_by(|left, right| {
            left.start
                .cmp(&right.start)
                .then_with(|| left.app.cmp(&right.app))
        });
        Ok(slices)
    }

    fn count_window_events_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            r#"
            SELECT COUNT(*)
            FROM raw_events r
            JOIN window_events w ON w.raw_event_id = r.id
            WHERE r.event_type = 'window_focus'
              AND r.event_ts >= ?1
              AND r.event_ts < ?2
              AND w.capture_status = 'ok'
            "#,
            params![start.to_rfc3339(), end.to_rfc3339()],
            |row| row.get(0),
        )?;
        Ok(count.max(0) as usize)
    }

    fn count_rows_between(
        &self,
        table: &str,
        column: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<usize> {
        let sql = match (table, column) {
            ("screenshot_thumbnails", "captured_at") => {
                "SELECT COUNT(*) FROM screenshot_thumbnails WHERE captured_at >= ?1 AND captured_at < ?2 AND capture_status = 'ok'"
            }
            ("high_res_screenshots", "captured_at") => {
                "SELECT COUNT(*) FROM high_res_screenshots WHERE captured_at >= ?1 AND captured_at < ?2 AND capture_status = 'ok'"
            }
            ("visual_window_summaries", "window_start") => {
                "SELECT COUNT(*) FROM visual_window_summaries WHERE window_start >= ?1 AND window_start < ?2 AND error IS NULL"
            }
            _ => return Ok(0),
        };
        let count: i64 =
            self.conn
                .query_row(sql, params![start.to_rfc3339(), end.to_rfc3339()], |row| {
                    row.get(0)
                })?;
        Ok(count.max(0) as usize)
    }

    fn input_counts_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<(usize, usize)> {
        let (events, chars): (i64, i64) = self.conn.query_row(
            r#"
            SELECT COUNT(*),
                   COALESCE(SUM(CASE WHEN character IS NULL THEN 0 ELSE length(character) END), 0)
            FROM input_events
            WHERE event_ts >= ?1
              AND event_ts < ?2
            "#,
            params![start.to_rfc3339(), end.to_rfc3339()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok((events.max(0) as usize, chars.max(0) as usize))
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

    pub fn list_expirable_screenshot_files(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<StoredScreenshotFile>> {
        let mut files = Vec::new();
        self.collect_expirable_screenshot_files(
            ScreenshotStoreKind::Thumbnail,
            cutoff,
            &mut files,
        )?;
        self.collect_expirable_screenshot_files(ScreenshotStoreKind::HighRes, cutoff, &mut files)?;
        Ok(files)
    }

    fn collect_expirable_screenshot_files(
        &self,
        kind: ScreenshotStoreKind,
        cutoff: DateTime<Utc>,
        files: &mut Vec<StoredScreenshotFile>,
    ) -> Result<()> {
        let sql = format!(
            r#"
            SELECT id, captured_at, file_path, file_size_bytes
            FROM {}
            WHERE captured_at < ?1
              AND capture_status = 'ok'
              AND file_path <> ''
            ORDER BY captured_at ASC, id ASC
            "#,
            kind.table_name()
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![cutoff.to_rfc3339()], |row| {
            let captured_at: String = row.get(1)?;
            let file_size_bytes: i64 = row.get(3)?;
            Ok(StoredScreenshotFile {
                kind,
                id: row.get(0)?,
                captured_at: parse_ts(&captured_at)?,
                file_path: row.get(2)?,
                file_size_bytes: file_size_bytes.max(0) as u64,
            })
        })?;
        for row in rows {
            files.push(row?);
        }
        Ok(())
    }

    pub fn mark_screenshot_file_expired(
        &mut self,
        kind: ScreenshotStoreKind,
        id: i64,
        expired_at: DateTime<Utc>,
    ) -> Result<()> {
        let sql = format!(
            "UPDATE {} SET capture_status = 'expired', expired_at = ?1 WHERE id = ?2",
            kind.table_name()
        );
        self.conn
            .execute(&sql, params![expired_at.to_rfc3339(), id])?;
        Ok(())
    }

    pub fn get_db_stats(&self) -> Result<crate::models::DbStats> {
        self.get_db_stats_with_retention(30)
    }

    pub fn get_db_stats_with_retention(
        &self,
        retention_days: u32,
    ) -> Result<crate::models::DbStats> {
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
        let high_res_screenshots: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM high_res_screenshots WHERE capture_status = 'ok'",
            [],
            |r| r.get(0),
        )?;
        let blocker_hits: usize =
            self.conn
                .query_row("SELECT COUNT(*) FROM blocker_hits", [], |r| r.get(0))?;
        let expired_thumbnail_files: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM screenshot_thumbnails WHERE capture_status = 'expired'",
            [],
            |r| r.get(0),
        )?;
        let expired_high_res_files: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM high_res_screenshots WHERE capture_status = 'expired'",
            [],
            |r| r.get(0),
        )?;
        let active_thumbnail_bytes: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(file_size_bytes), 0) FROM screenshot_thumbnails WHERE capture_status = 'ok'",
            [],
            |r| r.get(0),
        )?;
        let active_high_res_bytes: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(file_size_bytes), 0) FROM high_res_screenshots WHERE capture_status = 'ok'",
            [],
            |r| r.get(0),
        )?;
        let expired_thumbnail_bytes: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(file_size_bytes), 0) FROM screenshot_thumbnails WHERE capture_status = 'expired'",
            [],
            |r| r.get(0),
        )?;
        let expired_high_res_bytes: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(file_size_bytes), 0) FROM high_res_screenshots WHERE capture_status = 'expired'",
            [],
            |r| r.get(0),
        )?;
        let active_files = screenshots + high_res_screenshots;
        let pending_google_drive_upload = active_files > 0;
        let google_drive_message = pending_google_drive_upload.then(|| {
            format!(
                "Local screenshots are temporary for {retention_days} days. Upload older evidence to Google Drive before cleanup."
            )
        });

        Ok(crate::models::DbStats {
            window_events,
            lifecycle_events,
            input_events,
            text_segments,
            screenshots,
            high_res_screenshots,
            blocker_hits,
            image_retention: ImageRetentionStats {
                retention_days,
                active_files,
                expired_files: expired_thumbnail_files + expired_high_res_files,
                active_bytes: (active_thumbnail_bytes + active_high_res_bytes).max(0) as u64,
                expired_bytes: (expired_thumbnail_bytes + expired_high_res_bytes).max(0) as u64,
                pending_google_drive_upload,
                google_drive_message,
            },
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
        identity_tags: unknown_visual_tags(),
        routine_tags: unknown_visual_tags(),
        visible_apps: parse_string_vec(&visible_apps_json)?,
        visible_text_hints: parse_string_vec(&visible_text_hints_json)?,
        risk_flags: parse_string_vec(&risk_flags_json)?,
        confidence: row.get(12)?,
        created_at: parse_ts(&created_at)?,
        error: row.get(14)?,
    })
}

fn map_visual_observation_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<VisualObservation> {
    let captured_at: String = row.get(2)?;
    let activity_category: String = row.get(8)?;
    let project_hints_json: String = row.get(9)?;
    let visible_apps_json: String = row.get(10)?;
    let visible_text_hints_json: String = row.get(11)?;
    let risk_flags_json: String = row.get(12)?;
    let created_at: String = row.get(14)?;
    let activity_category = ActivityCategory::from_db(&activity_category).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            8,
            Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown activity_category: {activity_category}"),
            )),
        )
    })?;

    Ok(VisualObservation {
        id: row.get(0)?,
        high_res_screenshot_id: row.get(1)?,
        captured_at: parse_ts(&captured_at)?,
        file_path: row.get(3)?,
        model_provider: row.get(4)?,
        model_name: row.get(5)?,
        prompt_version: row.get(6)?,
        summary_text: row.get(7)?,
        activity_category,
        project_hints: parse_string_vec(&project_hints_json)?,
        identity_tags: unknown_visual_tags(),
        routine_tags: unknown_visual_tags(),
        visible_apps: parse_string_vec(&visible_apps_json)?,
        visible_text_hints: parse_string_vec(&visible_text_hints_json)?,
        risk_flags: parse_string_vec(&risk_flags_json)?,
        confidence: row.get(13)?,
        created_at: parse_ts(&created_at)?,
        error: row.get(15)?,
    })
}

fn map_visual_window_summary_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<VisualWindowSummary> {
    let window_start: String = row.get(1)?;
    let window_end: String = row.get(2)?;
    let sampled_screenshot_ids_json: String = row.get(3)?;
    let primary_activity: String = row.get(10)?;
    let project_hints_json: String = row.get(11)?;
    let trajectory_json: String = row.get(13)?;
    let visible_apps_json: String = row.get(18)?;
    let visible_text_hints_json: String = row.get(19)?;
    let risk_flags_json: String = row.get(20)?;
    let raw_summary_json: String = row.get(22)?;
    let created_at: String = row.get(23)?;
    let primary_activity = ActivityCategory::from_db(&primary_activity).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            10,
            Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown primary_activity: {primary_activity}"),
            )),
        )
    })?;

    Ok(VisualWindowSummary {
        id: row.get(0)?,
        window_start: parse_ts(&window_start)?,
        window_end: parse_ts(&window_end)?,
        sampled_screenshot_ids: parse_i64_vec(&sampled_screenshot_ids_json)?,
        previous_summary_id: row.get(4)?,
        model_provider: row.get(5)?,
        model_name: row.get(6)?,
        prompt_version: row.get(7)?,
        summary_text: row.get(8)?,
        continuity: row.get(9)?,
        primary_activity,
        project_hints: parse_string_vec(&project_hints_json)?,
        identity_tags: unknown_visual_tags(),
        routine_tags: unknown_visual_tags(),
        task_intent: row.get(12)?,
        trajectory: parse_visual_trajectory(&trajectory_json)?,
        switching_level: row.get(14)?,
        switching_evidence: row.get(15)?,
        loafing_level: row.get(16)?,
        loafing_evidence: row.get(17)?,
        visible_apps: parse_string_vec(&visible_apps_json)?,
        visible_text_hints: parse_string_vec(&visible_text_hints_json)?,
        risk_flags: parse_string_vec(&risk_flags_json)?,
        confidence: row.get(21)?,
        raw_summary_json: parse_json(&raw_summary_json)?,
        created_at: parse_ts(&created_at)?,
        error: row.get(24)?,
    })
}

fn map_insight_report_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<InsightReport> {
    let period_start: String = row.get(1)?;
    let period_end: String = row.get(2)?;
    let generated_at: String = row.get(3)?;
    let category_mix_json: String = row.get(8)?;
    let project_hints_json: String = row.get(9)?;
    let evidence_count: i64 = row.get(10)?;

    Ok(InsightReport {
        id: row.get(0)?,
        period_start: parse_ts(&period_start)?,
        period_end: parse_ts(&period_end)?,
        generated_at: parse_ts(&generated_at)?,
        report_kind: row.get(4)?,
        model_provider: row.get(5)?,
        model_name: row.get(6)?,
        summary_text: row.get(7)?,
        category_mix: parse_category_mix(&category_mix_json)?,
        project_hints: parse_string_vec(&project_hints_json)?,
        evidence_count: evidence_count.max(0) as usize,
        error: row.get(11)?,
    })
}

fn map_daily_brief_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DailyBrief> {
    let period_start: String = row.get(2)?;
    let period_end: String = row.get(3)?;
    let generated_at: String = row.get(4)?;
    let descriptive_stats_json: String = row.get(10)?;
    let hourly_metrics_json: String = row.get(11)?;
    let comparison_json: String = row.get(12)?;
    let five_hour_report_ids_json: String = row.get(13)?;
    let raw_summary_json: String = row.get(16)?;

    Ok(DailyBrief {
        id: row.get(0)?,
        date: row.get(1)?,
        period_start: parse_ts(&period_start)?,
        period_end: parse_ts(&period_end)?,
        generated_at: parse_ts(&generated_at)?,
        scheduled_for_local: row.get(5)?,
        model_provider: row.get(6)?,
        model_name: row.get(7)?,
        prompt_version: row.get(8)?,
        status: row.get(9)?,
        descriptive_stats: parse_daily_activity_stats(&descriptive_stats_json)?,
        hourly_metrics: parse_hourly_activity_metrics(&hourly_metrics_json)?,
        comparison: parse_daily_comparison(&comparison_json)?,
        five_hour_report_ids: parse_i64_vec(&five_hour_report_ids_json)?,
        daily_summary_text: row.get(14)?,
        action_trajectory: row.get(15)?,
        raw_summary_json: parse_json(&raw_summary_json)?,
        error: row.get(17)?,
    })
}

fn category_mix_from_reports(reports: &[InsightReport]) -> Vec<ActivityCategoryCount> {
    let mut counts: Vec<ActivityCategoryCount> = Vec::new();
    for report in reports {
        for item in &report.category_mix {
            if let Some(existing) = counts
                .iter_mut()
                .find(|existing| existing.activity_category == item.activity_category)
            {
                existing.count += item.count;
            } else {
                counts.push(item.clone());
            }
        }
    }
    counts.sort_by(|left, right| {
        right.count.cmp(&left.count).then_with(|| {
            left.activity_category
                .as_str()
                .cmp(right.activity_category.as_str())
        })
    });
    counts
}

fn dominant_category_from_reports(
    reports: &[InsightReport],
    report_ids: &[i64],
) -> ActivityCategory {
    let filtered = reports
        .iter()
        .filter(|report| report_ids.contains(&report.id))
        .cloned()
        .collect::<Vec<_>>();
    category_mix_from_reports(&filtered)
        .first()
        .map(|item| item.activity_category.clone())
        .unwrap_or(ActivityCategory::Unknown)
}

pub(crate) fn parse_ts(value: &str) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))
}

fn utc_day_bounds(date: &str) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d")?;
    let start = date
        .and_hms_opt(0, 0, 0)
        .expect("midnight is valid")
        .and_utc();
    let end = date
        .succ_opt()
        .expect("date successor is valid")
        .and_hms_opt(0, 0, 0)
        .expect("midnight is valid")
        .and_utc();
    Ok((start, end))
}

fn parse_json(value: &str) -> rusqlite::Result<serde_json::Value> {
    serde_json::from_str(value)
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err)))
}

fn parse_string_vec(value: &str) -> rusqlite::Result<Vec<String>> {
    serde_json::from_str(value)
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err)))
}

fn parse_i64_vec(value: &str) -> rusqlite::Result<Vec<i64>> {
    serde_json::from_str(value)
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err)))
}

fn parse_visual_trajectory(value: &str) -> rusqlite::Result<Vec<VisualTrajectoryPoint>> {
    serde_json::from_str(value)
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err)))
}

fn unknown_visual_tags() -> Vec<String> {
    vec!["unknown".to_string()]
}

fn parse_category_mix(value: &str) -> rusqlite::Result<Vec<ActivityCategoryCount>> {
    serde_json::from_str(value)
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err)))
}

fn parse_daily_activity_stats(value: &str) -> rusqlite::Result<DailyActivityStats> {
    serde_json::from_str(value)
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err)))
}

fn parse_hourly_activity_metrics(value: &str) -> rusqlite::Result<Vec<HourlyActivityMetric>> {
    serde_json::from_str(value)
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err)))
}

fn parse_daily_comparison(value: &str) -> rusqlite::Result<DailyComparison> {
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
