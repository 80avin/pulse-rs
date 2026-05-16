use clap::Args;
use pulse_core::PulseCore;

use crate::output::{print_json, format_bytes, relative_time};

#[derive(Debug, Args)]
pub struct DiagArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: DiagArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    let use_json = args.json || global_json;

    let stats = core.get_db_stats().await?;
    let feeds = core.get_feeds().await?;

    let total_feeds = feeds.len();
    let enabled_feeds = feeds.iter().filter(|f| f.is_enabled).count();
    let healthy_feeds = feeds.iter().filter(|f| {
        f.total_fetches == 0 || {
            let rate = (f.total_fetches - f.total_failures) as f64 / f.total_fetches as f64;
            rate >= 0.9
        }
    }).count();
    let degraded_feeds = feeds.iter().filter(|f| {
        if f.total_fetches == 0 { return false; }
        let rate = (f.total_fetches - f.total_failures) as f64 / f.total_fetches as f64;
        rate >= 0.5 && rate < 0.9
    }).count();
    let failing_feeds = feeds.iter().filter(|f| {
        if f.total_fetches == 0 { return false; }
        let rate = (f.total_fetches - f.total_failures) as f64 / f.total_fetches as f64;
        rate < 0.5
    }).count();

    let now = chrono::Utc::now();

    if use_json {
        let report = serde_json::json!({
            "generated_at": now.to_rfc3339(),
            "db": {
                "path": core.config.db_path.display().to_string(),
                "size_bytes": stats.db_size_bytes,
                "size_human": format_bytes(stats.db_size_bytes),
            },
            "feeds": {
                "total": total_feeds,
                "enabled": enabled_feeds,
                "healthy_gt90pct": healthy_feeds,
                "degraded_50_90pct": degraded_feeds,
                "failing_lt50pct": failing_feeds,
            },
            "items": {
                "total": stats.item_count,
                "unread": stats.unread_count,
                "saved": stats.saved_count,
            },
            "ai": {
                "tag_count": stats.tag_count,
                "active_model": "rule-engine",
            },
        });
        print_json(&report);
        return Ok(());
    }

    println!("Pulse Diagnostic Report — {}", now.format("%Y-%m-%d %H:%M:%S UTC"));
    println!();
    println!("System:");
    println!("  DB path:    {}", core.config.db_path.display());
    println!("  DB size:    {}", format_bytes(stats.db_size_bytes));
    println!();
    println!("Feeds:");
    println!("  Total:           {}", total_feeds);
    println!("  Enabled:         {}", enabled_feeds);
    println!("  Healthy (>90%):  {}", healthy_feeds);
    println!("  Degraded (50-90%): {}", degraded_feeds);
    println!("  Failing (<50%):  {}", failing_feeds);
    println!();
    println!("Items:");
    println!("  Total:   {}", stats.item_count);
    println!("  Unread:  {}", stats.unread_count);
    println!("  Saved:   {}", stats.saved_count);
    println!();
    println!("AI Pipeline:");
    println!("  Active model:  rule-engine (Phase 1)");
    println!("  Tags applied:  {}", stats.tag_count);
    println!();

    // Show failing feeds
    let failing: Vec<_> = feeds.iter().filter(|f| f.failure_streak > 0).collect();
    if !failing.is_empty() {
        println!("Failing Feeds:");
        for f in &failing {
            let title = f.title.as_deref().unwrap_or(&f.url);
            let last = f.last_fetched_at.map(|ts| relative_time(ts)).unwrap_or_else(|| "never".to_string());
            println!("  {} ({}): {} failures — last attempt {} ago",
                &f.id[..f.id.len().min(8)], title, f.failure_streak, last);
        }
    }

    Ok(())
}
