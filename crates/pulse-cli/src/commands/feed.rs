use clap::{Args, Subcommand};
use pulse_core::{PulseCore, types::{Feed, FeedType, FeedGroup}};
use uuid::Uuid;

use crate::output::{print_json, print_error, relative_time, confirm};

#[derive(Debug, Args)]
pub struct FeedArgs {
    #[command(subcommand)]
    pub command: FeedCommand,
}

#[derive(Debug, Subcommand)]
pub enum FeedCommand {
    /// Add a new feed
    Add(FeedAddArgs),
    /// List feeds
    List(FeedListArgs),
    /// Show details for one feed
    Show(FeedShowArgs),
    /// Remove a feed and all its items
    Remove(FeedRemoveArgs),
    /// Enable a feed
    Enable(FeedIdArgs),
    /// Disable a feed
    Disable(FeedIdArgs),
    /// Edit feed settings
    Edit(FeedEditArgs),
    /// Show health metrics for feeds
    Health(FeedHealthArgs),
}

#[derive(Debug, Args)]
pub struct FeedAddArgs {
    /// Feed URL (or HN section name like "topstories", "askhn")
    pub url: String,
    /// Force feed type (auto-detected if omitted)
    #[arg(long)]
    pub r#type: Option<String>,
    /// Add to group (by name)
    #[arg(long)]
    pub group: Option<String>,
    /// Override feed title
    #[arg(long)]
    pub name: Option<String>,
    /// Poll interval in seconds
    #[arg(long)]
    pub interval: Option<i64>,
}

#[derive(Debug, Args)]
pub struct FeedListArgs {
    /// Filter by group name
    #[arg(long)]
    pub group: Option<String>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct FeedShowArgs {
    /// Feed ID
    pub id: String,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct FeedRemoveArgs {
    /// Feed ID
    pub id: String,
    /// Skip confirmation prompt
    #[arg(long)]
    pub yes: bool,
}

#[derive(Debug, Args)]
pub struct FeedIdArgs {
    /// Feed ID
    pub id: String,
}

#[derive(Debug, Args)]
pub struct FeedEditArgs {
    /// Feed ID
    pub id: String,
    /// Change the fetch URL
    #[arg(long)]
    pub url: Option<String>,
    /// Set poll interval in seconds
    #[arg(long)]
    pub interval: Option<i64>,
    /// Move to group by name
    #[arg(long)]
    pub group: Option<String>,
    /// Override feed title
    #[arg(long)]
    pub name: Option<String>,
}

#[derive(Debug, Args)]
pub struct FeedHealthArgs {
    /// Feed ID (omit for all feeds)
    pub id: Option<String>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Auto-detect feed type from URL or input string.
fn detect_feed_type(url: &str) -> FeedType {
    let lower = url.to_lowercase();
    if lower.contains("reddit.com") || lower.starts_with("r/") {
        FeedType::Reddit
    } else if lower == "topstories"
        || lower == "newstories"
        || lower == "askhn"
        || lower == "showhn"
        || lower == "beststories"
        || lower == "jobstories"
        || lower.starts_with("hn:")
    {
        FeedType::Hn
    } else {
        FeedType::Rss
    }
}

/// Resolve a group name to its ID, optionally creating it.
async fn resolve_or_create_group(core: &PulseCore, name: &str) -> anyhow::Result<String> {
    let groups = core.get_feed_groups().await?;
    if let Some(g) = groups.iter().find(|g| g.name.eq_ignore_ascii_case(name)) {
        return Ok(g.id.clone());
    }
    // Create it
    let now = chrono::Utc::now().timestamp();
    let group = FeedGroup {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        description: None,
        color: None,
        sort_order: 0,
        created_at: now,
        updated_at: now,
    };
    let gid = group.id.clone();
    core.db.insert_feed_group(group).await?;
    Ok(gid)
}

/// Build the default poll interval based on feed type.
fn default_interval(feed_type: &FeedType) -> i64 {
    match feed_type {
        FeedType::Hn => 900,      // 15 min
        FeedType::Reddit => 1200, // 20 min
        FeedType::Rss => 3600,    // 60 min
    }
}

pub async fn run(args: FeedArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    match args.command {
        FeedCommand::Add(a) => cmd_add(a, core).await,
        FeedCommand::List(a) => cmd_list(a, core, global_json).await,
        FeedCommand::Show(a) => cmd_show(a, core, global_json).await,
        FeedCommand::Remove(a) => cmd_remove(a, core).await,
        FeedCommand::Enable(a) => cmd_enable(a, core, true).await,
        FeedCommand::Disable(a) => cmd_enable(a, core, false).await,
        FeedCommand::Edit(a) => cmd_edit(a, core).await,
        FeedCommand::Health(a) => cmd_health(a, core, global_json).await,
    }
}

async fn cmd_add(args: FeedAddArgs, core: &PulseCore) -> anyhow::Result<()> {
    let feed_type = if let Some(ref t) = args.r#type {
        t.parse::<FeedType>().map_err(|e| anyhow::anyhow!(e))?
    } else {
        detect_feed_type(&args.url)
    };

    let group_id = if let Some(ref g) = args.group {
        Some(resolve_or_create_group(core, g).await?)
    } else {
        None
    };

    let interval = args.interval.unwrap_or_else(|| default_interval(&feed_type));
    let now = chrono::Utc::now().timestamp();

    let feed = Feed {
        id: Uuid::new_v4().to_string(),
        url: args.url.clone(),
        feed_type: feed_type.clone(),
        title: args.name,
        description: None,
        site_url: None,
        icon_url: None,
        group_id,
        poll_interval_secs: interval,
        is_enabled: true,
        etag: None,
        last_modified: None,
        last_fetched_at: None,
        last_success_at: None,
        last_item_at: None,
        failure_streak: 0,
        total_fetches: 0,
        total_failures: 0,
        avg_latency_ms: None,
        next_fetch_at: Some(now),
        source_config: serde_json::json!({}),
        language: None,
        created_at: now,
        updated_at: now,
    };

    let feed_id = feed.id.clone();
    core.add_feed(feed).await?;
    println!("added feed {} (type: {})", &feed_id[..8], feed_type);
    Ok(())
}

async fn cmd_list(args: FeedListArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    let use_json = args.json || global_json;
    let feeds = core.get_feeds().await?;
    let groups = core.get_feed_groups().await?;

    // Filter by group name if requested
    let filtered: Vec<_> = if let Some(ref gname) = args.group {
        let gid = groups.iter().find(|g| g.name.eq_ignore_ascii_case(gname)).map(|g| g.id.clone());
        match gid {
            Some(ref id) => feeds.iter().filter(|f| f.group_id.as_deref() == Some(id.as_str())).collect(),
            None => {
                print_error(&format!("group '{}' not found", gname));
                return Ok(());
            }
        }
    } else {
        feeds.iter().collect()
    };

    if use_json {
        // Enrich with group_name
        let enriched: Vec<serde_json::Value> = filtered.iter().map(|f| {
            let group_name = f.group_id.as_ref()
                .and_then(|gid| groups.iter().find(|g| &g.id == gid))
                .map(|g| g.name.clone());
            let success_rate = if f.total_fetches > 0 {
                Some((f.total_fetches - f.total_failures) as f64 / f.total_fetches as f64 * 100.0)
            } else {
                None
            };
            serde_json::json!({
                "id": f.id,
                "url": f.url,
                "feed_type": f.feed_type,
                "title": f.title,
                "group_id": f.group_id,
                "group_name": group_name,
                "poll_interval_secs": f.poll_interval_secs,
                "failure_streak": f.failure_streak,
                "success_rate_pct": success_rate,
                "avg_latency_ms": f.avg_latency_ms,
                "last_success_at": f.last_success_at,
                "last_item_at": f.last_item_at,
                "is_enabled": f.is_enabled,
            })
        }).collect();
        print_json(&enriched);
        return Ok(());
    }

    // Human-readable table
    println!("{:<8}  {:<6}  {:<32}  {:<12}  {:<8}  {:<6}  {}",
        "ID", "TYPE", "TITLE", "GROUP", "INTERVAL", "HEALTH", "LAST SYNC");
    for f in &filtered {
        let id_prefix = &f.id[..f.id.len().min(8)];
        let feed_type = f.feed_type.as_str();
        let title = f.title.as_deref().unwrap_or(&f.url);
        let title_trunc = if title.len() > 32 { &title[..32] } else { title };
        let group_name = f.group_id.as_ref()
            .and_then(|gid| groups.iter().find(|g| &g.id == gid))
            .map(|g| g.name.as_str())
            .unwrap_or("-");
        let interval_min = f.poll_interval_secs / 60;
        let interval_str = format!("{}m", interval_min);

        let health_str = if f.total_fetches == 0 {
            "- new".to_string()
        } else {
            let rate = (f.total_fetches - f.total_failures) as f64 / f.total_fetches as f64 * 100.0;
            if f.failure_streak > 0 {
                format!("✗ {:.0}%", rate)
            } else {
                format!("✓ {:.0}%", rate)
            }
        };

        let last_sync = f.last_success_at
            .map(|ts| relative_time(ts))
            .unwrap_or_else(|| "never".to_string());

        println!("{:<8}  {:<6}  {:<32}  {:<12}  {:<8}  {:<8}  {}",
            id_prefix, feed_type, title_trunc, group_name, interval_str, health_str, last_sync);
    }
    Ok(())
}

async fn cmd_show(args: FeedShowArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    let use_json = args.json || global_json;
    let feed = core.get_feed(&args.id).await.map_err(|e| {
        print_error(&format!("feed not found: {}", args.id));
        e
    })?;

    if use_json {
        print_json(&feed);
        return Ok(());
    }

    let groups = core.get_feed_groups().await?;
    let group_name = feed.group_id.as_ref()
        .and_then(|gid| groups.iter().find(|g| &g.id == gid))
        .map(|g| g.name.as_str())
        .unwrap_or("-");

    println!("ID:           {}", feed.id);
    println!("URL:          {}", feed.url);
    println!("Type:         {}", feed.feed_type);
    println!("Title:        {}", feed.title.as_deref().unwrap_or("-"));
    println!("Description:  {}", feed.description.as_deref().unwrap_or("-"));
    println!("Group:        {}", group_name);
    println!("Interval:     {}s", feed.poll_interval_secs);
    println!("Enabled:      {}", feed.is_enabled);
    println!("Total fetches: {}", feed.total_fetches);
    println!("Total failures: {}", feed.total_failures);
    println!("Failure streak: {}", feed.failure_streak);
    if let Some(lat) = feed.avg_latency_ms {
        println!("Avg latency:  {:.0}ms", lat);
    }
    if let Some(ts) = feed.last_success_at {
        println!("Last success: {} ago", relative_time(ts));
    }
    if let Some(ts) = feed.next_fetch_at {
        let now = chrono::Utc::now().timestamp();
        let secs = ts - now;
        if secs <= 0 {
            println!("Next sync:    now");
        } else {
            println!("Next sync:    in {}s", secs);
        }
    }
    Ok(())
}

async fn cmd_remove(args: FeedRemoveArgs, core: &PulseCore) -> anyhow::Result<()> {
    if !args.yes && !confirm(&format!("Remove feed {} and all its items?", &args.id)) {
        println!("cancelled");
        return Ok(());
    }
    core.delete_feed(&args.id).await.map_err(|e| {
        print_error(&format!("failed to remove feed: {e}"));
        e
    })?;
    println!("removed feed {}", &args.id);
    Ok(())
}

async fn cmd_enable(args: FeedIdArgs, core: &PulseCore, enabled: bool) -> anyhow::Result<()> {
    let mut feed = core.get_feed(&args.id).await.map_err(|e| {
        print_error(&format!("feed not found: {}", args.id));
        e
    })?;
    feed.is_enabled = enabled;
    feed.updated_at = chrono::Utc::now().timestamp();
    core.db.upsert_feed(feed).await?;
    println!("feed {} {}", &args.id, if enabled { "enabled" } else { "disabled" });
    Ok(())
}

async fn cmd_edit(args: FeedEditArgs, core: &PulseCore) -> anyhow::Result<()> {
    let mut feed = core.get_feed(&args.id).await.map_err(|e| {
        print_error(&format!("feed not found: {}", args.id));
        e
    })?;

    if let Some(url) = args.url {
        feed.url = url;
    }
    if let Some(interval) = args.interval {
        feed.poll_interval_secs = interval;
    }
    if let Some(name) = args.name {
        feed.title = Some(name);
    }
    if let Some(group_name) = args.group {
        let gid = resolve_or_create_group(core, &group_name).await?;
        feed.group_id = Some(gid);
    }
    feed.updated_at = chrono::Utc::now().timestamp();
    core.db.upsert_feed(feed).await?;
    println!("feed {} updated", &args.id);
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct FeedHealth {
    id: String,
    title: Option<String>,
    success_rate_pct: Option<f64>,
    avg_latency_ms: Option<f64>,
    failure_streak: i64,
    last_success_at: Option<i64>,
}

async fn cmd_health(args: FeedHealthArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    let use_json = args.json || global_json;

    let feeds = if let Some(ref id) = args.id {
        vec![core.get_feed(id).await.map_err(|e| {
            print_error(&format!("feed not found: {id}"));
            e
        })?]
    } else {
        core.get_feeds().await?
    };

    let health: Vec<FeedHealth> = feeds.iter().map(|f| {
        let success_rate = if f.total_fetches > 0 {
            Some((f.total_fetches - f.total_failures) as f64 / f.total_fetches as f64 * 100.0)
        } else {
            None
        };
        FeedHealth {
            id: f.id[..f.id.len().min(8)].to_string(),
            title: f.title.clone(),
            success_rate_pct: success_rate,
            avg_latency_ms: f.avg_latency_ms,
            failure_streak: f.failure_streak,
            last_success_at: f.last_success_at,
        }
    }).collect();

    if use_json {
        print_json(&health);
        return Ok(());
    }

    println!("{:<8}  {:<28}  {:<10}  {:<12}  {:<14}  {}",
        "ID", "TITLE", "SUCCESS%", "AVG_LAT_MS", "FAIL_STREAK", "LAST_SUCCESS");
    for h in &health {
        let title = h.title.as_deref().unwrap_or("-");
        let title_trunc = if title.len() > 28 { &title[..28] } else { title };
        let rate = h.success_rate_pct.map(|r| format!("{:.0}%", r)).unwrap_or_else(|| "-".to_string());
        let lat = h.avg_latency_ms.map(|l| format!("{:.0}", l)).unwrap_or_else(|| "-".to_string());
        let last = h.last_success_at.map(|ts| relative_time(ts)).unwrap_or_else(|| "never".to_string());
        println!("{:<8}  {:<28}  {:<10}  {:<12}  {:<14}  {}",
            h.id, title_trunc, rate, lat, h.failure_streak, last);
    }
    Ok(())
}
