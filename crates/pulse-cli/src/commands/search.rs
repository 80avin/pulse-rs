use clap::Args;
use pulse_core::PulseCore;

use crate::output::print_json;
use crate::commands::timeline::print_items_human;

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Search query (SQLite FTS5 syntax)
    pub query: String,
    /// Max results
    #[arg(long, default_value = "20")]
    pub limit: usize,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: SearchArgs, core: &PulseCore, global_json: bool) -> anyhow::Result<()> {
    let use_json = args.json || global_json;
    let items = core.search(&args.query, Some(args.limit)).await?;

    if use_json {
        print_json(&items);
        return Ok(());
    }

    if items.is_empty() {
        println!("no results for '{}'", args.query);
        return Ok(());
    }

    print_items_human(&items);
    Ok(())
}
