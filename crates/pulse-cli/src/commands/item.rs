use clap::{Args, Subcommand};
use pulse_core::{PulseCore, types::ItemStatePatch};

use crate::output::{print_error, print_json, relative_time};

#[derive(Debug, Args)]
pub struct ItemArgs {
    #[command(subcommand)]
    pub command: ItemCommand,
}

#[derive(Debug, Subcommand)]
pub enum ItemCommand {
    /// Show full details for an item
    Show(ItemShowArgs),
    /// Mark an item as read
    Read(ItemIdArgs),
    /// Mark an item as unread
    Unread(ItemIdArgs),
    /// Save an item
    Save(ItemIdArgs),
    /// Remove an item from saved
    Unsave(ItemIdArgs),
    /// Hide an item
    Hide(ItemIdArgs),
    /// Unhide an item
    Unhide(ItemIdArgs),
    /// Manage AI tags
    Tags(ItemTagsArgs),
    /// Open item URL in browser
    Open(ItemIdArgs),
}

#[derive(Debug, Args)]
pub struct ItemShowArgs {
    /// Item ID
    pub id: String,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ItemIdArgs {
    /// Item ID
    pub id: String,
}

#[derive(Debug, Args)]
pub struct ItemTagsArgs {
    #[command(subcommand)]
    pub command: ItemTagsCommand,
}

#[derive(Debug, Subcommand)]
pub enum ItemTagsCommand {
    /// Show AI tags for an item
    Show(ItemTagsShowArgs),
}

#[derive(Debug, Args)]
pub struct ItemTagsShowArgs {
    /// Item ID
    pub id: String,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: ItemArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    match args.command {
        ItemCommand::Show(a) => cmd_show(a, core, global_json).await,
        ItemCommand::Read(a) => {
            cmd_set_state(
                a.id,
                core,
                ItemStatePatch {
                    is_read: Some(true),
                    ..Default::default()
                },
                "marked read",
            )
            .await
        }
        ItemCommand::Unread(a) => {
            cmd_set_state(
                a.id,
                core,
                ItemStatePatch {
                    is_read: Some(false),
                    ..Default::default()
                },
                "marked unread",
            )
            .await
        }
        ItemCommand::Save(a) => {
            cmd_set_state(
                a.id,
                core,
                ItemStatePatch {
                    is_saved: Some(true),
                    ..Default::default()
                },
                "saved",
            )
            .await
        }
        ItemCommand::Unsave(a) => {
            cmd_set_state(
                a.id,
                core,
                ItemStatePatch {
                    is_saved: Some(false),
                    ..Default::default()
                },
                "unsaved",
            )
            .await
        }
        ItemCommand::Hide(a) => {
            cmd_set_state(
                a.id,
                core,
                ItemStatePatch {
                    is_hidden: Some(true),
                    ..Default::default()
                },
                "hidden",
            )
            .await
        }
        ItemCommand::Unhide(a) => {
            cmd_set_state(
                a.id,
                core,
                ItemStatePatch {
                    is_hidden: Some(false),
                    ..Default::default()
                },
                "unhidden",
            )
            .await
        }
        ItemCommand::Tags(a) => cmd_tags(a, core, global_json).await,
        ItemCommand::Open(a) => cmd_open(a, core).await,
    }
}

async fn cmd_show(args: ItemShowArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    let use_json = args.json || global_json;

    // Resolve prefix → full ID, then fetch the joined view directly (no
    // recency cap — the old code only searched the newest 1000 items).
    let full_id = match core.resolve_item_id(&args.id).await? {
        Some(id) => id,
        None => {
            print_error(&format!("item not found: {}", args.id));
            return Ok(());
        }
    };
    let item = match core.get_item_view(&full_id).await {
        Ok(i) => i,
        Err(e) => {
            print_error(&format!("item not found: {}", args.id));
            return Err(e.into());
        }
    };

    if use_json {
        print_json(&item);
        return Ok(());
    }

    // Get AI tags
    let tags = core.get_item_tags(&item.id).await.unwrap_or_default();

    // Human display
    println!("Title:      {}", item.title);
    println!(
        "Feed:       {} ({})",
        item.feed_title.as_deref().unwrap_or("-"),
        item.feed_type
    );
    println!("URL:        {}", item.url.as_deref().unwrap_or("-"));
    let published_fmt = chrono::DateTime::from_timestamp(item.published_at, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "-".to_string());
    println!(
        "Published:  {} ({} ago)",
        published_fmt,
        relative_time(item.published_at)
    );
    if let Some(score) = item.score {
        println!("Score:      {}", score);
    } else {
        println!("Score:      -");
    }
    if let Some(cc) = item.comment_count {
        println!("Comments:   {}", cc);
    } else {
        println!("Comments:   -");
    }
    if let Some(wc) = item.word_count {
        let read_min = (wc as f64 / 200.0).ceil() as i64;
        println!("Word count: ~{} words (~{} min read)", wc, read_min);
    }
    let state = if item.is_saved {
        "saved"
    } else if item.is_hidden {
        "hidden"
    } else if item.is_read {
        "read"
    } else {
        "unread"
    };
    println!("State:      {}", state);

    if !tags.is_empty() {
        println!("\nAI Tags:");
        for tag in &tags {
            println!(
                "  {:<12}  ({:.2})  \"{}\"",
                tag.tag, tag.confidence, tag.explanation
            );
        }
    } else {
        println!("\nAI Tags:    (none)");
    }

    if !item.ai_tags.is_empty() {
        println!("\nTag names:  {}", item.ai_tags.join(", "));
    }

    Ok(())
}

async fn cmd_set_state(
    item_id: String,
    core: &PulseCore,
    patch: ItemStatePatch,
    action: &str,
) -> anyhow::Result<()> {
    let full_id = match core.resolve_item_id(&item_id).await? {
        Some(id) => id,
        None => {
            print_error(&format!("item not found: {}", item_id));
            return Ok(());
        }
    };
    core.update_item_state(&full_id, patch).await.map_err(|e| {
        print_error(&format!("failed to update item: {e}"));
        e
    })?;
    println!("item {} {}", &full_id[..full_id.len().min(8)], action);
    Ok(())
}

async fn cmd_tags(args: ItemTagsArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    match args.command {
        ItemTagsCommand::Show(a) => {
            let use_json = a.json || global_json;
            let full_id = match core.resolve_item_id(&a.id).await? {
                Some(id) => id,
                None => {
                    print_error(&format!("item not found: {}", a.id));
                    return Ok(());
                }
            };
            let tags = core.get_item_tags(&full_id).await.map_err(|e| {
                print_error(&format!("failed to get tags: {e}"));
                e
            })?;
            if use_json {
                print_json(&tags);
                return Ok(());
            }
            if tags.is_empty() {
                println!("no AI tags for item {}", a.id);
                return Ok(());
            }
            for tag in &tags {
                println!(
                    "{:<12}  {:.2}  {}  \"{}\"",
                    tag.tag,
                    tag.confidence,
                    tag.tagger_source.as_str(),
                    tag.explanation
                );
            }
        }
    }
    Ok(())
}

async fn cmd_open(args: ItemIdArgs, core: &PulseCore) -> anyhow::Result<()> {
    let full_id = match core.resolve_item_id(&args.id).await? {
        Some(id) => id,
        None => {
            print_error(&format!("item not found: {}", args.id));
            return Ok(());
        }
    };
    let item = core.get_item(&full_id).await?;
    let url = item.url.as_deref().unwrap_or("");
    if url.is_empty() {
        print_error("item has no URL to open");
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(url).spawn()?;
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(url).spawn()?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd")
        .args(["/c", "start", url])
        .spawn()?;
    Ok(())
}
