use std::{net::SocketAddr, path::PathBuf, time::Duration};

use anyhow::Result;
use clap::{Parser, Subcommand};
use tokio::time;
use tsr_collector::{
    api, models::LifecycleType, notion_smoke::run_notion_daily_archive_smoke, storage::Store,
    window::sample_foreground_window,
};

#[derive(Debug, Parser)]
#[command(name = "tsr-collector")]
#[command(about = "Time State Recorder Windows collector")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    SampleOnce,
    Record {
        #[arg(long, default_value = "data/local.sqlite3")]
        db: PathBuf,
        #[arg(long, default_value_t = 30)]
        seconds: u64,
        #[arg(long, default_value_t = 1000)]
        poll_ms: u64,
    },
    Serve {
        #[arg(long, default_value = "data/local.sqlite3")]
        db: PathBuf,
        #[arg(long, default_value = "127.0.0.1:4317")]
        addr: SocketAddr,
        #[arg(long, default_value_t = 1000)]
        poll_ms: u64,
        #[arg(long, default_value = "blocker_config.json")]
        blocker_config: PathBuf,
    },
    NotionDailyArchiveSmoke {
        #[arg(long, default_value = "reports/notion-daily-archive-smoke.json")]
        artifact: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    load_local_env();
    let cli = Cli::parse();

    match cli.command {
        Command::SampleOnce => {
            let snapshot = sample_foreground_window()?;
            println!("{}", serde_json::to_string_pretty(&snapshot)?);
        }
        Command::Record {
            db,
            seconds,
            poll_ms,
        } => {
            ensure_poll_ms(poll_ms)?;
            let mut store = Store::open(db)?;
            store.init()?;
            let now = chrono::Utc::now();
            store.close_stale_sessions(now, "abnormal_stop")?;
            let session_id = store.create_session(env!("CARGO_PKG_VERSION"), "default")?;
            store.insert_lifecycle_event(
                &session_id,
                now,
                LifecycleType::SessionStart,
                None,
                serde_json::json!({ "appVersion": env!("CARGO_PKG_VERSION") }),
            )?;
            record_for(&mut store, &session_id, seconds, poll_ms).await?;
            store.close_session(&session_id, chrono::Utc::now(), "completed")?;
        }
        Command::Serve {
            db,
            addr,
            poll_ms,
            blocker_config,
        } => {
            ensure_poll_ms(poll_ms)?;
            let store = Store::open(db)?;
            store.init()?;
            api::serve(store, addr, poll_ms, Some(blocker_config)).await?;
        }
        Command::NotionDailyArchiveSmoke { artifact } => {
            let summary = run_notion_daily_archive_smoke(&artifact).await?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
    }

    Ok(())
}

fn load_local_env() {
    let _ = dotenvy::from_filename(".env.local");
}

fn ensure_poll_ms(poll_ms: u64) -> Result<()> {
    anyhow::ensure!(poll_ms >= 100, "--poll-ms must be at least 100");
    Ok(())
}

async fn record_for(store: &mut Store, session_id: &str, seconds: u64, poll_ms: u64) -> Result<()> {
    let deadline = time::Instant::now() + Duration::from_secs(seconds);
    let mut last_identity: Option<(i64, u32, Option<String>)> = None;

    while time::Instant::now() < deadline {
        let snapshot = sample_foreground_window()?;
        let identity = (snapshot.hwnd, snapshot.pid, snapshot.window_title.clone());

        if last_identity.as_ref() != Some(&identity) {
            store.insert_window_focus(session_id, &snapshot)?;
            last_identity = Some(identity);
        }

        time::sleep(Duration::from_millis(poll_ms)).await;
    }

    Ok(())
}
