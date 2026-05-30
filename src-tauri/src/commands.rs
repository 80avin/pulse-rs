use std::collections::HashMap;
use std::sync::Arc;

use chrono::TimeZone;
use pulse_core::types::{
    Feed, FeedGroup, FeedType, ItemStatePatch, TimelineCursor, TimelineFilter,
};
use tauri::{Emitter, State};
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
pub async fn get_pending_share(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let mut lock = state.pending_share.lock().unwrap();
    Ok(lock.take())
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
        read: view.is_read,
        saved: view.is_saved,
        hidden: view.is_hidden,
        score: view.score,
        n: view.comment_count.unwrap_or(0),
        tags: view.ai_tags.clone(),
        og_image: view.og_image.clone(),
        signal: view.signal,
        note: view.note.clone(),
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

// ── Known downloadable models ──────────────────────────────────────────────────

struct ModelSpec {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    hf_owner: &'static str,
    hf_repo: &'static str,
    files: &'static [(&'static str, &'static str)],
    size_mb: u32,
    kind: &'static str,
}

const KNOWN_MODELS: &[ModelSpec] = &[
    ModelSpec {
        id: "clip-vit-b32",
        name: "CLIP ViT-B/32",
        description: "~125 MB — zero-shot image classification for Reddit image posts",
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
        size_mb: 125,
        kind: "vision",
    },
    ModelSpec {
        id: "minilm",
        name: "MiniLM-L6 Semantic Classifier",
        description: "87 MB — semantic tagging for civic, local-rec, culture, research, clickbait, technical",
        hf_owner: "Xenova",
        hf_repo: "all-MiniLM-L6-v2",
        files: &[
            ("onnx/model.onnx", "model.onnx"),
            ("tokenizer.json", "tokenizer.json"),
        ],
        size_mb: 87,
        kind: "miniml",
    },
];

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
        updated_at: now,
        ..existing
    };
    core.db
        .upsert_feed(updated)
        .await
        .map_err(|e| e.to_string())
}

// ── Item commands ──────────────────────────────────────────────────────────────

/// Cursor input from the frontend for paginated timeline requests.
#[derive(serde::Deserialize)]
pub struct CursorInput {
    pub published_at: i64,
    pub item_id: String,
}

/// Paginated timeline command. Returns up to `limit` items starting after `cursor`.
/// The response includes `nextCursor` when more items exist beyond this page.
#[tauri::command]
pub async fn get_items_page(
    state: State<'_, AppState>,
    group_id: Option<String>,
    feed_id: Option<String>,
    tag: Option<String>,
    limit: Option<usize>,
    cursor: Option<CursorInput>,
) -> Result<ItemPageDto, String> {
    let core = state.core().await?;
    let limit = limit.unwrap_or(100);
    let filter = TimelineFilter {
        group_id: group_id.filter(|g| g != "all"),
        feed_id,
        tag,
        ..Default::default()
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
    note: Option<String>,
) -> Result<(), String> {
    let core = state.core().await?;
    core.toggle_saved(&id, saved, note)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn hide_item(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let core = state.core().await?;
    core.hide_item(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_items(state: State<'_, AppState>) -> Result<(), String> {
    let core = state.core().await?;
    core.clear_all_items().await.map_err(|e| e.to_string())
}

// ── Group commands ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_groups(state: State<'_, AppState>) -> Result<Vec<GroupDto>, String> {
    let core = state.core().await?;
    let groups = core.get_feed_groups().await.map_err(|e| e.to_string())?;
    let unread_map = core.get_unread_counts_by_feed().await.unwrap_or_default();

    // Build unread per group by summing across feeds in each group
    let feeds = core.get_feeds().await.map_err(|e| e.to_string())?;
    let mut group_unread: HashMap<String, i64> = HashMap::new();
    for feed in &feeds {
        let g = feed.group_id.as_deref().unwrap_or("all").to_string();
        let n = *unread_map.get(&feed.id).unwrap_or(&0);
        *group_unread.entry(g).or_default() += n;
        *group_unread.entry("all".to_string()).or_default() += n;
    }

    // Ensure "all" group exists at the front
    let mut dtos: Vec<GroupDto> = Vec::new();

    // Add "All" pseudo-group (not in DB, synthesized)
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
    // Fetch the existing group to preserve other fields, then upsert with new name
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

    // Run syncs concurrently
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
) -> Result<Vec<FeedItemDto>, String> {
    let core = state.core().await?;
    let views = core
        .search(&query, limit)
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

// ── AI management commands ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_ai_status(state: State<'_, AppState>) -> Result<AiStatusDto, String> {
    let core = state.core().await?;
    let onnx_loaded = core.onnx_loaded();
    let vision_loaded = core.vision_loaded();
    let fasttext_loaded = core.fasttext_loaded();
    let miniml_loaded = core.miniml_loaded();
    let model_name = core.active_model_name();
    let vision_model_name = core.active_vision_model_name();
    let fasttext_model_name = core.active_fasttext_model_name();
    let miniml_model_name = core.active_miniml_model_name();
    let tagging_mode = match (fasttext_loaded, miniml_loaded, onnx_loaded, vision_loaded) {
        (true, true, _, true) => "fasttext+miniml+vision",
        (true, true, _, false) => "fasttext+miniml",
        (true, false, _, true) => "fasttext+vision",
        (true, false, _, false) => "fasttext",
        (false, _, true, true) => "onnx+vision",
        (false, _, true, false) => "onnx",
        (false, _, false, true) => "vision",
        _ => "none",
    }
    .to_string();
    Ok(AiStatusDto {
        model_loaded: onnx_loaded,
        vision_loaded,
        fasttext_loaded,
        miniml_loaded,
        model_name,
        vision_model_name,
        fasttext_model_name,
        miniml_model_name,
        tagging_mode,
    })
}

#[tauri::command]
pub async fn list_models(state: State<'_, AppState>) -> Result<Vec<ModelInfoDto>, String> {
    let core = state.core().await?;
    let active_nli = core.active_model_name();
    let active_vision = core.active_vision_model_name();
    let active_fasttext = core.active_fasttext_model_name();
    let active_miniml = core.active_miniml_model_name();

    Ok(KNOWN_MODELS
        .iter()
        .map(|spec| {
            let model_dir = core.config.models_dir().join(spec.id);
            let downloaded = match spec.kind {
                "nli" => {
                    model_dir.join("model_quantized.onnx").exists()
                        || model_dir.join("model.onnx").exists()
                }
                "vision" => {
                    model_dir.join("vision_model_q4f16.onnx").exists()
                        || model_dir.join("vision_model_quantized.onnx").exists()
                        || model_dir.join("vision_model.onnx").exists()
                        || model_dir.join("model.onnx").exists()
                }
                "miniml" => model_dir.join("model.onnx").exists(),
                "fasttext" => model_dir.join("fasttext.pftm").exists(),
                _ => false,
            };
            let active = match spec.kind {
                "nli" => active_nli.as_deref() == Some(spec.id),
                "vision" => active_vision.as_deref() == Some(spec.id),
                "fasttext" => active_fasttext.as_deref() == Some(spec.id),
                "miniml" => active_miniml.as_deref() == Some(spec.id),
                _ => false,
            };
            ModelInfoDto {
                id: spec.id.to_string(),
                name: spec.name.to_string(),
                description: spec.description.to_string(),
                size_mb: spec.size_mb,
                downloaded,
                active,
                kind: spec.kind.to_string(),
            }
        })
        .collect())
}

#[tauri::command]
pub async fn download_model(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    model_id: String,
) -> Result<(), String> {
    let core = state.core().await?;
    let spec = KNOWN_MODELS
        .iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| format!("unknown model '{}'", model_id))?;

    let model_dir = core.config.models_dir().join(spec.id);
    std::fs::create_dir_all(&model_dir).map_err(|e| e.to_string())?;

    let client = reqwest::Client::builder()
        .user_agent("Pulse/1.0 model-downloader")
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())?;

    for (hf_path, local_name) in spec.files {
        let url = format!(
            "https://huggingface.co/{}/{}/resolve/main/{}",
            spec.hf_owner, spec.hf_repo, hf_path
        );
        let dest = model_dir.join(local_name);

        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("download error for {}: {}", local_name, e))?;

        if !resp.status().is_success() {
            return Err(format!("HTTP {} for {}", resp.status(), local_name));
        }

        let total = resp.content_length().unwrap_or(0);
        let mut bytes_done: u64 = 0;
        let mut buf: Vec<u8> = if total > 0 {
            Vec::with_capacity(total as usize)
        } else {
            Vec::new()
        };

        use futures_util::StreamExt;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("stream error for {}: {}", local_name, e))?;
            buf.extend_from_slice(&chunk);
            bytes_done += chunk.len() as u64;
            let _ = app.emit(
                "ai://download-progress",
                DownloadProgressEvent {
                    model_id: model_id.clone(),
                    file: local_name.to_string(),
                    bytes_done,
                    bytes_total: total,
                    done: false,
                },
            );
        }

        std::fs::write(&dest, &buf).map_err(|e| e.to_string())?;
    }

    // Emit done before activation so the UI unblocks even if activation fails
    let _ = app.emit(
        "ai://download-progress",
        DownloadProgressEvent {
            model_id: model_id.clone(),
            file: "done".into(),
            bytes_done: 0,
            bytes_total: 0,
            done: true,
        },
    );

    // Activate + hot-reload the model into memory (best-effort — files are on disk regardless)
    if spec.kind == "nli" {
        if let Err(e) = core.set_active_model(spec.id) {
            tracing::error!(model_id = %model_id, error = %e, "model activation failed after download");
        } else if let Err(e) = core.reload_onnx_tagger() {
            tracing::error!(model_id = %model_id, error = %e, "model hot-reload failed after activation");
        }
    } else if spec.kind == "vision" {
        // Delete stale label_embeddings.bin so descriptions are always fresh after download.
        let stale = model_dir.join("label_embeddings.bin");
        if stale.exists() {
            let _ = std::fs::remove_file(&stale);
        }
        if let Err(e) = core.set_active_vision_model(spec.id) {
            tracing::error!(model_id = %model_id, error = %e, "vision model activation failed after download");
        } else if let Err(e) = core.reload_vision_tagger() {
            tracing::error!(model_id = %model_id, error = %e, "vision model hot-reload failed after activation");
        }
    } else if spec.kind == "miniml" {
        // mlp_head.pmlp + miniml_thresholds.json are extracted by lib.rs on startup;
        // model.onnx + tokenizer.json were just downloaded above — now activate.
        if let Err(e) = core.set_active_miniml_model(spec.id) {
            tracing::error!(model_id = %model_id, error = %e, "miniml model activation failed after download");
        } else if let Err(e) = core.reload_miniml_tagger() {
            tracing::error!(model_id = %model_id, error = %e, "miniml model hot-reload failed after activation");
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_model(state: State<'_, AppState>, model_id: String) -> Result<(), String> {
    let core = state.core().await?;
    let spec = KNOWN_MODELS.iter().find(|m| m.id == model_id);
    match spec.map(|s| s.kind) {
        Some("vision") => core
            .remove_vision_model(&model_id)
            .map_err(|e| e.to_string()),
        Some("miniml") => core
            .remove_miniml_model(&model_id)
            .map_err(|e| e.to_string()),
        // NLI and unknown — fall back to the generic remove_model path
        _ => core.remove_model(&model_id).map_err(|e| e.to_string()),
    }
}

/// Switch to an already-downloaded model without re-downloading it.
#[tauri::command]
pub async fn activate_model(state: State<'_, AppState>, model_id: String) -> Result<(), String> {
    let core = state.core().await?;
    let spec = KNOWN_MODELS
        .iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| format!("unknown model '{}'", model_id))?;

    match spec.kind {
        "nli" => {
            core.set_active_model(&model_id)
                .map_err(|e| e.to_string())?;
            core.reload_onnx_tagger().map_err(|e| e.to_string())?;
        }
        "vision" => {
            core.set_active_vision_model(&model_id)
                .map_err(|e| e.to_string())?;
            core.reload_vision_tagger().map_err(|e| e.to_string())?;
        }
        "fasttext" => {
            core.set_active_fasttext_model(&model_id)
                .map_err(|e| e.to_string())?;
            core.reload_fasttext_tagger().map_err(|e| e.to_string())?;
        }
        "miniml" => {
            core.set_active_miniml_model(&model_id)
                .map_err(|e| e.to_string())?;
            core.reload_miniml_tagger().map_err(|e| e.to_string())?;
        }
        _ => return Err(format!("unknown model kind '{}'", spec.kind)),
    }
    Ok(())
}

#[tauri::command]
pub async fn retag_all(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<i64, String> {
    let settings = load_settings(&state.data_dir);

    // If the user has disabled AI tagging, skip entirely.
    if !settings.ai_tagging {
        return Ok(0);
    }

    let core = state.core().await?;
    let app2 = app.clone();
    let progress = move |tagged: usize, total: usize| {
        let _ = app2.emit(
            "ai://tagging-progress",
            TaggingProgressEvent {
                tagged,
                total,
                done: false,
            },
        );
    };
    let (items, tags) = core
        .run_tagger_direct(None, true, Some(&progress))
        .await
        .map_err(|e| e.to_string())?;
    let _ = app.emit(
        "ai://tagging-progress",
        TaggingProgressEvent {
            tagged: items,
            total: items,
            done: true,
        },
    );

    let threshold = settings.confidence_threshold as f32;
    if threshold > 0.15 {
        let _ = core.delete_tags_below_confidence(threshold).await;
    }

    Ok(tags as i64)
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

/// Return the log directory path so the frontend can display or open it.
#[tauri::command]
pub fn get_log_path(state: State<'_, AppState>) -> String {
    state.data_dir.join("logs").to_string_lossy().to_string()
}

/// Open the log directory in the system file manager (desktop only).
#[tauri::command]
pub fn open_logs_folder(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let log_dir = state.data_dir.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    app.opener()
        .open_path(log_dir.to_string_lossy().as_ref(), None::<&str>)
        .map_err(|e| e.to_string())
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
