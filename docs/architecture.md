# Pulse — System Architecture

## Overview

Pulse is a local-first feed reader with on-device, deterministic tagging. All data lives on-device. Network access is limited to feed fetching and enrichment (OpenGraph metadata). There is **no ML stack** — tagging comes from a rule engine only. The system has no backend server, no user accounts, no cloud sync, and no telemetry.

The architecture prioritizes:
- **Correctness over cleverness** — deterministic data flows, explicit error paths
- **Testability** — core logic is pure functions or mockable IO boundaries
- **UI-agnosticism** — `pulse-core` knows nothing about how results will be displayed
- **Layered dependencies** — lower layers never depend on higher layers

## System Layers

```
┌─────────────────────────────────────────────────────────────────┐
│                        INTERFACE LAYER                          │
│  ┌──────────────────────┐    ┌──────────────────────────────┐   │
│  │  pulse-cli (clap)    │    │  src-tauri (Tauri commands)  │   │
│  └──────────┬───────────┘    └──────────────┬───────────────┘   │
└─────────────│────────────────────────────────│───────────────────┘
              │                                │
              └───────────────┬────────────────┘
                              │ calls PulseCore
┌─────────────────────────────▼───────────────────────────────────┐
│                      pulse-core LIBRARY                         │
│                                                                 │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────────┐    │
│  │  Timeline   │  │   Search     │  │  Config             │    │
│  │  Service    │  │  Service     │  │  (data dir, sync,   │    │
│  │  (cursor    │  │  (FTS5,      │  │   reddit auth)      │    │
│  │  paging)    │  │  rank sort)  │  └─────────────────────┘    │
│  └──────┬──────┘  └──────┬───────┘                             │
│         │                │                                      │
│  ┌──────▼────────────────▼──────────────────┐                   │
│  │              Storage Layer               │                   │
│  │  (sqlx + SQLite, migrations, FTS5 sync,  │                   │
│  │   writer actor + read pool)              │                   │
│  └──────────────────────┬───────────────────┘                   │
│                         │                                       │
│  ┌──────────────────────▼───────────────────────────────────┐   │
│  │                 Tagging Pipeline                         │   │
│  │  Tagger trait → RulesTagger → RuleEngine (rules only)    │   │
│  │  Bounded queue (200) → tagger_task → insert_ai_tags      │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                  Sync Engine                             │   │
│  │  SyncScheduler → per-feed tokio task → perform_sync      │   │
│  │  ETag/Last-Modified caching · exponential backoff        │   │
│  │  (60s → 4h cap, ±10% jitter) · health tracking           │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                  Feed Sources                            │   │
│  │  ┌───────────┐  ┌─────────────┐  ┌────────────────────┐ │   │
│  │  │ RSS/Atom  │  │ Hacker News │  │  Reddit JSON API   │ │   │
│  │  │ (feed-rs) │  │ (Firebase   │  │  (OAuth2 optional) │ │   │
│  │  │           │  │  API)       │  │                    │ │   │
│  │  └───────────┘  └─────────────┘  └────────────────────┘ │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
              │
              ▼
┌─────────────────────┐
│   SQLite Database   │
│   (single file,     │
│   WAL mode)         │
└─────────────────────┘
```

## Storage: sqlx + a single-writer actor

The persistence layer is **`sqlx`** (SQLite), not rusqlite. Two pools are opened against the same database file:

- **Writer pool** (`open_writer_pool`) — `max_connections(1)`. Sets WAL, `synchronous = FULL` on Android / `NORMAL` on desktop, `busy_timeout = 5s`, `foreign_keys = ON`, and platform-aware `mmap_size`.
- **Reader pool** (`open_reader_pool`) — `max_connections(3)`, read-only. WAL is inherited from the writer connection; readers never set it.

SQLite is single-writer, so all mutations funnel through a **DB writer actor**: a single Tokio task that owns the writer pool and receives typed `DbCommand` messages over an `mpsc` channel. Each command carries a `oneshot` reply channel, so callers await the result.

```rust
// crates/pulse-core/src/storage/actor.rs
pub enum DbCommand {
    UpsertItems { items, reply },        // INSERT OR IGNORE + item_states + FTS
    UpdateItemState { item_id, patch, reply },
    UpsertFeed { feed, reply },
    InsertFeedGroup { group, reply },
    UpdateFeedHealth { update, reply },
    InsertAiTags { item_id, tags, reply },
    ReplaceUserTags { item_id, tags, reply },
    UpdateFeedSourceConfig { feed_id, source_config, reply },
    DeleteFeed { feed_id, reply },
    ClearFeedCache { feed_id, reply },
    EnrichItem { item_id, body_text, source_meta_patch, reply },
    DeleteFeedGroup { id, reply },
    DeleteItemTags { item_id, reply },
    MarkFeedRead { feed_id, reply },
    Vacuum { reply },                    // rebuilds FTS after (rowids renumber)
    FixRelativeUrls { reply },           // M0006 backfill maintenance
}
```

Reads never go through the actor. `DbHandle::with_reader(closure)` hands a clone of the reader pool to the closure, so concurrent readers are truly concurrent while all writes stay serialized on one connection. This eliminates the "N tasks × 1 mutex" thread-exhaustion problem without spawning extra OS threads.

## Crate selection

| Crate | Purpose | Rationale |
|---|---|---|
| `feed-rs` | RSS/Atom parsing | RSS 0.9/1.0/2.0, Atom, JSON Feed. Zero unsafe parsing. |
| `sqlx` | SQLite access | Async SQLite with connection pools, `bundled` SQLite, WAL. The two-pool pattern keeps writes serialized while allowing concurrent reads. |
| `reqwest` | HTTP client | Async, TLS via `rustls` (compiles on Android). One shared client on the scheduler. |
| `tokio` | Async runtime | Standard choice; drives sync tasks, the writer actor, the tagger task. |
| `clap` | CLI argument parsing | Derive macros, global `--data-dir` / `--db` / `--json`. |
| `uuid` | ID generation | UUIDv5 for deterministic item IDs, UUIDv4 for feed/group IDs. |
| `chrono` | Date/time | SQLite stores Unix timestamps (i64). |
| `tracing` | Structured logging | Logs go to rolling `pulse.log.*` files in the data dir on Tauri. |
| `thiserror` / `anyhow` | Errors | `thiserror` for library error enums, `anyhow` for the CLI. |
| `serde` / `serde_json` | Serialization | `source_config` stored as JSON blobs; DTOs over IPC. |

**Not used:** `rusqlite`, `diesel`, `ort` (ONNX), `tokenizers`, `ratatui` — the ML stack and the interactive TUI were removed in v0.6.

## Async model

The Tokio runtime runs in the CLI and the Tauri process. Key async boundaries:

```
┌─ Tokio Runtime ─────────────────────────────────────────────┐
│                                                             │
│  ┌─ main task ──────────────────────────────────────────┐   │
│  │  CLI command dispatch or Tauri event loop            │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─ sync task (per active feed) ────────────────────────┐   │
│  │  sleep(next_fetch_at) → perform_sync → upsert →      │   │
│  │  tagger.tag_item(...) for each new item              │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─ DB writer actor ────────────────────────────────────┐   │
│  │  Owns single-connection writer pool                  │   │
│  │  Drains mpsc DbCommand channel, replies via oneshot  │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─ reader pool (up to 3 concurrent) ───────────────────┐   │
│  │  Read-only WAL connections for queries               │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─ tagger task ────────────────────────────────────────┐   │
│  │  Bounded mpsc channel (cap 200)                      │   │
│  │  Runs RuleEngine synchronously (fast, no blocking)   │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Data flow

### Feed ingestion

```
1. Sync task fires for feed F (at next_fetch_at)
2. HTTP GET with If-None-Match / If-Modified-Since
3a. 304 Not Modified → update last_checked_at only
3b. 200 OK → parse and normalize to Vec<FeedItem>
4. Deterministic UUIDv5 per item (namespace = feed url, name = source_guid)
5. body_html is passed through resolve_relative_urls at ingestion:
   relative src/href → absolute against the item URL; http media → https
   (a CSP-safe pass — img-src only allows https:)
6. DbCommand::UpsertItems (INSERT OR IGNORE; FTS + item_states inserted
   only when the row was actually new)
7. Each new item is queued via tagger.tag_item(id, feed_type)
8. update_feed_health: EMA latency, failure_streak, ETag/Last-Modified;
   after a failure compute_next_fetch applies exponential backoff
   (base × 2^streak, cap 4h, ±10% jitter); feeds disable after 10
   consecutive failures
```

### Timeline query

```
1. User requests a page (group/feed/read/saved/tag filter, cursor, limit)
2. get_timeline builds the WHERE clause; hidden items always excluded
3. JOIN feed_items + feeds + feed_groups + item_states + ai_tags + user_tags
4. Cursor-based pagination on (published_at, id) — or (saved_at, id) when
   the saved view is active
5. Returns FeedItemView structs (flattened) + nextCursor + counts
```

### Tagging

```
1. New item queued (bounded channel, cap 200; drop + warn when full)
2. tagger_task → process_tag_request:
   a. load item
   b. skip direct image URLs (no text to match)
   c. RulesTagger.tag(item, feed_type) → Vec<TagResult>
   d. drop no-context when a substantive tag fires; strip semantic tags
      when noise ≥ 0.70
   e. db.insert_ai_tags (ON CONFLICT(item_id, tag, tagger_source) DO UPDATE)
3. Tags available immediately for the tag filter / tag chips
```

See [tagging.md](tagging.md) for the full tagging pipeline.

### Enrichment (OpenGraph)

Link posts are enriched after sync in a bounded concurrent pass (`enrich_pending`, default concurrency 5): fetch OG title/description/image, mark `enriched_at` in `source_meta`. Direct image URLs and non-HTTP hosts are skipped. This is what populates `og_image` thumbnails and the reader's description text.

## Frontend shell

The SvelteKit UI is a single responsive shell (`src/lib/screens/AppShell.svelte`). `useIsDesktop` (`matchMedia('(min-width: 768px)')`) selects the layout:

- **Wide (desktop)** — an app toolbar, a left rail with an **Overview / Feeds** toggle, group tabs, a sources list, a resizable timeline column, and a reader pane. Keyboard navigation (`j/k/m/s/o/x`, `/` focus search, `?` cheatsheet) is handled in the shell.
- **Narrow (Android)** — a bottom-nav shell with Feed / Sources / Search / Saved / Settings tabs, history-backed navigation so system back unwinds in-app, swipe gestures in the reader, and the share-intent listener.

**Home / Overview** (`src/lib/components/Home.svelte`) is the search tab on mobile and the overview screen on desktop: a global FTS search box plus a grid of per-group cards (recent items + total/unread counts + drill-in via `get_overview`).

**Saved view** (`SavedList.svelte`) pages saved items and groups them by the calendar month in which they were **saved** (from `item_states.saved_at`), newest first.

**Overlays and context menus** — the UI uses a small overlay registry (`src/lib/stores/overlays.svelte.ts`) that blocks shell-level keyboard shortcuts while any overlay is open. `ContextMenu.svelte` renders a right-click popup or a bottom sheet and registers itself in the registry. Source rows open a context menu with view / refresh / mark-read / edit / remove actions.

**Reader item resolution** (`ReaderPane.svelte`) — the opener hands the reader the exact list the item was clicked from. Resolution walks: opener's list → the paginated `items` store → `knownItems` cache → a fetch-by-ID fallback (`get_item`). Feed-body links open externally (never in the webview) with a hover/long-press URL preview via a `use:bodyLinks` action.

**First-run onboarding / discover** (`Onboarding.svelte`) shows when a cold start finishes with no sources. It offers the 68-feed curated catalog from `pulse-core/src/onboarding.rs`, grouped by category (Mailing Lists flagged experimental), with per-feed **preview** (`preview_feed` Tauri command → transient items, never persisted). The Android share flow uses `detect_feed_url` (`crates/pulse-core/src/feeds/detect.rs`) to turn a shared URL into an addable feed.

## Tauri IPC layer

`src-tauri/src/commands.rs` holds all `#[tauri::command]` functions. They receive `State<AppState>` (which wraps `Arc<PulseCore>` plus the data dir and a live log filter) and return serializable DTOs from `src-tauri/src/models.rs` (camelCase). Registered commands cover: sources (`get_sources`/`add_source`/`update_source`/`delete_source`), items (`get_items_page`/`get_item`/`mark_items_read`/`mark_source_read`/`toggle_saved`/`set_item_note`/`hide_item`), tags (`get_user_tags`/`set_user_tags`/`get_tag_stats`), groups (`get_groups`/`add_group`/`rename_group`/`delete_group`), overview (`get_overview`), onboarding (`get_popular_feeds`/`add_onboard_feeds`/`preview_feed`), sync (`sync_source`/`sync_all`), search (`search_items`), stats (`get_db_stats`), settings (`get_settings`/`save_settings`), share/detect (`detect_feed`/`get_pending_share`), and logging (`log_from_frontend`/`set_log_level`/`get_log_content`/`get_log_path`/`open_logs_folder`/`share_log_file`). No business logic lives in the shell.

The Android share bridge (`ShareBridge.kt` → JNI → `lib.rs`) buffers incoming URLs in `PENDING_SHARE` and emits `share://incoming-url`; `+layout.svelte` listens and opens the ShareSheet.

## Platform portability

`pulse-core` compiles and runs on:
- Linux x86_64 (development, CLI)
- macOS arm64 (development, desktop)
- Android aarch64 (production primary target)
- Windows x86_64 (desktop, secondary)

Key constraints:
- `reqwest` uses `rustls` (+ `rustls-platform-verifier` on Android for the system trust store), not native-tls.
- `sqlx` is used with the `bundled` SQLite feature — no system SQLite dependency.
- `PulseConfig::is_android` gates pragma differences (FULL vs NORMAL sync, mmap disabled) and the Android data dir.
- All file paths go through `PulseConfig::platform_data_dir()`:
  - Linux/macOS: `$XDG_DATA_HOME/pulse` (fallback `~/.local/share/pulse`)
  - Windows: `%APPDATA%\pulse`
  - Android: app-private data dir (keyed to package ID, survives APK updates)

## Known architecture risks

1. **SQLite single-writer**: all writes serialize through one connection. Under a large sync this is the hot path, but batch upserts run in a single transaction and reads are unaffected (WAL).
2. **Reddit JSON API reliability**: the public `.json` endpoint is best-effort; an OAuth2 script-app flow is supported (`--reddit-client-id`/`--reddit-client-secret`), and the feed health system disables persistently failing feeds.
3. **FTS ↔ VACUUM**: `VACUUM` can renumber implicit rowids and corrupt the external-content FTS mapping — `DbCommand::Vacuum` always rebuilds the FTS index afterward.
