use pulse_core::types::FeedType;

/// Print a value as JSON to stdout.
pub fn print_json<T: serde::Serialize>(val: &T) {
    match serde_json::to_string_pretty(val) {
        Ok(s) => println!("{}", s),
        Err(e) => print_error(&format!("failed to serialize JSON: {e}")),
    }
}

/// Print an error message to stderr.
pub fn print_error(msg: &str) {
    eprintln!("error: {msg}");
}

/// Format a Unix timestamp as a human-relative string, e.g. "2m ago", "3h ago", "2d ago".
pub fn relative_time(ts: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let diff = now - ts;

    if diff < 0 {
        return "just now".to_string();
    }

    if diff < 60 {
        return format!("{}s", diff);
    }

    let minutes = diff / 60;
    if minutes < 60 {
        return format!("{}m", minutes);
    }

    let hours = diff / 3600;
    if hours < 24 {
        return format!("{}h", hours);
    }

    let days = diff / 86400;
    if days < 7 {
        return format!("{}d", days);
    }

    let weeks = diff / 604800;
    if weeks < 52 {
        return format!("{}w", weeks);
    }

    let months = diff / 2592000;
    format!("{}mo", months)
}

/// Format a score for display. Right-aligns to 6 chars.
/// Reddit -> ▲NNN, HN -> ★NNN, none -> "     -"
pub fn score_display(score: Option<i64>, feed_type: &FeedType) -> String {
    match score {
        Some(s) => match feed_type {
            FeedType::Reddit => format!("{:>5}", format!("▲{}", s)),
            FeedType::Hn => format!("{:>5}", format!("★{}", s)),
            FeedType::Rss => format!("{:>5}", "-"),
        },
        None => format!("{:>5}", "-"),
    }
}

/// Format bytes into a human-readable size string.
pub fn format_bytes(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = 1024 * KB;
    const GB: i64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Prompt the user for a yes/no confirmation. Returns true if user confirmed.
pub fn confirm(prompt: &str) -> bool {
    use std::io::Write;
    print!("{} [y/N] ", prompt);
    let _ = std::io::stdout().flush();
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}
