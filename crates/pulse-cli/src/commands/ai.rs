use clap::{Args, Subcommand};
use pulse_core::PulseCore;
use pulse_core::ai::{RuleScope, default_rules};

use crate::output::{print_json, print_error};

#[derive(Debug, Args)]
pub struct AiArgs {
    #[command(subcommand)]
    pub command: AiCommand,
}

#[derive(Debug, Subcommand)]
pub enum AiCommand {
    /// Trigger tagging on untagged items
    Run(AiRunArgs),
    /// Show AI pipeline status
    Status(AiStatusArgs),
    /// Manage tag rules
    Rules(AiRulesArgs),
}

#[derive(Debug, Args)]
pub struct AiRunArgs {
    /// Limit to a specific feed
    #[arg(long)]
    pub feed: Option<String>,
}

#[derive(Debug, Args)]
pub struct AiStatusArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
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
    /// Add a tag rule
    Add(AiRulesAddArgs),
    /// Disable a rule
    Disable(AiRulesIdArgs),
    /// Enable a rule
    Enable(AiRulesIdArgs),
}

#[derive(Debug, Args)]
pub struct AiRulesListArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct AiRulesAddArgs {
    /// Tag name to apply
    #[arg(long)]
    pub tag: String,
    /// Keyword to match (repeatable)
    #[arg(long)]
    pub keyword: Vec<String>,
    /// Treat keyword as regex
    #[arg(long)]
    pub regex: bool,
    /// Field to match against: title, body, both
    #[arg(long, default_value = "both")]
    pub field: String,
    /// Confidence score (0.0-1.0)
    #[arg(long, default_value = "0.80")]
    pub confidence: f32,
}

#[derive(Debug, Args)]
pub struct AiRulesIdArgs {
    /// Rule ID
    pub id: String,
}

pub async fn run(args: AiArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    match args.command {
        AiCommand::Run(a) => cmd_run(a, core).await,
        AiCommand::Status(a) => cmd_status(a, core, global_json).await,
        AiCommand::Rules(a) => cmd_rules(a, core, global_json).await,
    }
}

async fn cmd_run(args: AiRunArgs, core: &PulseCore) -> anyhow::Result<()> {
    // Resolve optional prefix to full feed ID
    let feed_id: Option<String> = if let Some(ref prefix) = args.feed {
        let feeds = core.get_feeds().await?;
        let found = feeds.into_iter().find(|f| f.id.starts_with(prefix.as_str()));
        match found {
            Some(f) => Some(f.id),
            None => {
                print_error(&format!("feed '{}' not found", prefix));
                return Ok(());
            }
        }
    } else {
        None
    };

    eprintln!("running rule engine on untagged items...");
    let (items, tags) = core.run_tagger_direct(feed_id.as_deref()).await?;
    eprintln!("tagged {} items, {} tags created", items, tags);
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct AiStatus {
    active_model: String,
    tagging_mode: String,
    rule_count: usize,
}

async fn cmd_status(args: AiStatusArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    let use_json = args.json || global_json;
    let rules = default_rules();
    let enabled = rules.iter().filter(|r| r.enabled).count();

    let status = AiStatus {
        active_model: "rule-engine".to_string(),
        tagging_mode: "rule-based".to_string(),
        rule_count: enabled,
    };

    if use_json {
        print_json(&status);
        return Ok(());
    }

    let stats = core.get_db_stats().await?;
    println!("Active model:  rule-engine (Phase 1)");
    println!("Tagging mode:  rule-based");
    println!("Rules loaded:  {} enabled / {} total", enabled, rules.len());
    println!("Tags in DB:    {}", stats.tag_count);
    Ok(())
}

async fn cmd_rules(args: AiRulesArgs, _core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    match args.command {
        AiRulesCommand::List(a) => cmd_rules_list(a, global_json).await,
        AiRulesCommand::Add(a) => cmd_rules_add(a).await,
        AiRulesCommand::Disable(a) => cmd_rules_toggle(a, false).await,
        AiRulesCommand::Enable(a) => cmd_rules_toggle(a, true).await,
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

    let views: Vec<RuleView> = rules.iter().map(|r| RuleView {
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
    }).collect();

    if use_json {
        print_json(&views);
        return Ok(());
    }

    println!("{:<20}  {:<20}  {:<6}  {:<8}  {:<8}  {}",
        "ID", "TAG", "CONF", "ENABLED", "PATTERNS", "SCOPE");
    for v in &views {
        println!("{:<20}  {:<20}  {:<6.2}  {:<8}  {:<8}  {}",
            v.id, v.tag, v.confidence, v.enabled, v.pattern_count, v.scope);
    }
    Ok(())
}

async fn cmd_rules_add(args: AiRulesAddArgs) -> anyhow::Result<()> {
    // Phase 1: rules are code-defined. Inform the user.
    print_error(
        "rule persistence is not implemented in Phase 1. \
         Rules are defined in crates/pulse-core/src/ai/rules.rs. \
         The flags you provided would create:"
    );
    eprintln!("  tag: {}", args.tag);
    eprintln!("  keywords: {:?}", args.keyword);
    eprintln!("  field: {}", args.field);
    eprintln!("  confidence: {}", args.confidence);
    eprintln!("  regex: {}", args.regex);
    Ok(())
}

async fn cmd_rules_toggle(args: AiRulesIdArgs, enable: bool) -> anyhow::Result<()> {
    print_error(&format!(
        "rule persistence is not implemented in Phase 1. \
         To {} rule '{}', edit crates/pulse-core/src/ai/rules.rs.",
        if enable { "enable" } else { "disable" },
        args.id
    ));
    Ok(())
}
