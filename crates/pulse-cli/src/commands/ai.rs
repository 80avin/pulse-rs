use clap::{Args, Subcommand};
use pulse_core::PulseCore;
use pulse_core::ai::{RuleScope, default_rules};
use reqwest::Client;

use crate::output::{print_error, print_json};

// ── Known model registry ───────────────────────────────────────────────────────

struct VisionModelSpec {
    name: &'static str,
    description: &'static str,
    hf_owner: &'static str,
    hf_repo: &'static str,
    /// (hf_path, local_filename)
    files: &'static [(&'static str, &'static str)],
    size_mb_approx: u32,
}

const KNOWN_VISION_MODELS: &[VisionModelSpec] = &[
    VisionModelSpec {
        name: "mobileclip-s2",
        description: "Apple MobileCLIP-S2 — 37 MB int8 vision encoder, better zero-shot than CLIP ViT-B/32 (recommended)",
        hf_owner: "Xenova",
        hf_repo: "mobileclip_s2",
        files: &[
            (
                "onnx/vision_model_quantized.onnx",
                "vision_model_quantized.onnx",
            ),
            (
                "onnx/text_model_quantized.onnx",
                "text_model_quantized.onnx",
            ),
            ("tokenizer.json", "tokenizer.json"),
            ("preprocessor_config.json", "preprocessor_config.json"),
        ],
        size_mb_approx: 103,
    },
    VisionModelSpec {
        name: "mobileclip-s1",
        description: "Apple MobileCLIP-S1 — 22 MB int8 vision encoder, fastest option for Android",
        hf_owner: "Xenova",
        hf_repo: "mobileclip_s1",
        files: &[
            (
                "onnx/vision_model_quantized.onnx",
                "vision_model_quantized.onnx",
            ),
            (
                "onnx/text_model_quantized.onnx",
                "text_model_quantized.onnx",
            ),
            ("tokenizer.json", "tokenizer.json"),
            ("preprocessor_config.json", "preprocessor_config.json"),
        ],
        size_mb_approx: 89,
    },
    VisionModelSpec {
        name: "clip-vit-b32",
        description: "CLIP ViT-B/32 vision encoder — legacy, 53 MB q4f16 (use mobileclip-s2 instead)",
        hf_owner: "Xenova",
        hf_repo: "clip-vit-base-patch32",
        files: &[
            ("onnx/vision_model_q4f16.onnx", "vision_model_q4f16.onnx"),
            (
                "onnx/text_model_quantized.onnx",
                "text_model_quantized.onnx",
            ),
            ("tokenizer.json", "tokenizer.json"),
        ],
        size_mb_approx: 78,
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
    /// Show raw CLIP cosine scores for an image URL (vision threshold calibration)
    VisionDebug(AiVisionDebugArgs),
    /// Download a CLIP vision model from HuggingFace and make it active
    VisionDownload(AiVisionDownloadArgs),
    /// Label items interactively for supervised training
    Label(AiLabelArgs),
    /// Manage training data
    Train(AiTrainArgs),
    /// Manage tag rules (rule-based fallback)
    Rules(AiRulesArgs),
}

#[derive(Debug, Args)]
pub struct AiRunArgs {
    /// Limit to a specific feed
    #[arg(long)]
    pub feed: Option<String>,

    /// Re-tag ALL items, clearing existing tags first (use after vocabulary changes)
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct AiStatusArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

// ── Label subcommand ──────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct AiLabelArgs {
    /// Limit to items from a specific feed (substring match on feed title/URL)
    #[arg(long)]
    pub feed: Option<String>,
    /// Stop after labeling N items
    #[arg(long)]
    pub limit: Option<usize>,
    /// Show already-labeled items for review/correction
    #[arg(long)]
    pub review: bool,
}

// ── Train subcommand ──────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct AiTrainArgs {
    #[command(subcommand)]
    pub command: AiTrainCommand,
}

#[derive(Debug, Subcommand)]
pub enum AiTrainCommand {
    /// Show labeling statistics
    Stats,
    /// Export labels in FastText supervised format
    ExportFasttext(AiTrainExportArgs),
    /// Export labels as JSONL for MiniLM fine-tuning
    ExportJsonl(AiTrainExportArgs),
}

#[derive(Debug, Args)]
pub struct AiTrainExportArgs {
    /// Output file path (default: {data_dir}/training/train.txt or train.jsonl)
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,
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
pub struct AiVisionDebugArgs {
    /// Image URL to classify via CLIP
    pub url: String,
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
        AiCommand::VisionDebug(a) => cmd_vision_debug(a, core).await,
        AiCommand::VisionDownload(a) => cmd_vision_download(a, core).await,
        AiCommand::Label(a) => cmd_label(a, core).await,
        AiCommand::Train(a) => cmd_train(a, core).await,
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
            None => {
                print_error(&format!("feed '{}' not found", prefix));
                return Ok(());
            }
        }
    } else {
        None
    };

    let mode = match (
        core.fasttext_loaded(),
        core.miniml_loaded(),
        core.vision_loaded(),
    ) {
        (true, true, true) => "fasttext+miniml+vision",
        (true, true, false) => "fasttext+miniml",
        (true, false, true) => "fasttext+vision",
        (true, false, false) => "fasttext",
        (false, _, true) => "vision",
        (false, _, false) => "rule-engine",
    };
    if args.force {
        eprintln!("running tagger ({}) — force-retagging ALL items...", mode);
    } else {
        eprintln!("running tagger ({}) on untagged items...", mode);
    }
    let (items, tags) = core
        .run_tagger_direct(feed_id.as_deref(), args.force, None)
        .await?;
    eprintln!("tagged {} items, {} tags created", items, tags);
    Ok(())
}

pub async fn cmd_label(args: AiLabelArgs, core: &PulseCore) -> anyhow::Result<()> {
    use pulse_core::training::{LabelStore, LabeledItem, build_input_text};
    use std::io::{BufRead, Write};

    let label_store = LabelStore::new(&core.config.training_dir())?;
    let already_labeled = label_store.labeled_ids()?;

    // Load items from timeline
    let filter = pulse_core::types::TimelineFilter {
        feed_id: None,
        ..Default::default()
    };
    let page = core.get_timeline_page(filter, None, 2000).await?;

    let items: Vec<_> = if args.review {
        // Show already-labeled items
        page.items
            .into_iter()
            .filter(|i| already_labeled.contains(i.id.as_str()))
            .collect()
    } else {
        // Show unlabeled items
        page.items
            .into_iter()
            .filter(|i| !already_labeled.contains(i.id.as_str()))
            .collect()
    };

    // Apply feed filter if given
    let items: Vec<_> = if let Some(ref feed_filter) = args.feed {
        let ff = feed_filter.to_lowercase();
        items
            .into_iter()
            .filter(|i| {
                i.feed_title
                    .as_deref()
                    .map(|t| t.to_lowercase().contains(&ff))
                    .unwrap_or(false)
                    || i.url
                        .as_deref()
                        .map(|u| u.to_lowercase().contains(&ff))
                        .unwrap_or(false)
            })
            .collect()
    } else {
        items
    };

    let total = items.len().min(args.limit.unwrap_or(usize::MAX));
    if total == 0 {
        eprintln!("No items to label (try syncing first with `pulse sync run`).");
        return Ok(());
    }

    // Known tags for reference
    let known_tags = [
        "technical",
        "tutorial",
        "research",
        "news",
        "discussion",
        "security",
        "ai-ml",
        "privacy",
        "policy",
        "science",
        "clickbait",
        "show-hn",
        "ask-hn",
        "job-posting",
        "paywall",
        "video",
        "low-effort",
    ];

    let stdin = std::io::stdin();
    let mut labeled = 0usize;

    eprintln!("Known tags: {}", known_tags.join(", "));
    eprintln!("Commands: Enter=skip, q=quit, tags=comma-separated");
    eprintln!();

    for (i, item) in items.iter().take(total).enumerate() {
        let domain = item
            .url
            .as_deref()
            .and_then(|u| u.split("://").nth(1))
            .and_then(|s| s.split('/').next())
            .map(|h| h.trim_start_matches("www."))
            .unwrap_or("unknown");

        let source = item.feed_title.as_deref().unwrap_or("?");
        eprintln!("[{}/{}] {} — {}", i + 1, total, domain, source);
        eprintln!("  {}", item.title);

        // Show existing labels if in review mode
        if args.review && !item.ai_tags.is_empty() {
            eprintln!("  Current: {}", item.ai_tags.join(", "));
        }

        print!("  Tags> ");
        std::io::stdout().flush()?;

        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        let line = line.trim();

        if line == "q" || line == "quit" {
            eprintln!("Quit. Labeled {} items.", labeled);
            break;
        }
        if line.is_empty() {
            continue;
        }

        let tags: Vec<String> = line
            .split(',')
            .map(|t| t.trim().to_lowercase())
            .filter(|t| !t.is_empty())
            .collect();

        let text = build_input_text(&item.title, item.url.as_deref());
        let now = chrono::Utc::now().timestamp();

        label_store.upsert(LabeledItem {
            item_id: item.id.to_string(),
            text,
            tags,
            labeled_at: now,
        })?;

        labeled += 1;

        let stats = label_store.stats()?;
        eprintln!("  Saved. ({} total labeled)", stats.total);
    }

    eprintln!();
    eprintln!("Session complete: {} items labeled.", labeled);
    let stats = label_store.stats()?;
    eprintln!("Total in store: {}", stats.total);
    for (tag, count) in stats.tag_counts.iter() {
        eprintln!("  {}: {}", tag, count);
    }

    Ok(())
}

pub async fn cmd_train(args: AiTrainArgs, core: &PulseCore) -> anyhow::Result<()> {
    use pulse_core::training::LabelStore;

    let label_store = LabelStore::new(&core.config.training_dir())?;

    match args.command {
        AiTrainCommand::Stats => {
            let stats = label_store.stats()?;
            println!(
                "Label store: {}",
                core.config.training_dir().join("labels.jsonl").display()
            );
            println!("Total examples: {}", stats.total);
            if !stats.tag_counts.is_empty() {
                println!("Tag distribution:");
                let mut counts: Vec<_> = stats.tag_counts.iter().collect();
                counts.sort_by(|a, b| b.1.cmp(a.1));
                for (tag, count) in counts {
                    println!("  {:20} {}", tag, count);
                }
            }
        }
        AiTrainCommand::ExportFasttext(export_args) => {
            let dest = export_args
                .output
                .unwrap_or_else(|| core.config.training_dir().join("train.txt"));
            let n = label_store.export_fasttext(&dest)?;
            println!("Wrote {} examples to {}", n, dest.display());
            println!();
            println!(
                "Next: python scripts/train_fasttext.py --input {} --output ~/.local/share/pulse/models/fasttext-v1/",
                dest.display()
            );
        }
        AiTrainCommand::ExportJsonl(export_args) => {
            let dest = export_args
                .output
                .unwrap_or_else(|| core.config.training_dir().join("train.jsonl"));
            let n = label_store.export_jsonl(&dest)?;
            println!("Wrote {} examples to {}", n, dest.display());
            println!();
            println!(
                "Next: python scripts/train_miniml.py --input {} --model-dir ~/.local/share/pulse/models/miniml-v1/",
                dest.display()
            );
        }
    }

    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct AiStatus {
    active_vision_model: String,
    active_fasttext_model: String,
    active_miniml_model: String,
    tagging_mode: String,
    rule_count: usize,
    vision_loaded: bool,
    fasttext_loaded: bool,
    miniml_loaded: bool,
}

async fn cmd_status(args: AiStatusArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    let use_json = args.json || global_json;
    let rules = default_rules();
    let enabled = rules.iter().filter(|r| r.enabled).count();
    let vision_loaded = core.vision_loaded();
    let fasttext_loaded = core.fasttext_loaded();
    let miniml_loaded = core.miniml_loaded();
    let active_vision_model = core
        .active_vision_model_name()
        .unwrap_or_else(|| "none".to_string());
    let active_fasttext_model = core
        .active_fasttext_model_name()
        .unwrap_or_else(|| "none".to_string());
    let active_miniml_model = core
        .active_miniml_model_name()
        .unwrap_or_else(|| "none".to_string());
    let ft = core.fasttext_loaded();
    let ml = core.miniml_loaded();
    let tagging_mode = match (ft, ml, vision_loaded) {
        (true, true, true) => "fasttext+miniml+vision",
        (true, true, false) => "fasttext+miniml",
        (true, false, true) => "fasttext+vision",
        (true, false, false) => "fasttext",
        (false, _, true) => "vision",
        _ => "rule-based",
    }
    .to_string();

    let status = AiStatus {
        active_vision_model: active_vision_model.clone(),
        active_fasttext_model: active_fasttext_model.clone(),
        active_miniml_model: active_miniml_model.clone(),
        tagging_mode: tagging_mode.clone(),
        rule_count: enabled,
        vision_loaded,
        fasttext_loaded,
        miniml_loaded,
    };

    if use_json {
        print_json(&status);
        return Ok(());
    }

    let stats = core.get_db_stats().await?;
    println!("Vision model:  {}", active_vision_model);
    println!(
        "FastText:      {}",
        if fasttext_loaded {
            format!("loaded ({})", active_fasttext_model)
        } else {
            format!("not loaded ({})", active_fasttext_model)
        }
    );
    println!(
        "MiniLM:        {}",
        if miniml_loaded {
            format!("loaded ({})", active_miniml_model)
        } else {
            format!("not loaded ({})", active_miniml_model)
        }
    );
    println!("Tagging mode:  {}", tagging_mode);
    println!("Vision loaded: {}", vision_loaded);
    println!("Rules loaded:  {} enabled / {} total", enabled, rules.len());
    println!("Tags in DB:    {}", stats.tag_count);
    Ok(())
}

async fn cmd_vision_debug(args: AiVisionDebugArgs, core: &PulseCore) -> anyhow::Result<()> {
    let vision = core.vision_tagger.snapshot();
    let Some(ref vision) = vision else {
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
                // ✓ marks scores above the minimum semantic threshold (0.20)
                let marker = if *score >= 0.20 { " ✓" } else { "" };
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

async fn cmd_vision_download(args: AiVisionDownloadArgs, core: &PulseCore) -> anyhow::Result<()> {
    if args.list {
        println!("{:<30}  {:<8}  {}", "NAME", "SIZE", "DESCRIPTION");
        for m in KNOWN_VISION_MODELS {
            println!(
                "{:<30}  ~{:>4} MB  {}",
                m.name, m.size_mb_approx, m.description
            );
        }
        return Ok(());
    }

    let model_name = args.name.as_deref().unwrap_or("mobileclip-s2");
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
        .timeout(std::time::Duration::from_secs(600))
        .build()?;

    for (hf_path, local_name) in spec.files {
        let url = format!(
            "https://huggingface.co/{}/{}/resolve/main/{}",
            spec.hf_owner, spec.hf_repo, hf_path
        );
        let dest = model_dir.join(local_name);

        eprint!("  {} ... ", local_name);
        std::io::Write::flush(&mut std::io::stderr())?;

        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("network error fetching {}: {}", local_name, e))?;

        if !resp.status().is_success() {
            anyhow::bail!("HTTP {} for {}: {}", resp.status(), local_name, url);
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| anyhow::anyhow!("read error for {}: {}", local_name, e))?;

        std::fs::write(&dest, &bytes)?;
        eprintln!("{:.1} MB", bytes.len() as f64 / 1_048_576.0);
    }

    eprintln!("Download complete → {}", model_dir.display());

    if !args.no_activate {
        // Delete stale label_embeddings.bin so reload regenerates it with the current label set.
        // This is required when the model or label set changes.
        let embeddings_path = model_dir.join("label_embeddings.bin");
        if embeddings_path.exists() {
            eprintln!(
                "Removing stale label_embeddings.bin (will regenerate for current labels)..."
            );
            std::fs::remove_file(&embeddings_path)?;
        }

        core.set_active_vision_model(spec.name)?;

        eprintln!(
            "Generating label embeddings via {} text encoder...",
            spec.name
        );
        match core.reload_vision_tagger() {
            Ok(()) => {
                eprintln!("Vision model '{}' loaded and ready.", spec.name);
                eprintln!("Calibrate thresholds: pulse ai vision-debug <image_url>");
            }
            Err(e) => {
                eprintln!("Warning: vision model loaded but tagger init failed: {e}");
                eprintln!("Try: pulse ai model set {} --vision", spec.name);
            }
        }
    }

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
