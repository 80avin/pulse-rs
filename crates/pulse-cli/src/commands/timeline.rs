use clap::Args;
use pulse_core::{PulseCore, types::{TimelineFilter, FeedItemView}};

use crate::output::{print_json, relative_time, score_display};

#[derive(Debug, Args)]
pub struct TimelineArgs {
    /// Max items to show
    #[arg(long, default_value = "50")]
    pub limit: usize,
    /// Show only unread items
    #[arg(long)]
    pub unread: bool,
    /// Show only saved items
    #[arg(long)]
    pub saved: bool,
    /// Filter by group name
    #[arg(long)]
    pub group: Option<String>,
    /// Filter by feed ID
    #[arg(long)]
    pub feed: Option<String>,
    /// Filter by AI tag
    #[arg(long)]
    pub tag: Option<String>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: TimelineArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    let use_json = args.json || global_json;

    // Resolve group name to ID
    let group_id = if let Some(ref gname) = args.group {
        let groups = core.get_feed_groups().await?;
        match groups.iter().find(|g| g.name.eq_ignore_ascii_case(gname)) {
            Some(g) => Some(g.id.clone()),
            None => {
                crate::output::print_error(&format!("group '{}' not found", gname));
                return Ok(());
            }
        }
    } else {
        None
    };

    let filter = TimelineFilter {
        group_id,
        feed_id: args.feed,
        is_read: if args.unread { Some(false) } else { None },
        is_saved: if args.saved { Some(true) } else { None },
        tag: args.tag,
    };

    let page = core.get_timeline_page(filter, None, args.limit).await?;

    if use_json {
        print_json(&page.items);
        return Ok(());
    }

    print_items_human(&page.items);
    Ok(())
}

pub fn print_items_human(items: &[FeedItemView]) {
    for item in items {
        let id_prefix = &item.id[..item.id.len().min(8)];
        let state = if item.is_saved {
            "★"
        } else if item.is_hidden {
            "✕"
        } else if !item.is_read {
            "●"
        } else {
            "○"
        };
        let age = relative_time(item.published_at);
        let score = score_display(item.score, &item.feed_type);
        let feed = item.feed_title.as_deref().unwrap_or(&item.feed_url);
        let feed_trunc = if feed.len() > 20 { &feed[..20] } else { feed };
        let title = &item.title;
        let title_trunc = if title.len() > 60 { &title[..60] } else { title };
        println!("{} {} {}  {:<5}  {:<20}  \"{}\"",
            id_prefix, state, age, score, feed_trunc, title_trunc);
    }
}
