use clap::{Args, Subcommand};
use pulse_core::PulseCore;

use crate::output::{print_json, format_bytes};

#[derive(Debug, Args)]
pub struct DbArgs {
    #[command(subcommand)]
    pub command: DbCommand,
}

#[derive(Debug, Subcommand)]
pub enum DbCommand {
    /// Run pending database migrations
    Migrate(DbMigrateArgs),
    /// Show database statistics
    Stats(DbStatsArgs),
    /// VACUUM the database to reclaim space
    Vacuum,
}

#[derive(Debug, Args)]
pub struct DbMigrateArgs {
    /// Show what would be done without doing it
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct DbStatsArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: DbArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    match args.command {
        DbCommand::Migrate(a) => cmd_migrate(a, core).await,
        DbCommand::Stats(a) => cmd_stats(a, core, global_json).await,
        DbCommand::Vacuum => cmd_vacuum(core).await,
    }
}

async fn cmd_migrate(args: DbMigrateArgs, _core: &PulseCore) -> anyhow::Result<()> {
    if args.dry_run {
        println!("--dry-run: migrations would run on next startup (run_migrations is called at init)");
        return Ok(());
    }
    // Migrations run automatically at PulseCore::init. Since we're already initialized, report.
    println!("migrations already applied at startup");
    Ok(())
}

async fn cmd_stats(args: DbStatsArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    let use_json = args.json || global_json;
    let stats = core.get_db_stats().await?;

    if use_json {
        print_json(&stats);
        return Ok(());
    }

    let db_path = core.config.db_path.display().to_string();
    println!("Database:   {}", db_path);
    println!("Size:       {}", format_bytes(stats.db_size_bytes));
    println!();
    println!("{:<24}  {}", "Metric", "Count");
    println!("{}", "─".repeat(36));
    println!("{:<24}  {}", "feeds (enabled)", stats.feed_count);
    println!("{:<24}  {}", "feed_items", stats.item_count);
    println!("{:<24}  {}", "unread items", stats.unread_count);
    println!("{:<24}  {}", "saved items", stats.saved_count);
    println!("{:<24}  {}", "ai_tags", stats.tag_count);
    Ok(())
}

async fn cmd_vacuum(core: &PulseCore) -> anyhow::Result<()> {
    // Run VACUUM via the reader pool (VACUUM must be done outside transactions)
    let pool = core.db.reader_pool().clone();
    sqlx::query("VACUUM").execute(&pool).await?;
    println!("VACUUM complete");
    Ok(())
}
