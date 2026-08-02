use clap::{Args, Subcommand};
use pulse_core::PulseCore;
use pulse_core::ai::{RuleScope, default_rules};

use crate::output::print_json;

#[derive(Debug, Args)]
pub struct AiArgs {
    #[command(subcommand)]
    pub command: AiCommand,
}

#[derive(Debug, Subcommand)]
pub enum AiCommand {
    /// Tag untagged items using the deterministic rule engine
    Run(AiRunArgs),
    /// Manage tag rules
    Rules(AiRulesArgs),
}

#[derive(Debug, Args)]
pub struct AiRunArgs {
    /// Limit to a specific feed (id prefix)
    #[arg(long)]
    pub feed: Option<String>,
    /// Re-tag ALL items, clearing existing tags first (use after vocabulary changes)
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct AiRulesArgs {
    #[command(subcommand)]
    pub command: AiRulesCommand,
}

#[derive(Debug, Subcommand)]
pub enum AiRulesCommand {
    /// List all tag rules
    List(AiRulesListArgs),
}

#[derive(Debug, Args)]
pub struct AiRulesListArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: AiArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    match args.command {
        AiCommand::Run(a) => cmd_run(a, core).await,
        AiCommand::Rules(a) => cmd_rules(a, global_json).await,
    }
}

async fn cmd_run(args: AiRunArgs, core: &PulseCore) -> anyhow::Result<()> {
    let feed_id: Option<String> = if let Some(ref prefix) = args.feed {
        let feeds = core.get_feeds().await?;
        match feeds
            .into_iter()
            .find(|f| f.id.starts_with(prefix.as_str()))
        {
            Some(f) => Some(f.id),
            None => anyhow::bail!("feed '{}' not found", prefix),
        }
    } else {
        None
    };

    if args.force {
        eprintln!("running tagger (rule-engine) — force-retagging ALL items...");
    } else {
        eprintln!("running tagger (rule-engine) on untagged items...");
    }
    let (items, tags) = core
        .run_tagger_direct(feed_id.as_deref(), args.force, None)
        .await?;
    eprintln!("tagged {} items, {} tags created", items, tags);
    Ok(())
}

// ── Rules command handlers ─────────────────────────────────────────────────────

async fn cmd_rules(args: AiRulesArgs, global_json: bool) -> anyhow::Result<()> {
    match args.command {
        AiRulesCommand::List(a) => cmd_rules_list(a, global_json).await,
    }
}

#[derive(Debug, serde::Serialize)]
struct RuleView {
    id: String,
    tag: String,
    confidence: f32,
    enabled: bool,
    pattern_count: usize,
    scope: String,
}

async fn cmd_rules_list(args: AiRulesListArgs, global_json: bool) -> anyhow::Result<()> {
    let use_json = args.json || global_json;
    let rules = default_rules();

    let views: Vec<RuleView> = rules
        .iter()
        .map(|r| RuleView {
            id: r.id.clone(),
            tag: r.tag.clone(),
            confidence: r.confidence,
            enabled: r.enabled,
            pattern_count: r.patterns.len(),
            scope: match r.scope {
                RuleScope::All => "both".to_string(),
                RuleScope::TitleOnly => "title".to_string(),
                RuleScope::BodyOnly => "body".to_string(),
            },
        })
        .collect();

    if use_json {
        print_json(&views);
        return Ok(());
    }

    println!(
        "{:<20}  {:<20}  {:<6}  {:<8}  {:<8}  {}",
        "ID", "TAG", "CONF", "ENABLED", "PATTERNS", "SCOPE"
    );
    for v in &views {
        println!(
            "{:<20}  {:<20}  {:<6.2}  {:<8}  {:<8}  {}",
            v.id, v.tag, v.confidence, v.enabled, v.pattern_count, v.scope
        );
    }
    Ok(())
}
