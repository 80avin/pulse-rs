use clap::{Args, Subcommand};
use pulse_core::PulseCore;

use crate::output::{print_json, relative_time};

#[derive(Debug, Args)]
pub struct SyncArgs {
    #[command(subcommand)]
    pub command: SyncCommand2,
}

#[derive(Debug, Subcommand)]
pub enum SyncCommand2 {
    /// Trigger a sync (blocking by default)
    Run(SyncRunArgs),
    /// Show sync status for all feeds
    Status(SyncStatusArgs),
}

#[derive(Debug, Args)]
pub struct SyncRunArgs {
    /// Sync only a specific feed
    #[arg(long)]
    pub feed: Option<String>,
    /// Fire and forget (return immediately)
    #[arg(long)]
    pub detach: bool,
    /// Clear ETag and last_seen_id before syncing, forcing a full re-fetch
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct SyncStatusArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: SyncArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    match args.command {
        SyncCommand2::Run(a) => cmd_run(a, core).await,
        SyncCommand2::Status(a) => cmd_status(a, core, global_json).await,
    }
}

async fn cmd_run(args: SyncRunArgs, core: &PulseCore) -> anyhow::Result<()> {
    if args.detach {
        // `sync run --detach` cannot just spawn an in-process task: main() calls
        // core.shutdown() on exit, which aborts it before it can run. Spawn a
        // detached child process that performs the blocking sync instead.
        return cmd_detach(core, args.feed.as_deref(), args.force);
    }

    let feeds = core.get_feeds().await?;
    let targets: Vec<_> = if let Some(ref fid) = args.feed {
        feeds
            .iter()
            .filter(|f| f.id == *fid || f.id.starts_with(fid.as_str()))
            .collect()
    } else {
        feeds.iter().filter(|f| f.is_enabled).collect()
    };

    if targets.is_empty() {
        eprintln!("no feeds to sync");
        return Ok(());
    }

    let mut total = 0usize;
    let mut failures = 0usize;
    for feed in &targets {
        let title = feed.title.as_deref().unwrap_or(&feed.url);
        if args.force
            && let Err(e) = core.clear_feed_cache(&feed.id).await
        {
            eprintln!("warn: could not clear cache for '{}': {}", title, e);        }
        eprint!("syncing '{}'...", title);
        match core.sync_feed(&feed.id).await {
            Ok(n) => {
                eprintln!(" {} new items", n);
                total += n;
            }
            Err(e) => {
                eprintln!(" error: {}", e);
                failures += 1;
            }
        }
    }

    if targets.len() > 1 {
        eprintln!("total: {} new items across {} feeds", total, targets.len());
    }
    if failures > 0 {
        anyhow::bail!("{} of {} feeds failed to sync", failures, targets.len());
    }
    Ok(())
}

/// Spawn a detached child process running the blocking sync, so the parent can
/// exit (main() aborts in-process tasks on shutdown).
fn cmd_detach(core: &PulseCore, feed: Option<&str>, force: bool) -> anyhow::Result<()> {
    use std::process::{Command, Stdio};

    let mut cmd = Command::new(std::env::current_exe()?);
    let custom_db = core.config.db_path != core.config.data_dir.join("pulse.db");
    if custom_db {
        cmd.arg("--db").arg(&core.config.db_path);
    } else {
        cmd.arg("--data-dir").arg(&core.config.data_dir);
    }
    cmd.arg("sync").arg("run");
    if let Some(f) = feed {
        cmd.arg("--feed").arg(f);
    }
    if force {
        cmd.arg("--force");
    }
    cmd.stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
    eprintln!("sync started in background");
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct SyncStatus {
    id: String,
    title: Option<String>,
    next_fetch_at: Option<i64>,
    last_success_at: Option<i64>,
    failure_streak: i64,
    is_enabled: bool,
}

async fn cmd_status(
    args: SyncStatusArgs,
    core: &PulseCore,
    global_json: bool,
) -> anyhow::Result<()> {
    let use_json = args.json || global_json;
    let feeds = core.get_feeds().await?;

    let statuses: Vec<SyncStatus> = feeds
        .iter()
        .map(|f| SyncStatus {
            id: f.id[..f.id.len().min(8)].to_string(),
            title: f.title.clone(),
            next_fetch_at: f.next_fetch_at,
            last_success_at: f.last_success_at,
            failure_streak: f.failure_streak,
            is_enabled: f.is_enabled,
        })
        .collect();

    if use_json {
        print_json(&statuses);
        return Ok(());
    }

    let now = chrono::Utc::now().timestamp();
    println!(
        "{:<8}  {:<28}  {:<12}  {:<12}  {:<4}  ENABLED",
        "ID", "TITLE", "NEXT_SYNC", "LAST_SUCCESS", "FAIL"
    );
    for s in &statuses {
        let title = s.title.as_deref().unwrap_or("-");
        let title_trunc = crate::output::truncate_chars(title, 28);
        let next = s
            .next_fetch_at
            .map(|ts| {
                let secs = ts - now;
                if secs <= 0 {
                    "now".to_string()
                } else {
                    format!("in {}s", secs)
                }
            })
            .unwrap_or_else(|| "-".to_string());
        let last = s
            .last_success_at
            .map(relative_time)
            .unwrap_or_else(|| "never".to_string());
        println!(
            "{:<8}  {:<28}  {:<12}  {:<12}  {:<4}  {}",
            s.id,
            title_trunc,
            next,
            last,
            s.failure_streak,
            if s.is_enabled { "yes" } else { "no" }
        );
    }
    Ok(())
}
