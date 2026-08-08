use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use chrono::TimeZone;
use pulse_core::types::{
    Feed, FeedGroup, FeedType, ItemStatePatch, TimelineCursor, TimelineFilter,
};
use tauri::State;
use uuid::Uuid;

use crate::AppState;
use crate::models::*;

// ── Share / feed detection commands ───────────────────────────────────────────

#[tauri::command]
pub async fn detect_feed(url: String) -> Result<FeedCandidateDto, String> {
    let client = reqwest::Client::builder()
        .user_agent("Pulse/1.0 feed-detector")
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|e| e.to_string())?;

    let candidate = pulse_core::feeds::detect_feed_url(&client, &url)
        .await
        .map_err(|e| e.to_string())?;

    Ok(FeedCandidateDto {
        feed_url: candidate.feed_url,
        kind: candidate.kind,
        name: candidate.name,
        is_direct_feed: candidate.is_direct_feed,
        is_hn: candidate.is_hn,
        no_feed_found: candidate.no_feed_found,
        candidates: candidate
            .candidates
            .into_iter()
            .map(|c| FeedLinkDto {
                url: c.url,
                title: c.title,
            })
            .collect(),
    })
}

#[tauri::command]
pub async fn get_pending_share() -> Result<Option<String>, String> {
    // Read the JNI-buffered slot (the same store onShareUrl writes to).
    let pending = crate::PENDING_SHARE
        .get()
        .and_then(|m| m.lock().ok())
        .and_then(|mut g| g.take());
    Ok(pending)
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn domain_of(url: &str) -> String {
    reqwest::Url::parse(url)
        .map(|u| u.host_str().unwrap_or("").replace("www.", ""))
        .unwrap_or_default()
}

fn adapt_feed(feed: &Feed, unread: i64, item_count: i64) -> SourceDto {
    let kind = feed.feed_type.as_str().to_string();
    let name = feed.title.clone().unwrap_or_else(|| domain_of(&feed.url));
    let last_sync = feed.last_success_at.map(|ts| {
        chrono::Utc
            .timestamp_opt(ts, 0)
            .single()
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default()
    });
    SourceDto {
        id: feed.id.clone(),
        name,
        url: feed.url.clone(),
        kind,
        group: feed.group_id.clone().unwrap_or_else(|| "all".into()),
        unread,
        item_count,
        avg_latency_ms: feed.avg_latency_ms,
        last_sync,
        enabled: feed.is_enabled,
        failure_streak: feed.failure_streak,
        hue: feed.hue,
    }
}

fn adapt_item(view: &pulse_core::types::FeedItemView) -> FeedItemDto {
    // Decode Reddit HTML entities in body_html so the frontend can render it safely.
    let body_html = view.body_html.as_deref().map(|h| {
        h.replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&nbsp;", "\u{00A0}")
    });
    FeedItemDto {
        id: view.id.clone(),
        source_id: view.feed_id.clone(),
        source_name: view.feed_title.clone().unwrap_or_default(),
        title: view.title.clone(),
        url: view.url.clone().unwrap_or_default(),
        body: view.body_text.clone().unwrap_or_default(),
        body_html,
        external_url: view.external_url.clone(),
        author: view.author.clone(),
        published_at: chrono::Utc
            .timestamp_opt(view.published_at, 0)
            .single()
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default(),
        saved_at: view.saved_at.and_then(|ts| {
            chrono::Utc
                .timestamp_opt(ts, 0)
                .single()
                .map(|dt| dt.to_rfc3339())
        }),
        read: view.is_read,
        saved: view.is_saved,
        hidden: view.is_hidden,
        score: view.score,
        n: view.comment_count.unwrap_or(0),
        tags: view.ai_tags.clone(),
        user_tags: view.user_tags.clone(),
        og_image: view.og_image.clone(),
        note: view.note.clone(),
    }
}

fn adapt_preview_item(item: &pulse_core::types::PreviewItem) -> PreviewItemDto {
    PreviewItemDto {
        id: item.id.clone(),
        title: item.title.clone(),
        url: item.url.clone(),
        author: item.author.clone(),
        published_at: item.published_at.and_then(|ts| {
            chrono::Utc
                .timestamp_opt(ts, 0)
                .single()
                .map(|dt| dt.to_rfc3339())
        }),
    }
}

fn settings_path(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("tauri_settings.json")
}

fn load_settings(data_dir: &std::path::Path) -> AppSettingsDto {
    let path = settings_path(data_dir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_settings_to_disk(
    data_dir: &std::path::Path,
    settings: &AppSettingsDto,
) -> anyhow::Result<()> {
    let path = settings_path(data_dir);
    let json = serde_json::to_string_pretty(settings)?;
    std::fs::write(&path, json)?;
    Ok(())
}

// ── Source commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_sources(state: State<'_, AppState>) -> Result<Vec<SourceDto>, String> {
    let core = state.core().await?;
    let feeds = core.get_feeds().await.map_err(|e| e.to_string())?;
    let unread_map = core.get_unread_counts_by_feed().await.unwrap_or_default();
    let total_map = core.get_total_counts_by_feed().await.unwrap_or_default();
    let dtos: Vec<SourceDto> = feeds
        .iter()
        .map(|f| {
            let unread = *unread_map.get(&f.id).unwrap_or(&0);
            let item_count = *total_map.get(&f.id).unwrap_or(&0);
            adapt_feed(f, unread, item_count)
        })
        .collect();
    Ok(dtos)
}

#[tauri::command]
pub async fn add_source(state: State<'_, AppState>, source: SourceDto) -> Result<(), String> {
    let core = state.core().await?;
    let now = chrono::Utc::now().timestamp();
    let feed_type = match source.kind.as_str() {
        "reddit" => FeedType::Reddit,
        "hn" => FeedType::Hn,
        _ => FeedType::Rss,
    };
    let id = if source.id.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        source.id.clone()
    };
    let feed = Feed {
        id,
        url: source.url.clone(),
        feed_type,
        title: Some(source.name.clone()),
        description: None,
        site_url: None,
        icon_url: None,
        group_id: Some(source.group.clone()).filter(|g| !g.is_empty() && g != "all"),
        poll_interval_secs: 3600,
        is_enabled: true,
        etag: None,
        last_modified: None,
        last_fetched_at: None,
        last_success_at: None,
        last_item_at: None,
        failure_streak: 0,
        total_fetches: 0,
        total_failures: 0,
        avg_latency_ms: None,
        next_fetch_at: None,
        source_config: serde_json::json!({}),
        language: None,
        hue: source.hue,
        created_at: now,
        updated_at: now,
    };
    core.add_feed(feed).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_source(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let core = state.core().await?;
    core.delete_feed(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_source(
    state: State<'_, AppState>,
    id: String,
    name: String,
    url: String,
    kind: String,
    group: String,
    hue: Option<i64>,
) -> Result<(), String> {
    let core = state.core().await?;
    let existing = core.get_feed(&id).await.map_err(|e| e.to_string())?;
    let feed_type = match kind.as_str() {
        "reddit" => FeedType::Reddit,
        "hn" => FeedType::Hn,
        _ => FeedType::Rss,
    };
    let now = chrono::Utc::now().timestamp();
    let updated = Feed {
        url,
        feed_type,
        title: Some(name),
        group_id: Some(group).filter(|g| !g.is_empty() && g != "all"),
        hue,
        updated_at: now,
        ..existing
    };
    core.db
        .upsert_feed(updated)
        .await
        .map_err(|e| e.to_string())?;
    // URL/kind edits must apply now: respawn the task or trigger an immediate refresh.
    core.scheduler.refresh_feed(id.clone()).await;
    Ok(())
}

// ── Item commands ──────────────────────────────────────────────────────────────

/// Cursor input from the frontend for paginated timeline requests.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorInput {
    pub published_at: i64,
    pub item_id: String,
}

/// Fetch a single item by ID (full view). Errors if the item doesn't exist.
#[tauri::command]
pub async fn get_item(state: State<'_, AppState>, item_id: String) -> Result<FeedItemDto, String> {
    let core = state.core().await?;
    let view = core
        .get_item_view(&item_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(adapt_item(&view))
}

/// Paginated timeline command. Returns up to `limit` items starting after `cursor`.
/// The response includes `nextCursor` when more items exist beyond this page.
#[tauri::command]
pub async fn get_items_page(
    state: State<'_, AppState>,
    group_id: Option<String>,
    feed_id: Option<String>,
    tag: Option<String>,
    is_read: Option<bool>,
    is_saved: Option<bool>,
    limit: Option<usize>,
    cursor: Option<CursorInput>,
) -> Result<ItemPageDto, String> {
    let core = state.core().await?;
    let limit = limit.unwrap_or(100);
    let filter = TimelineFilter {
        group_id: group_id.filter(|g| g != "all"),
        feed_id,
        tag,
        is_read,
        is_saved,
    };
    let tauri_cursor = cursor.map(|c| TimelineCursor {
        published_at: c.published_at,
        id: c.item_id,
    });
    let page = core
        .get_timeline_page(filter, tauri_cursor, limit)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ItemPageDto {
        items: page.items.iter().map(adapt_item).collect(),
        next_cursor: page.next_cursor.map(|c| CursorDto {
            published_at: c.published_at,
            item_id: c.id,
        }),
        counts: TimelineCountsDto {
            total: page.counts.total,
            unread: page.counts.unread,
            saved: page.counts.saved,
        },
    })
}

#[tauri::command]
pub async fn mark_items_read(
    state: State<'_, AppState>,
    ids: Vec<String>,
    read: bool,
) -> Result<(), String> {
    let core = state.core().await?;
    for id in &ids {
        core.update_item_state(
            id,
            ItemStatePatch {
                is_read: Some(read),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn mark_source_read(state: State<'_, AppState>, source_id: String) -> Result<(), String> {
    let core = state.core().await?;
    core.mark_feed_read(&source_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_saved(
    state: State<'_, AppState>,
    id: String,
    saved: bool,
) -> Result<(), String> {
    let core = state.core().await?;
    core.toggle_saved(&id, saved)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_item_note(
    state: State<'_, AppState>,
    id: String,
    note: Option<String>,
) -> Result<(), String> {
    let core = state.core().await?;
    core.set_item_note(&id, note)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_user_tags(state: State<'_, AppState>, id: String) -> Result<Vec<String>, String> {
    let core = state.core().await?;
    core.get_user_tags(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_user_tags(
    state: State<'_, AppState>,
    id: String,
    tags: Vec<String>,
) -> Result<(), String> {
    let core = state.core().await?;
    core.set_user_tags(&id, tags)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn hide_item(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let core = state.core().await?;
    core.hide_item(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_popular_feeds() -> Result<Vec<PopularCategoryDto>, String> {
    // Preserve first-seen category order (catalog order) — a HashMap would be
    // nondeterministic across launches.
    let mut cats: Vec<PopularCategoryDto> = Vec::new();
    for f in pulse_core::onboarding::POPULAR_FEEDS {
        if let Some(cat) = cats.iter_mut().find(|c| c.category == f.category) {
            cat.feeds.push(PopularFeedDto {
                name: f.name.to_string(),
                url: f.url.to_string(),
                kind: f.kind.as_str().to_string(),
            });
        } else {
            cats.push(PopularCategoryDto {
                category: f.category.to_string(),
                experimental: f.category == "Mailing Lists",
                feeds: vec![PopularFeedDto {
                    name: f.name.to_string(),
                    url: f.url.to_string(),
                    kind: f.kind.as_str().to_string(),
                }],
            });
        }
    }
    Ok(cats)
}

#[tauri::command]
pub async fn add_onboard_feeds(
    state: State<'_, AppState>,
    selections: Vec<OnboardSelectionDto>,
) -> Result<usize, String> {
    let core = state.core().await?;
    let sels: Vec<pulse_core::onboarding::OnboardSelection> = selections
        .into_iter()
        .map(|s| pulse_core::onboarding::OnboardSelection {
            name: s.name,
            url: s.url,
            kind: match s.kind.as_str() {
                "reddit" => FeedType::Reddit,
                "hn" => FeedType::Hn,
                _ => FeedType::Rss,
            },
            category: s.category,
        })
        .collect();
    core.add_onboard_feeds(&sels)
        .await
        .map_err(|e| e.to_string())
}

/// Fetch a feed's current items without subscribing (discover preview).
#[tauri::command]
pub async fn preview_feed(
    state: State<'_, AppState>,
    url: String,
    kind: String,
    limit: Option<usize>,
) -> Result<Vec<PreviewItemDto>, String> {
    let core = state.core().await?;
    let feed_type = pulse_core::types::FeedType::from_str(&kind)
        .map_err(|e| format!("invalid feed kind: {e}"))?;
    let items = core
        .preview_feed(&url, feed_type, limit.unwrap_or(30))
        .await
        .map_err(|e| e.to_string())?;
    Ok(items.iter().map(adapt_preview_item).collect())
}
// ── Group commands ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_groups(state: State<'_, AppState>) -> Result<Vec<GroupDto>, String> {
    let core = state.core().await?;
    let groups = core.get_feed_groups().await.map_err(|e| e.to_string())?;
    let unread_map = core.get_unread_counts_by_feed().await.unwrap_or_default();

    let feeds = core.get_feeds().await.map_err(|e| e.to_string())?;
    let mut group_unread: HashMap<String, i64> = HashMap::new();
    for feed in &feeds {
        let n = *unread_map.get(&feed.id).unwrap_or(&0);
        if let Some(gid) = &feed.group_id {
            *group_unread.entry(gid.clone()).or_default() += n;
        }
        // Every feed contributes to the "all" pseudo-group
        *group_unread.entry("all".to_string()).or_default() += n;
    }

    let mut dtos: Vec<GroupDto> = Vec::new();

    // "All" pseudo-group is synthesized, not stored in the DB
    let total_unread = *group_unread.get("all").unwrap_or(&0);
    dtos.push(GroupDto {
        id: "all".into(),
        name: "All".into(),
        n: total_unread,
    });

    for g in &groups {
        let n = *group_unread.get(&g.id).unwrap_or(&0);
        dtos.push(GroupDto {
            id: g.id.clone(),
            name: g.name.clone(),
            n,
        });
    }

    Ok(dtos)
}

/// Per-group recent items for the overview screen.
#[tauri::command]
pub async fn get_overview(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<GroupOverviewDto>, String> {
    let core = state.core().await?;
    let overview = core
        .get_overview(limit.unwrap_or(8))
        .await
        .map_err(|e| e.to_string())?;
    Ok(overview
        .iter()
        .map(|o| GroupOverviewDto {
            group_id: o.group_id.clone(),
            group_name: o.group_name.clone(),
            total_items: o.total_items,
            unread_count: o.unread_count,
            items: o.items.iter().map(adapt_item).collect(),
        })
        .collect())
}

#[tauri::command]
pub async fn add_group(state: State<'_, AppState>, id: String, name: String) -> Result<(), String> {
    let core = state.core().await?;
    let now = chrono::Utc::now().timestamp();
    let group = FeedGroup {
        id,
        name,
        description: None,
        color: None,
        sort_order: 100,
        created_at: now,
        updated_at: now,
    };
    core.db
        .insert_feed_group(group)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_group(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> Result<(), String> {
    let core = state.core().await?;
    // Preserve other fields; upsert with the new name
    let groups = core.get_feed_groups().await.map_err(|e| e.to_string())?;
    let existing = groups
        .into_iter()
        .find(|g| g.id == id)
        .ok_or_else(|| format!("group '{}' not found", id))?;
    let now = chrono::Utc::now().timestamp();
    let updated = FeedGroup {
        name,
        updated_at: now,
        ..existing
    };
    core.db
        .insert_feed_group(updated)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_group(state: State<'_, AppState>, id: String) -> Result<(), String> {
    if id == "all" {
        return Ok(());
    }
    let core = state.core().await?;
    core.delete_feed_group(&id).await.map_err(|e| e.to_string())
}

// ── Sync commands ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn sync_source(
    state: State<'_, AppState>,
    source_id: String,
) -> Result<SyncResultDto, String> {
    let core = state.core().await?;
    let result = match core.sync_feed(&source_id).await {
        Ok(n) => SyncResultDto {
            new_count: n as i64,
            error: None,
        },
        Err(e) => SyncResultDto {
            new_count: 0,
            error: Some(e.to_string()),
        },
    };
    if result.error.is_none() {
        let core2 = Arc::clone(&core);
        let sid = source_id.clone();
        tokio::spawn(async move {
            if let Err(e) = core2.enrich_pending(Some(&sid), 50, 4, |_| {}).await {
                tracing::warn!(feed_id = %sid, error = %e, "enrichment failed");
            }
        });
    }
    Ok(result)
}

#[tauri::command]
pub async fn sync_all(state: State<'_, AppState>) -> Result<SyncResultDto, String> {
    let core = state.core().await?;
    let feeds = core.get_feeds().await.map_err(|e| e.to_string())?;
    let mut total_new = 0i64;

    let handles: Vec<_> = feeds
        .iter()
        .map(|f| {
            let core = Arc::clone(&core);
            let fid = f.id.clone();
            tokio::spawn(async move { core.sync_feed(&fid).await })
        })
        .collect();

    for handle in handles {
        match handle.await {
            Ok(Ok(n)) => total_new += n as i64,
            Ok(Err(e)) => tracing::warn!(error = %e, "feed sync error"),
            Err(e) => tracing::error!(error = %e, "feed sync task panicked"),
        }
    }

    let core2 = Arc::clone(&core);
    tokio::spawn(async move {
        if let Err(e) = core2.enrich_pending(None, 200, 4, |_| {}).await {
            tracing::warn!(error = %e, "enrichment failed");
        }
    });

    Ok(SyncResultDto {
        new_count: total_new,
        error: None,
    })
}

// ── Settings commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettingsDto, String> {
    Ok(load_settings(&state.data_dir))
}

#[tauri::command]
pub async fn save_settings(
    state: State<'_, AppState>,
    settings: AppSettingsDto,
) -> Result<(), String> {
    save_settings_to_disk(&state.data_dir, &settings).map_err(|e| e.to_string())
}

// ── Search ────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn search_items(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
    sort: Option<String>,
) -> Result<Vec<FeedItemDto>, String> {
    let core = state.core().await?;
    let views = core
        .search(&query, limit, sort.as_deref().unwrap_or("relevance"))
        .await
        .map_err(|e| e.to_string())?;
    Ok(views.iter().map(adapt_item).collect())
}

// ── Stats commands ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_db_stats(state: State<'_, AppState>) -> Result<DbStatsDto, String> {
    let core = state.core().await?;
    let stats = core.get_db_stats().await.map_err(|e| e.to_string())?;
    Ok(DbStatsDto {
        total_items: stats.item_count,
        unread_items: stats.unread_count,
        saved_items: stats.saved_count,
        total_sources: stats.feed_count,
        db_size_kb: stats.db_size_bytes / 1024,
        tag_count: stats.tag_count,
    })
}

// ── Tag commands ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_tag_stats(state: State<'_, AppState>) -> Result<TagStatsDto, String> {
    let core = state.core().await?;
    let stats = core.get_tag_stats().await.map_err(|e| e.to_string())?;
    Ok(TagStatsDto {
        tagged_count: stats.tagged_count,
        tag_counts: stats.tag_counts,
    })
}

// ── Diagnostics commands ───────────────────────────────────────────────────────

/// Update the tracing filter level at runtime — no restart required.
/// Called by the frontend when the user toggles "Verbose logging" in settings.
#[tauri::command]
pub fn set_log_level(state: State<'_, AppState>, verbose: bool) -> Result<(), String> {
    let directive = crate::log_directive(verbose);
    state
        .log_filter
        .modify(|f| *f = tracing_subscriber::EnvFilter::new(directive))
        .map_err(|e| e.to_string())
}

/// Return the last `lines` lines of the most recent log file.
/// Used by the mobile "Share logs" flow.
#[tauri::command]
pub async fn get_log_content(
    state: State<'_, AppState>,
    lines: Option<usize>,
) -> Result<String, String> {
    let log_dir = state.data_dir.join("logs");
    let max_lines = lines.unwrap_or(500);

    let log_file = find_most_recent_log(&log_dir).ok_or_else(|| {
        "No log file found yet — try again after the app has been running.".to_string()
    })?;

    let content = std::fs::read_to_string(&log_file).map_err(|e| e.to_string())?;

    let collected: Vec<&str> = content.lines().collect();
    let start = collected.len().saturating_sub(max_lines);
    Ok(collected[start..].join("\n"))
}

#[tauri::command]
pub fn get_log_path(state: State<'_, AppState>) -> String {
    state.data_dir.join("logs").to_string_lossy().to_string()
}

#[tauri::command]
pub fn open_logs_folder(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let log_dir = state.data_dir.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    app.opener()
        .open_path(log_dir.to_string_lossy().as_ref(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// Copy the most recent log file to the app cache directory and share it via
/// the native share sheet (Android). On desktop, opens the original log file
/// in the default text editor.
#[tauri::command]
pub fn share_log_file(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let log_dir = state.data_dir.join("logs");
    let log_file = find_most_recent_log(&log_dir).ok_or_else(|| {
        "No log file found yet — try again after the app has been running.".to_string()
    })?;

    #[cfg(target_os = "android")]
    {
        use tauri::Manager;
        let cache_dir = match app.path().app_cache_dir() {
            Ok(d) => d,
            Err(e) => return Err(format!("cache dir unavailable: {e}")),
        };
        std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;

        let dest = cache_dir.join("pulse-debug-logs.txt");
        std::fs::copy(&log_file, &dest).map_err(|e| format!("failed to copy log: {e}"))?;
        let dest_str = dest.to_string_lossy().to_string();

        if let Some(vm) = crate::ANDROID_VM.get() {
            let Ok(_) = vm.attach_current_thread(|env: &mut jni::Env| -> jni::errors::Result<()> {
                let class =
                    env.find_class(jni::jni_str!("com/avinthakur080/pulse_rs/ShareBridge"))?;
                let jpath = env.new_string(&dest_str)?;
                env.call_static_method(
                    class,
                    jni::jni_str!("shareFile"),
                    jni::jni_sig!("(Ljava/lang/String;)V"),
                    &[jni::objects::JValue::Object(&jpath)],
                )?;
                tracing::info!(path = %dest_str, "share_log_file: shared via Android intent");
                Ok(())
            }) else {
                return Err("JNI: cannot attach thread".into());
            };
            return Ok(());
        }
        return Err("JNI: Android VM not available (app not fully initialized)".into());
    }

    #[cfg(not(target_os = "android"))]
    {
        use tauri_plugin_opener::OpenerExt;
        let log_path = log_file.to_string_lossy().to_string();
        app.opener()
            .open_path(&log_path, None::<&str>)
            .map_err(|e| e.to_string())
    }
}

fn find_most_recent_log(log_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    std::fs::read_dir(log_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("pulse.log."))
        .max_by_key(|e| {
            e.metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        })
        .map(|e| e.path())
}

// ── Frontend logging bridge ────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum FrontendLogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

/// Route a log event from the JS/TS frontend into the Rust tracing subscriber
/// so it lands in the same log file as backend events.
#[tauri::command]
pub fn log_from_frontend(level: FrontendLogLevel, message: String, context: Option<String>) {
    match level {
        FrontendLogLevel::Error => {
            tracing::error!(target: "pulse_frontend", context = ?context, "{}", message)
        }
        FrontendLogLevel::Warn => {
            tracing::warn!(target: "pulse_frontend", context = ?context, "{}", message)
        }
        FrontendLogLevel::Info => {
            tracing::info!(target: "pulse_frontend", context = ?context, "{}", message)
        }
        FrontendLogLevel::Debug => {
            tracing::debug!(target: "pulse_frontend", context = ?context, "{}", message)
        }
    }
}
