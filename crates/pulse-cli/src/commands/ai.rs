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
    /// Show raw ONNX similarity scores for a text (threshold calibration)
    Debug(AiDebugArgs),
    /// Manage the ONNX inference model
    Model(AiModelArgs),
    /// Manage tag rules (rule-based fallback)
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

// ── Model subcommands ──────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct AiModelArgs {
    #[command(subcommand)]
    pub command: AiModelCommand,
}

#[derive(Debug, Subcommand)]
pub enum AiModelCommand {
    /// List downloaded models
    List(AiModelListArgs),
    /// Set the active model (model files must already exist in the model directory)
    Set(AiModelSetArgs),
    /// Show where to place model files for a given model name
    Path(AiModelPathArgs),
    /// Remove a downloaded model
    Remove(AiModelRemoveArgs),
    /// Unset the active model (revert to rules-only tagging)
    Unset,
}

#[derive(Debug, Args)]
pub struct AiModelListArgs {
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct AiModelSetArgs {
    /// Model name (must match a directory under the models path)
    pub name: String,
}

#[derive(Debug, Args)]
pub struct AiModelPathArgs {
    /// Model name to show the expected path for
    pub name: String,
}

#[derive(Debug, Args)]
pub struct AiModelRemoveArgs {
    /// Model name to remove
    pub name: String,
}

// ── Rules subcommands ──────────────────────────────────────────────────────────

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
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct AiDebugArgs {
    /// Text to classify (use quotes for multi-word input)
    pub text: String,
}

pub async fn run(args: AiArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    match args.command {
        AiCommand::Run(a) => cmd_run(a, core).await,
        AiCommand::Status(a) => cmd_status(a, core, global_json).await,
        AiCommand::Debug(a) => cmd_debug(a, core).await,
        AiCommand::Model(a) => cmd_model(a, core, global_json).await,
        AiCommand::Rules(a) => cmd_rules(a, global_json).await,
    }
}

async fn cmd_run(args: AiRunArgs, core: &PulseCore) -> anyhow::Result<()> {
    let feed_id: Option<String> = if let Some(ref prefix) = args.feed {
        let feeds = core.get_feeds().await?;
        match feeds.into_iter().find(|f| f.id.starts_with(prefix.as_str())) {
            Some(f) => Some(f.id),
            None => {
                print_error(&format!("feed '{}' not found", prefix));
                return Ok(());
            }
        }
    } else {
        None
    };

    let mode = if core.onnx_tagger.is_some() { "onnx" } else { "rule-engine" };
    eprintln!("running tagger ({}) on untagged items...", mode);
    let (items, tags) = core.run_tagger_direct(feed_id.as_deref()).await?;
    eprintln!("tagged {} items, {} tags created", items, tags);
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct AiStatus {
    active_model: String,
    tagging_mode: String,
    rule_count: usize,
    onnx_loaded: bool,
}

async fn cmd_status(args: AiStatusArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    let use_json = args.json || global_json;
    let rules = default_rules();
    let enabled = rules.iter().filter(|r| r.enabled).count();
    let onnx_loaded = core.onnx_tagger.is_some();
    let active_model = core.active_model_name()
        .unwrap_or_else(|| "rule-engine".to_string());
    let tagging_mode = if onnx_loaded { "onnx+rules" } else { "rule-based" }.to_string();

    let status = AiStatus { active_model: active_model.clone(), tagging_mode: tagging_mode.clone(), rule_count: enabled, onnx_loaded };

    if use_json {
        print_json(&status);
        return Ok(());
    }

    let stats = core.get_db_stats().await?;
    println!("Active model:  {}", active_model);
    println!("Tagging mode:  {}", tagging_mode);
    println!("ONNX loaded:   {}", onnx_loaded);
    println!("Rules loaded:  {} enabled / {} total", enabled, rules.len());
    println!("Tags in DB:    {}", stats.tag_count);
    Ok(())
}

async fn cmd_debug(args: AiDebugArgs, core: &PulseCore) -> anyhow::Result<()> {
    let Some(ref tagger) = core.onnx_tagger else {
        print_error("no ONNX model loaded — run 'pulse ai model set <name>' first");
        return Ok(());
    };

    let sims = tagger.similarities(&args.text)?;
    println!("{:<20}  {}", "TAG", "ENTAILMENT");
    println!("{}", "-".repeat(35));
    for (tag, prob) in &sims {
        let marker = if *prob >= 0.50 { " ✓" } else { "" };
        println!("{:<20}  {:.4}{}", tag, prob, marker);
    }
    Ok(())
}

// ── Model command handlers ─────────────────────────────────────────────────────

async fn cmd_model(args: AiModelArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    match args.command {
        AiModelCommand::List(a) => cmd_model_list(a, core, global_json).await,
        AiModelCommand::Set(a) => cmd_model_set(a, core).await,
        AiModelCommand::Path(a) => cmd_model_path(a, core).await,
        AiModelCommand::Remove(a) => cmd_model_remove(a, core).await,
        AiModelCommand::Unset => cmd_model_unset(core).await,
    }
}

async fn cmd_model_list(args: AiModelListArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    let use_json = args.json || global_json;
    let models = core.list_models();
    let active = core.active_model_name();

    if use_json {
        print_json(&serde_json::json!({
            "models": models,
            "active": active,
        }));
        return Ok(());
    }

    if models.is_empty() {
        eprintln!("no models downloaded");
        eprintln!("use 'pulse ai model path <name>' to see where to place model files");
        return Ok(());
    }

    for m in &models {
        let marker = if active.as_deref() == Some(m.as_str()) { " (active)" } else { "" };
        println!("{}{}", m, marker);
    }
    Ok(())
}

async fn cmd_model_set(args: AiModelSetArgs, core: &PulseCore) -> anyhow::Result<()> {
    core.set_active_model(&args.name)?;
    eprintln!("active model set to '{}' (restart pulse to apply)", args.name);
    Ok(())
}

async fn cmd_model_path(args: AiModelPathArgs, core: &PulseCore) -> anyhow::Result<()> {
    let dir = core.model_dir(&args.name);
    println!("{}", dir.display());
    eprintln!("place model.onnx and tokenizer.json in that directory, then run:");
    eprintln!("  pulse ai model set {}", args.name);
    Ok(())
}

async fn cmd_model_remove(args: AiModelRemoveArgs, core: &PulseCore) -> anyhow::Result<()> {
    core.remove_model(&args.name)?;
    eprintln!("model '{}' removed", args.name);
    Ok(())
}

async fn cmd_model_unset(core: &PulseCore) -> anyhow::Result<()> {
    core.unset_active_model()?;
    eprintln!("active model cleared — tagging will use rules only");
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
