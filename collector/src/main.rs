use std::{net::SocketAddr, path::PathBuf, time::Duration};

use anyhow::Result;
use clap::{Parser, Subcommand};
use tokio::time;
use tsr_collector::{api, storage::Store, window::sample_foreground_window};

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
    },
}

#[tokio::main]
async fn main() -> Result<()> {
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
            let store = Store::open(db)?;
            store.init()?;
            let session_id = store.create_session(env!("CARGO_PKG_VERSION"), "default")?;
            let mut store = store;
            record_for(&mut store, &session_id, seconds, poll_ms).await?;
        }
        Command::Serve { db, addr, poll_ms } => {
            ensure_poll_ms(poll_ms)?;
            let store = Store::open(db)?;
            store.init()?;
            api::serve(store, addr, poll_ms).await?;
        }
    }

    Ok(())
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
