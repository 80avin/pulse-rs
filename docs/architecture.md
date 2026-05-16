# Pulse — System Architecture

## Overview

Pulse is a local-first feed intelligence system. All data lives on-device. Network access is limited to feed fetching. AI inference runs locally. The system has no backend server, no user accounts, and no cloud sync.

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
│  │    pulse-cli (TUI)   │    │   src-tauri (Tauri commands) │   │
│  └──────────┬───────────┘    └──────────────┬───────────────┘   │
└─────────────│────────────────────────────────│───────────────────┘
              │                                │
              └───────────────┬────────────────┘
                              │ calls
┌─────────────────────────────▼───────────────────────────────────┐
│                      pulse-core LIBRARY                         │
│                                                                 │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────────┐    │
│  │  Timeline   │  │   Search     │  │    Config           │    │
│  │  (query +   │  │  (FTS5 +     │  │   (settings,        │    │
│  │  pagination)│  │   ranking)   │  │    user prefs)      │    │
│  └──────┬──────┘  └──────┬───────┘  └─────────────────────┘    │
│         │                │                                      │
│  ┌──────▼────────────────▼──────────────────┐                   │
│  │              Storage Layer               │                   │
│  │   (rusqlite + migrations + FTS5 sync)    │                   │
│  └──────────────────────┬───────────────────┘                   │
│                         │                                       │
│  ┌──────────────────────▼───────────────────────────────────┐   │
│  │                  AI Tagging Pipeline                     │   │
│  │  ┌─────────────┐           ┌──────────────────────────┐  │   │
│  │  │ Rule Engine │  (Phase1) │  ONNX Model Inference    │  │   │
│  │  │ (keyword/   │           │  (ort crate, Phase 4)    │  │   │
│  │  │  regex)     │           └──────────────────────────┘  │   │
│  │  └─────────────┘                                         │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                  Sync Engine                             │   │
│  │  ┌───────────┐  ┌────────────┐  ┌─────────────────────┐ │   │
│  │  │ Scheduler │  │  Backoff   │  │  Health Tracker     │ │   │
│  │  │ (Tokio    │  │  (per-feed)│  │  (success rate,     │ │   │
│  │  │  tasks)   │  │            │  │   latency)          │ │   │
│  │  └───────────┘  └────────────┘  └─────────────────────┘ │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                  Feed Sources                            │   │
│  │  ┌───────────┐  ┌─────────────┐  ┌────────────────────┐ │   │
│  │  │ RSS/Atom  │  │ Hacker News │  │  Reddit JSON API   │ │   │
│  │  │ (feed-rs) │  │ (HN Firebase│  │  (no auth, .json)  │ │   │
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

## Data Flow

### Feed Ingestion

```
1. Scheduler fires for feed F
2. HTTP GET with If-None-Match / If-Modified-Since headers
3a. 304 Not Modified → update last_checked_at only
3b. 200 OK → parse response bytes
4. Source adapter normalizes to Vec<FeedItem>
5. Deterministic UUIDv5 per item (namespace=feed_url, name=item_guid)
6. Upsert into feed_items (INSERT OR IGNORE for new, no update for existing)
7. Insert new item_states rows (is_read=0, is_saved=0, is_hidden=0)
8. Trigger AI tagging pipeline for new items
9. Update FTS5 index
10. Update feed health metrics
```

### Timeline Query

```
1. User requests timeline (group=None|Some, limit, cursor, filters)
2. Build SQL query with appropriate WHERE clauses
3. JOIN feed_items + item_states + feeds (+ feed_groups if filtering)
4. Apply read/saved/hidden filters
5. Cursor-based pagination via (published_at, id) tuple
6. Return FeedItemView structs (flattened for display)
```

### AI Tagging

```
1. New FeedItem arrives in tagging queue
2. Normalize text: strip HTML, collapse whitespace, truncate to 512 tokens
3. Phase 1: run rule engine → Vec<(tag, confidence, explanation)>
   Phase 4: run ONNX model → embedding → tag classification
4. Store tags in ai_tags table
5. Tags available immediately for filtering
```

## Core Design Principles

### 1. Pure Functions for Business Logic

Feed normalization, item deduplication, tag rule evaluation, and timeline filtering are all pure functions with no IO side effects. They take data, return data. This makes them trivially testable and easy to reason about.

```rust
fn normalize_rss_item(raw: feed_rs::model::Entry, feed_id: &FeedId) -> FeedItem { ... }
fn evaluate_rules(item: &FeedItem, rules: &[TagRule]) -> Vec<TagResult> { ... }
fn apply_timeline_filter(items: &[FeedItemView], filter: &Filter) -> Vec<FeedItemView> { ... }
```

### 2. IO at the Edges

Network fetches and SQLite reads/writes happen at the edges of the system. The feed adapter fetches bytes; the normalizer processes them; the storage layer persists the result. No module reaches across two layers.

### 3. Explicit Error Types

Each module defines its own error enum via `thiserror`. Errors carry enough context to be actionable without stack traces:

```rust
#[derive(Debug, thiserror::Error)]
pub enum FeedError {
    #[error("HTTP {status} fetching {url}: {message}")]
    Http { url: String, status: u16, message: String },
    #[error("Feed parse error for {url}: {source}")]
    Parse { url: String, source: feed_rs::parser::ParseFeedError },
    #[error("Feed not found: {id}")]
    NotFound { id: FeedId },
}
```

### 4. No Global Mutable State

`pulse-core` is instantiated as a `PulseCore` struct that holds:
- A `DbHandle` — a cloneable sender into the DB writer actor (see §5)
- A `SyncScheduler` handle
- A `TaggerHandle` for the AI pipeline

This struct is passed explicitly to every subsystem. No `lazy_static!`, no `thread_local!`, no global singletons.

### 5. DB Writer Actor — Single Owned Connection, No Mutex

SQLite is single-writer. Rather than wrapping the connection in `Arc<Mutex<>>` and calling `spawn_blocking` from every task (which would spawn up to N OS threads all blocked waiting for the mutex under concurrent load), we use a **DB writer actor**: a single Tokio task that owns the `rusqlite::Connection` directly and receives write requests through an `mpsc` channel.

```rust
enum DbCommand {
    UpsertItems { items: Vec<FeedItem>, reply: oneshot::Sender<Result<usize>> },
    UpdateItemState { item_id: String, state: ItemStatePatch, reply: oneshot::Sender<Result<()>> },
    // ... other write operations
}

// Single task, owns connection exclusively — no Mutex, no spawn_blocking for writes
async fn db_writer_task(mut rx: mpsc::Receiver<DbCommand>, conn: rusqlite::Connection) {
    while let Some(cmd) = rx.recv().await {
        match cmd {
            DbCommand::UpsertItems { items, reply } => {
                let result = conn.upsert_items(&items);  // synchronous, no block needed
                let _ = reply.send(result);
            }
            // ...
        }
    }
}
```

Read operations use a **separate read-only connection** opened in WAL mode (WAL allows multiple concurrent readers). Reads do not go through the actor — they call `spawn_blocking` with their own connection handle:

```rust
#[derive(Clone)]
pub struct DbHandle {
    writer: mpsc::Sender<DbCommand>,
    read_pool: Arc<ReadPool>,  // pool of read-only connections (WAL mode)
}
```

This eliminates the "N tasks × 1 mutex" OS thread exhaustion problem entirely. The actor serializes all writes without spawning extra threads; readers are truly concurrent.

## Crate Selection Rationale

| Crate | Purpose | Rationale |
|---|---|---|
| `feed-rs` | RSS/Atom parsing | Handles RSS 0.9/1.0/2.0, Atom, JSON Feed. Well-maintained. Zero unsafe parsing. |
| `rusqlite` + `bundled-full` | SQLite access | Bundles SQLite with FTS5/JSON1. No system SQLite dependency. Sync API matches SQLite's nature. |
| `rusqlite_migration` | Schema migrations | Lightweight, simple. Runs migrations in order on startup. |
| `reqwest` | HTTP client | Async, TLS, follows redirects, connection pooling. Compiles on Android via ring/rustls. |
| `tokio` | Async runtime | Standard choice. Multi-threaded runtime for background sync tasks. |
| `ort` | ONNX Runtime Rust | Official bindings. Supports mobile. Dynamic linking to system ORT or static build. |
| `ratatui` | Terminal UI | Actively maintained fork of tui-rs. Flexible layout system. |
| `clap` | CLI argument parsing | Derive macros, good error messages, shell completion generation. |
| `uuid` v1 + v5 | ID generation | UUIDv5 for deterministic item IDs, UUIDv4 for feed/group IDs. |
| `chrono` | Date/time | Full timezone support. SQLite stores as Unix timestamps (i64). |
| `tracing` | Structured logging | Compatible with Tokio. Spans for sync cycles and AI inference. |
| `thiserror` | Error types | Zero-cost derive macros for error enums. |
| `anyhow` | Error propagation | For CLI and binary contexts where granular error types aren't needed. |
| `serde` + `serde_json` | Serialization | Source metadata stored as JSON blobs in SQLite. |

### Deliberately Not Used

- **`sqlx`** — Async SQLite via sqlx has correctness issues (WAL mode + async can cause reader starvation). The async story for SQLite is fundamentally awkward. The DB writer actor pattern with `rusqlite` is more correct.
- **`diesel`** — ORM overhead, proc-macro compile time, harder to write explicit SQL for FTS5 queries.
- **`async-std`** — Tokio is the clear ecosystem standard. No benefit to using async-std.
- **`crossbeam`** — Tokio channels cover all inter-task communication needs.

## Async Model

The Tokio runtime runs in the CLI and Tauri process. Key async boundaries:

```
┌─ Tokio Runtime ─────────────────────────────────────┐
│                                                     │
│  ┌─ main task ─────────────────────────────────┐   │
│  │  CLI command dispatch or Tauri event loop   │   │
│  └─────────────────────────────────────────────┘   │
│                                                     │
│  ┌─ sync task (per active feed) ───────────────┐   │
│  │  sleep(interval) → fetch → normalize →      │   │
│  │  DbHandle.send(UpsertItems) → await reply   │   │
│  └─────────────────────────────────────────────┘   │
│                                                     │
│  ┌─ DB writer actor ───────────────────────────┐   │
│  │  Owns Connection (no Mutex, no Arc)         │   │
│  │  Drains mpsc DbCommand channel              │   │
│  └─────────────────────────────────────────────┘   │
│                                                     │
│  ┌─ read pool (spawn_blocking) ────────────────┐   │
│  │  Read-only WAL connections for queries      │   │
│  └─────────────────────────────────────────────┘   │
│                                                     │
│  ┌─ tagging task ──────────────────────────────┐   │
│  │  Bounded mpsc channel (cap=200)             │   │
│  │  Runs rule engine (sync, fast)              │   │
│  │  Phase 4: spawn_blocking(ort inference)     │   │
│  └─────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

The sync scheduler spawns one Tokio task per feed. Each task sleeps for its configured interval, fetches, normalizes, and persists. Tasks are lightweight (just sleeping most of the time) so having 50 tasks for 50 feeds is fine.

## Platform Portability

`pulse-core` must compile and run on:
- Linux x86_64 (development, CLI)
- macOS arm64 (development, desktop)
- Android aarch64 (production primary target)
- Windows x86_64 (desktop, secondary)

Key constraints:
- `reqwest` uses `rustls` (not native-tls) for TLS. `rustls` compiles cleanly on Android.
- `rusqlite` with `bundled` feature bundles its own SQLite so there's no system dependency.
- `ort` requires the ONNX Runtime native library. On Android, this ships as a `.so` in the APK.
- All file paths go through `pulse-core`'s config layer, which resolves platform-appropriate data directories.

## Known Architecture Risks

1. **SQLite write contention**: Single-writer model. If sync tasks and user actions try to write simultaneously, the `Mutex<Connection>` becomes a bottleneck. Mitigation: WAL mode reduces this significantly. If it remains an issue in benchmarks, consider a dedicated writer task with a command channel.

2. **ONNX Runtime size on Android**: The ORT native library is ~6-8MB. This significantly increases APK size. Mitigation: use quantized models (q8 or q4f16), consider dynamic linking to system ORT if available on the device.

3. **Reddit JSON API reliability**: Reddit's unofficial JSON endpoint is not guaranteed to remain available. Mitigation: treat Reddit as a best-effort source; design the feed health system to gracefully handle source instability. OAuth support in Phase 2 provides a fallback.

4. **Background sync on Android**: Tauri 2 on Android provides no first-class equivalent of Android `WorkManager` or `JobScheduler`. Tauri runs as an Activity; when backgrounded, the process can be killed within minutes. **Phase 1-2 sync is foreground-only** — sync runs only while the app (CLI or Tauri) is in the foreground. True background sync on Android requires a Kotlin plugin that registers a `WorkManager` periodic task; this is explicitly planned as a Phase 3 prerequisite and is a significant implementation effort. Mitigation for Phase 1-2: the sync model is fully resumable, so each feed-open triggers a fresh sync, and previously-fetched content is always available offline.
