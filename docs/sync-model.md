# Pulse — Sync Model

## Overview

The sync engine is responsible for polling feed sources, normalizing results, persisting new items, and maintaining feed health metadata. It runs as a set of Tokio background tasks — one per enabled feed — coordinated by a `SyncScheduler`.

The sync model is designed around three principles:
1. **Resumability** — every feed tracks its own state; partial syncs don't corrupt anything
2. **Efficiency** — HTTP caching headers avoid re-downloading unchanged feeds
3. **Graceful degradation** — failing feeds back off and are clearly marked as unhealthy

## Scheduler Design

```
SyncScheduler
├── Feed A task (sleeps until next_fetch_at, then wakes and fetches)
├── Feed B task (...)
├── Feed C task (...)
└── Command channel (mpsc::Sender<SyncCommand>)
    ├── SyncCommand::RefreshFeed(FeedId)    → wake a specific task immediately
    ├── SyncCommand::AddFeed(FeedId)        → spawn a new task
    ├── SyncCommand::RemoveFeed(FeedId)     → cancel and remove the task
    └── SyncCommand::PauseAll / ResumeAll
```

Each feed task is a Tokio task:

```rust
async fn feed_sync_task(feed_id: FeedId, core: Arc<PulseCore>, mut cmd_rx: broadcast::Receiver<SyncCommand>) {
    loop {
        let feed = core.db().get_feed(&feed_id).await?;
        let delay = compute_delay(&feed);

        tokio::select! {
            _ = tokio::time::sleep(delay) => {
                perform_sync(&feed_id, &core).await;
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    SyncCommand::RefreshFeed(id) if id == feed_id => {
                        perform_sync(&feed_id, &core).await;
                    }
                    SyncCommand::RemoveFeed(id) if id == feed_id => break,
                    _ => {}
                }
            }
        }
    }
}
```

## Polling Intervals

Default intervals by feed type:

| Feed Type | Default Interval | Reasoning |
|---|---|---|
| RSS/Atom | 60 minutes | Most RSS feeds don't update more than hourly |
| HN Top Stories | 15 minutes | HN ranking changes frequently |
| HN New Stories | 10 minutes | New posts appear continuously |
| Reddit hot/new | 20 minutes | Reddit frontpage turns over moderately |
| Reddit top (day) | 60 minutes | Top posts are stable |

Users can override per-feed. Minimum allowed interval: 5 minutes (to prevent accidental abuse of source servers).

## HTTP Efficiency

For RSS/Atom and Reddit JSON, we use conditional HTTP requests to avoid re-downloading unchanged content.

### Request Flow

```
1. Load feed.etag and feed.last_modified from DB
2. Build GET request:
   - If etag: add header "If-None-Match: {etag}"
   - If last_modified: add header "If-Modified-Since: {last_modified}"
3. Send request
4. Handle response:
   - 304 Not Modified → update last_fetched_at, reset failure_streak; done
   - 200 OK → extract ETag and Last-Modified from response headers,
               parse body, normalize items, upsert to DB,
               update feed.etag / feed.last_modified
   - 4xx/5xx → see error handling below
```

This is particularly effective for RSS feeds, where 304s are common when nothing has changed.

HN's Firebase API does not support ETags. We store the highest item ID seen per feed section and only fetch item details for IDs we haven't seen.

### User-Agent

All requests use a descriptive User-Agent:
```
Pulse/0.1 (+https://github.com/avinthakur080/pulse-rs; feed-reader)
```

This is courteous to feed providers and helps them understand traffic sources.

## Backoff Strategy

Per-feed exponential backoff on HTTP errors and parse failures.

```
base_interval = feed.poll_interval_secs
failure_streak = feed.failure_streak

next_interval = min(
    base_interval * 2^failure_streak,
    MAX_BACKOFF_SECS   -- 4 hours (14400s)
) * jitter(0.9, 1.1)  -- ±10% random jitter
```

| failure_streak | With 60min base | Capped at |
|---|---|---|
| 0 | 60 min (normal) | — |
| 1 | 120 min | — |
| 2 | 240 min | — |
| 3 | 480 min → capped | 240 min |
| 4+ | capped | 240 min |

Jitter prevents "thundering herd" when multiple feeds all fail at the same time and would otherwise retry simultaneously.

### Success Reset

On a successful fetch (200 or 304), `failure_streak` is reset to 0, and the interval returns to the normal base.

### Permanent Failure

After `failure_streak >= 10` (configurable), the feed is marked `is_enabled = 0` and the sync task exits. The user sees a "disabled" indicator and must manually re-enable after fixing the feed URL.

## Feed Health Tracking

Each feed row maintains rolling health metrics:

```sql
failure_streak      -- current consecutive failures
total_fetches       -- lifetime fetch count
total_failures      -- lifetime failure count
avg_latency_ms      -- exponential moving average (α = 0.2)
last_success_at     -- timestamp of last 200 OK
last_item_at        -- timestamp of the newest item we've seen
```

**Success rate** is computed on-the-fly in queries:
```sql
ROUND((total_fetches - total_failures) * 100.0 / total_fetches, 1) AS success_rate
```

**Staleness**: A feed is "stale" if `last_item_at` is more than `7 * poll_interval_secs` in the past with no errors. This can mean the feed is dead (no new posts) or has changed URL. The CLI and UI surface this differently from an error state.

**Latency EMA update**:
```
new_avg = 0.2 * fetch_latency_ms + 0.8 * old_avg
```

## Deduplication

Item deduplication uses deterministic UUIDs (UUIDv5). Since the item ID is fully determined by `(feed_url, source_guid)`:

```sql
INSERT OR IGNORE INTO feed_items (id, feed_id, source_guid, title, ...)
VALUES (?, ?, ?, ?, ...);
```

`INSERT OR IGNORE` means: if the row already exists (by primary key or UNIQUE constraint), skip silently. No SELECT-then-INSERT dance needed. This is safe and idempotent.

A second `UNIQUE(feed_id, source_guid)` constraint provides defense-in-depth against hash collisions (theoretically possible but practically negligible with UUIDv5).

### Edge Case: Mutated Items

Some RSS feeds update their items after publication (correcting articles, updating scores). Since `feed_items` rows are immutable after insert, we don't update them when the source changes. Rationale: the immutable append-only model keeps query logic simple and avoids surprising users who marked an item as read from seeing it re-appear. If content freshness becomes important, a `feed_item_revisions` table can be added without changing the core model.

## Item State Initialization

When a new `feed_item` row is inserted, a corresponding `item_states` row is created in the **same explicit DB transaction** as the `feed_item` insert. This is non-negotiable: if the process is killed between the two inserts without a wrapping transaction, the `item_states` row will be missing and the item will silently disappear from all timeline queries.

```sql
-- Application code must wrap both in a single transaction:
BEGIN;

INSERT OR IGNORE INTO feed_items (id, feed_id, source_guid, title, ...)
VALUES (?, ?, ?, ?, ...);

-- Only create state row if the item was actually inserted (changes() = 1)
INSERT OR IGNORE INTO item_states (item_id, is_read, is_saved, is_hidden, updated_at)
SELECT id, 0, 0, 0, unixepoch()
FROM feed_items
WHERE id = ? AND changes() > 0;

COMMIT;
```

Additionally, the FTS5 insert (see data-model.md) must be triggered from application code after confirming `changes() > 0`, within the same transaction. Three operations, one transaction: feed_items insert, item_states insert, FTS insert.

Using `INSERT OR IGNORE` on item_states means re-syncing a feed never resets user state.

## Offline Model

Pulse is offline-first: previously fetched items are always accessible regardless of network connectivity.

Sync tasks fail gracefully:
- Network errors: increment failure_streak, schedule backoff, log error
- No crash, no user-visible error unless the user explicitly requests a sync and it fails

The UI/CLI distinguishes:
- **Fresh**: last_success_at < 2 * poll_interval_secs ago
- **Stale**: last_success_at between 2x and 24x poll_interval ago
- **Dead**: failure_streak >= 5 or is_enabled = 0

Staleness indicators are informational, not blocking. Users can always read previously fetched content.

## Android-Specific Constraints

Android aggressively kills background processes and restricts background network access.

### Sync Scope: Foreground-Only (Phase 1-2)

Tauri 2 on Android provides **no first-class equivalent of Android `WorkManager` or `JobScheduler`**. Tauri runs as a standard Activity. When the user backgrounds the app, Android can kill the process within minutes under memory pressure. There is no mechanism in Tauri's current API surface to register a periodic background job that survives app backgrounding.

**Phase 1-2 sync is foreground-only**: sync runs only while the CLI or Tauri app is in the foreground. This is the honest baseline. Key mitigations:
- The sync model is fully resumable — each feed's state is tracked independently
- Offline content (everything fetched while open) is always available
- The UI shows staleness indicators so users know when content is old
- Each app open triggers a sync cycle

**Phase 3 prerequisite**: True background sync on Android requires a Kotlin plugin that registers a `WorkManager` periodic task. This is significant implementation work (JNI bridge, Kotlin plugin API, Gradle manifest changes) and must be planned as a distinct Phase 3 deliverable, not assumed to come for free from Tauri.

### Battery Efficiency

Sync task design minimizes battery impact:
1. Most syncs result in 304 Not Modified (minimal data transfer)
2. Tasks sleep between polls (no busy-waiting)
3. Backoff prevents hammering dead feeds
4. When the app is in the foreground, sync frequency can be temporarily increased
5. When the device is on battery saver, sync frequency should be halved (detected via system API)

### Network Type Awareness

On Android, we should check connectivity before attempting syncs. Tauri provides network information APIs. The sync scheduler should:
- Skip sync if no network (cache offline indicator)
- Optionally skip sync on metered connection (user preference)
- Prefer Wi-Fi for model downloads (AI pipeline)

## Sync Pipeline Summary

```
SyncScheduler
    │
    ▼
feed_sync_task (per feed)
    │
    ├─► [HTTP] Fetch with conditional headers
    │       │
    │       ├─ 304 → update timestamps → done
    │       │
    │       └─ 200 → parse response bytes
    │                   │
    │                   ▼
    │           Source adapter normalizes
    │           to Vec<FeedItem>
    │                   │
    │                   ▼
    │           DB: INSERT OR IGNORE into feed_items
    │           DB: INSERT OR IGNORE into item_states
    │           DB: Update FTS5 triggers
    │                   │
    │                   ▼
    │           Send new item IDs → tagging queue (mpsc)
    │                   │
    │                   ▼
    │           Update feed health metrics
    │
    ▼
TaggingTask (singleton)
    │
    ├─ Drain tagging queue
    ├─ Run rule engine (Phase 1)
    └─ DB: INSERT into ai_tags
```

## Concurrency Model

All DB writes go through a single `Arc<Mutex<Connection>>`. This is intentionally simple.

Why not a connection pool?
- SQLite with WAL allows multiple concurrent readers but only one writer
- A pool of connections would still serialize on writes
- The overhead of pool management outweighs any benefit for our workload
- Single connection eliminates a class of "writer starved by readers" bugs

Performance concern: if sync tasks + user actions contend on the mutex, the user experiences lag. Mitigation:
1. All DB calls use `spawn_blocking`, so they don't block the Tokio executor
2. Individual DB operations are fast (microseconds); the lock isn't held across HTTP fetches
3. If benchmarks show contention, a separate `WriteQueue` actor pattern can be introduced

## Sync Error Taxonomy

| Error | Action | User-Visible |
|---|---|---|
| DNS resolution failure | Backoff, mark network error | "No network" indicator |
| TLS certificate error | Backoff, log URL | Warning in feed health |
| 404 Not Found | Increment streak, warn user | "Feed may have moved" |
| 410 Gone | Disable feed, notify user | "Feed explicitly gone (410)" |
| 429 Too Many Requests | Respect Retry-After header | "Rate limited, backing off" |
| 5xx Server Error | Backoff | Failure streak indicator |
| Parse error | Log item IDs, continue with parseable items | Partial sync indicator |
| Schema change | Log fields that couldn't be mapped | Warning in diagnostics |
