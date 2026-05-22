use clap::Args;
use pulse_core::PulseCore;
use pulse_core::types::{EnrichItemResult, EnrichStatus};

use crate::output::{print_error, print_json};

#[derive(Debug, Args)]
pub struct EnrichArgs {
    /// Limit to a specific feed (prefix ID)
    #[arg(long)]
    pub feed: Option<String>,
    /// Max items to process in this run (default: 200)
    #[arg(long, default_value = "200")]
    pub limit: usize,
    /// Parallel HTTP requests (default: 5)
    #[arg(long, default_value = "5")]
    pub concurrency: usize,
    /// Show per-item output (verbose)
    #[arg(long)]
    pub verbose: bool,
    /// Output summary as JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: EnrichArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    let use_json = args.json || global_json;

    // Resolve optional feed prefix to full ID
    let feed_id: Option<String> = if let Some(ref prefix) = args.feed {
        let feeds = core.get_feeds().await?;
        match feeds
            .into_iter()
            .find(|f| f.id.starts_with(prefix.as_str()))
        {
            Some(f) => Some(f.id),
            None => {
                print_error(&format!("feed '{}' not found", prefix));
                return Ok(());
            }
        }
    } else {
        None
    };

    // Show pending count
    let pending = core.count_pending_enrichment(feed_id.as_deref()).await?;
    if pending == 0 {
        if use_json {
            print_json(
                &serde_json::json!({"enriched":0,"image_posts":0,"skipped":0,"errors":0,"pending":0}),
            );
        } else {
            eprintln!("nothing to enrich");
        }
        return Ok(());
    }

    if !use_json {
        eprintln!(
            "enriching up to {} of {} pending items (concurrency={})...",
            args.limit.min(pending as usize),
            pending,
            args.concurrency
        );
    }

    let verbose = args.verbose && !use_json;

    let stats = core
        .enrich_pending(
            feed_id.as_deref(),
            args.limit,
            args.concurrency,
            |r: &EnrichItemResult| {
                if use_json {
                    // Collect for final JSON output
                } else if verbose {
                    print_item_result(r);
                }
            },
        )
        .await?;

    // For JSON mode, re-run would need results stored — for now just print stats
    if use_json {
        print_json(&serde_json::json!({
            "enriched": stats.enriched,
            "image_posts": stats.image_posts,
            "skipped": stats.skipped,
            "errors": stats.errors,
        }));
    } else {
        eprintln!(
            "done: {} enriched, {} image, {} skipped, {} errors",
            stats.enriched, stats.image_posts, stats.skipped, stats.errors
        );
        let remaining = core
            .count_pending_enrichment(feed_id.as_deref())
            .await
            .unwrap_or(0);
        if remaining > 0 {
            eprintln!(
                "{} items still pending (run again or increase --limit)",
                remaining
            );
        }
    }

    Ok(())
}

fn print_item_result(r: &EnrichItemResult) {
    let prefix = &r.item_id[..r.item_id.len().min(8)];
    match &r.status {
        EnrichStatus::Ok => {
            let desc = r
                .og_description
                .as_deref()
                .unwrap_or("-")
                .chars()
                .take(80)
                .collect::<String>();
            eprintln!("  [ok]    {} {}", prefix, desc);
            if let Some(img) = &r.og_image {
                eprintln!("          img: {}", img);
            }
        }
        EnrichStatus::Image => {
            eprintln!("  [image] {} (image post)", prefix);
        }
        EnrichStatus::Skipped => {
            eprintln!(
                "  [skip]  {} {}",
                prefix,
                r.url.chars().take(60).collect::<String>()
            );
        }
        EnrichStatus::Error(e) => {
            eprintln!(
                "  [err]   {} {} — {}",
                prefix,
                r.url.chars().take(50).collect::<String>(),
                e
            );
        }
    }
}
