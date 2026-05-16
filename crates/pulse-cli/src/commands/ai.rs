use clap::{Args, Subcommand};
use pulse_core::PulseCore;
use pulse_core::ai::{RuleScope, default_rules};
use reqwest::Client;

use crate::output::{print_json, print_error};

// ── Known model registry ───────────────────────────────────────────────────────

struct ModelSpec {
    name: &'static str,
    description: &'static str,
    hf_owner: &'static str,
    hf_repo: &'static str,
    /// (path_in_repo, filename_in_model_dir)
    files: &'static [(&'static str, &'static str)],
    size_mb_approx: u32,
}

struct VisionModelSpec {
    name: &'static str,
    description: &'static str,
    hf_owner: &'static str,
    hf_repo: &'static str,
    /// (hf_path, local_filename)
    files: &'static [(&'static str, &'static str)],
    size_mb_approx: u32,
}

const KNOWN_MODELS: &[ModelSpec] = &[
    ModelSpec {
        name: "nli-deberta-v3-xsmall",
        description: "DeBERTa v3 xsmall NLI cross-encoder — 22M params, ~35 MB quantized (recommended default)",
        hf_owner: "Xenova",
        hf_repo: "nli-deberta-v3-xsmall",
        files: &[
            ("onnx/model_quantized.onnx", "model_quantized.onnx"),
            ("tokenizer.json", "tokenizer.json"),
            ("config.json", "config.json"),
        ],
        size_mb_approx: 35,
    },
    ModelSpec {
        name: "nli-deberta-v3-small",
        description: "DeBERTa v3 small NLI — 44M params, ~68 MB quantized (higher quality, slower)",
        hf_owner: "Xenova",
        hf_repo: "nli-deberta-v3-small",
        files: &[
            ("onnx/model_quantized.onnx", "model_quantized.onnx"),
            ("tokenizer.json", "tokenizer.json"),
            ("config.json", "config.json"),
        ],
        size_mb_approx: 68,
    },
];

const KNOWN_VISION_MODELS: &[VisionModelSpec] = &[
    VisionModelSpec {
        name: "clip-vit-b32",
        description: "CLIP ViT-B/32 vision encoder — zero-shot image classification (~53 MB q4f16)",
        hf_owner: "Xenova",
        hf_repo: "clip-vit-base-patch32",
        files: &[
            ("onnx/vision_model_q4f16.onnx", "vision_model_q4f16.onnx"),
        ],
        size_mb_approx: 53,
    },
];

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
    /// Show raw NLI entailment scores for text (threshold calibration)
    Debug(AiDebugArgs),
    /// Show raw CLIP cosine scores for an image URL (vision threshold calibration)
    VisionDebug(AiVisionDebugArgs),
    /// Download an NLI text model from HuggingFace and make it active
    Download(AiDownloadArgs),
    /// Download a CLIP vision model from HuggingFace and make it active
    VisionDownload(AiVisionDownloadArgs),
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

#[derive(Debug, Args)]
pub struct AiVisionDebugArgs {
    /// Image URL to classify via CLIP
    pub url: String,
}

#[derive(Debug, Args)]
pub struct AiDownloadArgs {
    /// Model name to download (default: nli-deberta-v3-xsmall)
    pub name: Option<String>,
    /// Do not set this model as active after downloading
    #[arg(long)]
    pub no_activate: bool,
    /// List available models and exit
    #[arg(long)]
    pub list: bool,
}

#[derive(Debug, Args)]
pub struct AiVisionDownloadArgs {
    /// Vision model name to download (default: clip-vit-b32)
    pub name: Option<String>,
    /// Do not set this model as active after downloading
    #[arg(long)]
    pub no_activate: bool,
    /// List available vision models and exit
    #[arg(long)]
    pub list: bool,
}

pub async fn run(args: AiArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    match args.command {
        AiCommand::Run(a) => cmd_run(a, core).await,
        AiCommand::Status(a) => cmd_status(a, core, global_json).await,
        AiCommand::Debug(a) => cmd_debug(a, core).await,
        AiCommand::VisionDebug(a) => cmd_vision_debug(a, core).await,
        AiCommand::Download(a) => cmd_model_download(a, core).await,
        AiCommand::VisionDownload(a) => cmd_vision_download(a, core).await,
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

    let mode = match (core.onnx_tagger.is_some(), core.vision_tagger.is_some()) {
        (true, true)  => "onnx+vision+rules",
        (true, false) => "onnx+rules",
        (false, true) => "vision+rules",
        (false, false) => "rule-engine",
    };
    eprintln!("running tagger ({}) on untagged items...", mode);
    let (items, tags) = core.run_tagger_direct(feed_id.as_deref()).await?;
    eprintln!("tagged {} items, {} tags created", items, tags);
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct AiStatus {
    active_model: String,
    active_vision_model: String,
    tagging_mode: String,
    rule_count: usize,
    onnx_loaded: bool,
    vision_loaded: bool,
}

async fn cmd_status(args: AiStatusArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    let use_json = args.json || global_json;
    let rules = default_rules();
    let enabled = rules.iter().filter(|r| r.enabled).count();
    let onnx_loaded = core.onnx_tagger.is_some();
    let vision_loaded = core.vision_tagger.is_some();
    let active_model = core.active_model_name()
        .unwrap_or_else(|| "none".to_string());
    let active_vision_model = core.active_vision_model_name()
        .unwrap_or_else(|| "none".to_string());
    let tagging_mode = match (onnx_loaded, vision_loaded) {
        (true, true)  => "onnx+vision+rules",
        (true, false) => "onnx+rules",
        (false, true) => "vision+rules",
        (false, false) => "rule-based",
    }.to_string();

    let status = AiStatus {
        active_model: active_model.clone(),
        active_vision_model: active_vision_model.clone(),
        tagging_mode: tagging_mode.clone(),
        rule_count: enabled,
        onnx_loaded,
        vision_loaded,
    };

    if use_json {
        print_json(&status);
        return Ok(());
    }

    let stats = core.get_db_stats().await?;
    println!("Text model:    {}", active_model);
    println!("Vision model:  {}", active_vision_model);
    println!("Tagging mode:  {}", tagging_mode);
    println!("ONNX loaded:   {}", onnx_loaded);
    println!("Vision loaded: {}", vision_loaded);
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

async fn cmd_vision_debug(args: AiVisionDebugArgs, core: &PulseCore) -> anyhow::Result<()> {
    let Some(ref vision) = core.vision_tagger else {
        print_error("no vision model loaded — run 'pulse ai vision-download' first");
        return Ok(());
    };

    eprint!("fetching and classifying {} ...", args.url);
    std::io::Write::flush(&mut std::io::stderr())?;

    match vision.similarities_url(&args.url).await {
        Ok(sims) => {
            eprintln!(" done");
            println!("{:<20}  {}", "TAG", "COSINE SIM");
            println!("{}", "-".repeat(35));
            for (tag, score) in &sims {
                let marker = if *score >= 0.22 { " ✓" } else { "" };
                println!("{:<20}  {:.4}{}", tag, score, marker);
            }
        }
        Err(e) => {
            eprintln!(" error");
            print_error(&format!("vision debug failed: {}", e));
        }
    }
    Ok(())
}

// ── Download command ───────────────────────────────────────────────────────────

async fn cmd_model_download(args: AiDownloadArgs, core: &PulseCore) -> anyhow::Result<()> {
    if args.list {
        println!("{:<30}  {:<8}  {}", "NAME", "SIZE", "DESCRIPTION");
        for m in KNOWN_MODELS {
            println!("{:<30}  ~{:>4} MB  {}", m.name, m.size_mb_approx, m.description);
        }
        return Ok(());
    }

    let model_name = args.name.as_deref().unwrap_or("nli-deberta-v3-xsmall");
    let spec = match KNOWN_MODELS.iter().find(|m| m.name == model_name) {
        Some(s) => s,
        None => {
            print_error(&format!("unknown model '{}' — run 'pulse ai download --list' to see options", model_name));
            return Ok(());
        }
    };

    let model_dir = core.model_dir(spec.name);
    std::fs::create_dir_all(&model_dir)?;

    eprintln!("Downloading {} (~{} MB) from huggingface.co/{}/{} ...",
        spec.name, spec.size_mb_approx, spec.hf_owner, spec.hf_repo);

    let client = Client::builder()
        .user_agent("Pulse/0.1 model-downloader")
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    for (hf_path, local_name) in spec.files {
        let url = format!(
            "https://huggingface.co/{}/{}/resolve/main/{}",
            spec.hf_owner, spec.hf_repo, hf_path
        );
        let dest = model_dir.join(local_name);

        eprint!("  {} ... ", local_name);
        std::io::Write::flush(&mut std::io::stderr())?;

        let resp = client.get(&url).send().await
            .map_err(|e| anyhow::anyhow!("network error fetching {}: {}", local_name, e))?;

        if !resp.status().is_success() {
            anyhow::bail!("HTTP {} for {}: {}", resp.status(), local_name, url);
        }

        let bytes = resp.bytes().await
            .map_err(|e| anyhow::anyhow!("read error for {}: {}", local_name, e))?;

        std::fs::write(&dest, &bytes)?;
        eprintln!("{:.1} MB", bytes.len() as f64 / 1_048_576.0);
    }

    eprintln!("Download complete → {}", model_dir.display());

    if !args.no_activate {
        core.set_active_model(spec.name)?;
        eprintln!("Active model set to '{}'. Restart pulse to load it.", spec.name);
    }

    Ok(())
}

async fn cmd_vision_download(args: AiVisionDownloadArgs, core: &PulseCore) -> anyhow::Result<()> {
    if args.list {
        println!("{:<30}  {:<8}  {}", "NAME", "SIZE", "DESCRIPTION");
        for m in KNOWN_VISION_MODELS {
            println!("{:<30}  ~{:>4} MB  {}", m.name, m.size_mb_approx, m.description);
        }
        return Ok(());
    }

    let model_name = args.name.as_deref().unwrap_or("clip-vit-b32");
    let spec = match KNOWN_VISION_MODELS.iter().find(|m| m.name == model_name) {
        Some(s) => s,
        None => {
            print_error(&format!(
                "unknown vision model '{}' — run 'pulse ai vision-download --list' to see options",
                model_name
            ));
            return Ok(());
        }
    };

    let model_dir = core.model_dir(spec.name);
    std::fs::create_dir_all(&model_dir)?;

    eprintln!(
        "Downloading {} (~{} MB) from huggingface.co/{}/{} ...",
        spec.name, spec.size_mb_approx, spec.hf_owner, spec.hf_repo
    );

    let client = Client::builder()
        .user_agent("Pulse/0.1 model-downloader")
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    for (hf_path, local_name) in spec.files {
        let url = format!(
            "https://huggingface.co/{}/{}/resolve/main/{}",
            spec.hf_owner, spec.hf_repo, hf_path
        );
        let dest = model_dir.join(local_name);

        eprint!("  {} ... ", local_name);
        std::io::Write::flush(&mut std::io::stderr())?;

        let resp = client.get(&url).send().await
            .map_err(|e| anyhow::anyhow!("network error fetching {}: {}", local_name, e))?;

        if !resp.status().is_success() {
            anyhow::bail!("HTTP {} for {}: {}", resp.status(), local_name, url);
        }

        let bytes = resp.bytes().await
            .map_err(|e| anyhow::anyhow!("read error for {}: {}", local_name, e))?;

        std::fs::write(&dest, &bytes)?;
        eprintln!("{:.1} MB", bytes.len() as f64 / 1_048_576.0);
    }

    eprintln!("Download complete → {}", model_dir.display());
    eprintln!();
    eprintln!("Next: generate label embeddings (required before use):");
    eprintln!("  python3 scripts/compute_clip_labels.py --model-dir {}", model_dir.display());
    eprintln!();
    eprintln!("Then activate:");
    eprintln!("  pulse ai model set {} --vision", spec.name);
    eprintln!("  (or run 'pulse ai vision-download' again after generating embeddings)");

    if !args.no_activate {
        let embeddings_exist = model_dir.join("label_embeddings.bin").exists();
        if embeddings_exist {
            core.set_active_vision_model(spec.name)?;
            eprintln!("Active vision model set to '{}'. Restart pulse to load it.", spec.name);
        } else {
            eprintln!("(skipping auto-activate: label_embeddings.bin not yet generated)");
        }
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
