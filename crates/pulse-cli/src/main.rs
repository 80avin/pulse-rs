mod commands;
mod output;

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use pulse_core::{PulseCore, config::PulseConfig};
use tracing_subscriber::EnvFilter;

use commands::{
    ai::AiArgs,
    db::DbArgs,
    diag::DiagArgs,
    enrich::EnrichArgs,
    feed::FeedArgs,
    group::GroupArgs,
    item::ItemArgs,
    search::SearchArgs,
    sync::SyncArgs,
    timeline::TimelineArgs,
};

/// Pulse — a local-first feed reader
#[derive(Debug, Parser)]
#[command(name = "pulse", version, about)]
struct Cli {
    /// Override data directory (DB + models). Takes precedence over --db.
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,

    /// Override database path (ignored if --data-dir is set)
    #[arg(long, global = true)]
    db: Option<PathBuf>,

    /// Reddit OAuth2 client ID (script app). Also read from REDDIT_CLIENT_ID env var.
    #[arg(long, global = true, env = "REDDIT_CLIENT_ID")]
    reddit_client_id: Option<String>,

    /// Reddit OAuth2 client secret. Also read from REDDIT_CLIENT_SECRET env var.
    #[arg(long, global = true, env = "REDDIT_CLIENT_SECRET")]
    reddit_client_secret: Option<String>,

    /// Output as JSON (machine-readable)
    #[arg(long, global = true)]
    json: bool,

    /// Suppress informational output
    #[arg(long, short, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Manage feed sources
    Feed(FeedArgs),
    /// Manage feed groups
    Group(GroupArgs),
    /// Browse the unified timeline
    Timeline(TimelineArgs),
    /// Inspect and act on individual items
    Item(ItemArgs),
    /// Search items
    Search(SearchArgs),
    /// Control the sync engine
    Sync(SyncArgs),
    /// Enrich items with Open Graph metadata and crosspost content
    Enrich(EnrichArgs),
    /// AI tagging pipeline management
    Ai(AiArgs),
    /// Database utilities
    Db(DbArgs),
    /// Diagnostics and health
    Diag(DiagArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize tracing from RUST_LOG env var
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    // Build config
    let mut config = if let Some(ref data_dir) = cli.data_dir {
        PulseConfig::default_config().with_data_dir(data_dir.clone())
    } else if let Some(ref db_path) = cli.db {
        PulseConfig::default_config().with_db_path(db_path.clone())
    } else {
        PulseConfig::default_config()
    };

    if let (Some(id), Some(secret)) = (cli.reddit_client_id, cli.reddit_client_secret) {
        config = config.with_reddit_auth(id, secret);
    }

    // Initialize PulseCore
    let core = match PulseCore::init(config).await {
        Ok(c) => c,
        Err(e) => {
            output::print_error(&format!("failed to initialize: {e}"));
            process::exit(1);
        }
    };

    let global_json = cli.json;

    let result = match cli.command {
        Commands::Feed(args) => commands::feed::run(args, &core, global_json).await,
        Commands::Group(args) => commands::group::run(args, &core, global_json).await,
        Commands::Timeline(args) => commands::timeline::run(args, &core, global_json).await,
        Commands::Item(args) => commands::item::run(args, &core, global_json).await,
        Commands::Search(args) => commands::search::run(args, &core, global_json).await,
        Commands::Sync(args) => commands::sync::run(args, &core, global_json).await,
        Commands::Enrich(args) => commands::enrich::run(args, &core, global_json).await,
        Commands::Ai(args) => commands::ai::run(args, &core, global_json).await,
        Commands::Db(args) => commands::db::run(args, &core, global_json).await,
        Commands::Diag(args) => commands::diag::run(args, &core, global_json).await,
    };

    core.shutdown().await;

    if let Err(e) = result {
        output::print_error(&e.to_string());
        process::exit(1);
    }
}
