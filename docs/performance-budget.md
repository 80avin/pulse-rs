# Pulse — Performance Budget

## Motivation

Performance budgets are commitments, not aspirations. They must be measurable and measured. Without concrete numbers, "fast" is meaningless. This document defines specific targets and the measurement approach for each.

Target hardware baseline:
- **Android (primary)**: Snapdragon 7-series (e.g., SM7325), 6GB RAM, UFS 3.1 storage
- **Desktop**: Mid-range laptop, 8GB RAM, NVMe SSD

---

## Startup Time

| Scenario | Budget | Notes |
|---|---|---|
| CLI cold start | < 50ms | DB connection + migration check + display first page |
| Desktop app cold start | < 2000ms | Full Tauri init + Svelte hydration + DB + first timeline page |
| Android cold start | < 3000ms | More expensive WebView initialization |
| Desktop warm start (second launch) | < 500ms | OS page cache warm |

**What counts as "started"**: The first timeline page is visible and interactive. Not just the splash screen.

**Measurement**: `time pulse timeline --limit 1` for CLI. Tauri's `performance.now()` from `document.DOMContentLoaded` to first timeline item rendered for UI.

**Primary drivers**:
- SQLite file open + WAL checkpoint: ~5-20ms
- Migration check (no-op): < 1ms
- First timeline query (50 items): < 10ms
- Svelte hydration: < 100ms (desktop), < 300ms (Android WebView)
- Tauri IPC round-trip: < 10ms

**Risk**: Android WebView startup is outside our control. If it exceeds budget, show a loading state immediately on launch rather than waiting for the WebView to be ready.

---

## Timeline Operations

| Operation | Budget | Measurement |
|---|---|---|
| Load timeline page (50 items) | < 100ms | From IPC call start to items rendered |
| Load next page (scroll) | < 50ms | Should feel instant |
| Apply filter change | < 30ms | Filter toggle to new list visible |
| Group switch | < 30ms | Tab tap to new group items visible |
| Mark item read | < 10ms | Tap to visual state change |
| Save item | < 10ms | |

**Timeline query includes**: JOIN on feeds, feed_groups, item_states, GROUP_CONCAT of ai_tags. Must stay under 10ms on SQLite to meet the 100ms budget (accounting for IPC overhead).

**Index requirements**: The `idx_feed_items_timeline` composite index on `(published_at DESC, id)` is critical for cursor pagination. Without it, each page load scans the full table.

**Virtualization requirement**: Rendering 50 items without virtualization takes ~16ms in a browser. With virtualization (only ~20-30 DOM nodes for visible area), rendering is ~3ms.

---

## Search

| Operation | Budget | Measurement |
|---|---|---|
| FTS5 query (< 100k items) | < 50ms | Query execution time |
| FTS5 query (< 500k items) | < 150ms | |
| Search result display | < 100ms total | Query + render |
| Incremental search (debounced) | 300ms debounce | Prevents query-per-keystroke |

**FTS5 performance**: BM25 ranking with FTS5 on 500k rows typically runs in 20-80ms. The `unicode61` tokenizer is slower than ASCII but required for non-ASCII content.

**Search index size**: FTS5 typically adds 50-70% of the original text size as index. For 500k items averaging 100 words each, expect ~150-200MB FTS index. This must be accounted for in the database size budget.

---

## Sync Engine

| Operation | Budget | Measurement |
|---|---|---|
| Sync cycle (20 feeds, all 200 OK) | < 10s wall time | Parallel HTTP, dominated by slowest feed |
| Sync cycle (20 feeds, all 304) | < 3s | All conditional hits |
| Single feed sync | < 5s | HTTP + parse + upsert |
| DB upsert (100 new items) | < 50ms | Batch within single transaction |
| FTS5 trigger overhead (100 items) | < 20ms | Automatic on insert |

**Parallelism**: All feed fetches run concurrently (Tokio tasks). The wall time is dominated by the slowest feed, not the total count. With 20 feeds, most will respond in < 1s; total wall time is bounded by network conditions, not CPU.

**Transaction batching**: All items from a single feed sync are inserted in one transaction. This is 10-50x faster than per-item transactions.

**404/connection timeout budget**: If a feed times out, it must not hold up other feeds. Per-feed HTTP timeout: 10s connect + 30s read. After timeout, increment failure_streak and move on.

---

## AI Inference

| Operation | Budget | Notes |
|---|---|---|
| Rule engine (1 item) | < 1ms | Synchronous, regex evaluation |
| Rule engine (100 items, batch) | < 50ms | |
| ONNX inference (1 item, MobileBERT q8, desktop) | < 50ms | |
| ONNX inference (1 item, MobileBERT q8, Snapdragon 7xx CPU-only) | < 400ms | Realistic mobile estimate; NNAPI unreliable for BERT ops |
| ONNX inference (1 item, MiniLM q8, Snapdragon 7xx CPU-only) | < 200ms | |
| Model load time | < 2s | Session initialization from file |
| Tokenization (512 tokens) | < 5ms | `tokenizers` crate (HuggingFace) |
| Label embedding comparison (15 tags) | < 1ms | Cosine similarity, SIMD |

**Note on mobile inference**: The 400ms budget for MobileBERT on Snapdragon 7xx is intentional — it reflects CPU-only reality. This is not user-visible because tagging is always asynchronous (items appear in the timeline immediately; tags arrive seconds later). Do not optimize tagging latency at the expense of correctness or reliability. Benchmark on actual SM7325 hardware before accepting any claim below 200ms without NNAPI.

**Tagging is asynchronous and non-blocking**. Items appear in the timeline immediately after sync; tags appear a few seconds later when the tagging task processes them. This is acceptable — the user sees content first, refinements follow.

**Phase 1 only uses the rule engine** (< 1ms per item). ONNX inference performance is a Phase 4 concern and does not block Phase 1 delivery.

**Batch tagging** (`pulse ai retag`): Throughput targets:
- Desktop (MobileBERT q8): > 10 items/second → 10,000 items in ~17 minutes
- Mobile (Snapdragon 7xx, CPU-only): > 3 items/second → 10,000 items in ~55 minutes

Batch inference (processing 4-8 items per ONNX session call) is a Phase 4 optimization that can improve throughput 2-4x. For Phase 1 (rule engine only), throughput is > 1000 items/second — not a concern.

**IPC serialization overhead** (Tauri → Svelte): 50 FeedItemView structs ≈ 50-100KB JSON. Rust serde_json serialization: ~2ms. JS JSON.parse: ~5ms. Total IPC overhead: ~7ms. This is already within the 100ms timeline budget but must be measured — do not assume JSON is fast for large payloads.

---

## Memory

| Scenario | Budget |
|---|---|
| Base (no model loaded) | < 50MB RSS |
| With MobileBERT q8 model | < 150MB RSS |
| With DistilBERT q8 model | < 200MB RSS |
| SQLite page cache | 32MB (configured) |
| Android system kill threshold | > 200MB RSS risks OOM kill |

**Android memory constraint**: Android may kill the app process if memory exceeds ~200MB on 4GB RAM devices. The 150MB budget with MobileBERT is deliberate — it leaves headroom.

**Model memory**: The ONNX session keeps the model weights in memory while active. MobileBERT q8 is ~25MB on disk but ~60-80MB in memory (due to session overhead). Unload the session when the app goes to background for > 5 minutes.

**Timeline items**: A `FeedItemView` struct (for display) is approximately 200 bytes. 1000 items in memory = ~200KB. Not a concern.

---

## Database Size

| Component | Expected size (100k items) |
|---|---|
| feed_items table | ~80MB |
| item_states table | ~8MB |
| ai_tags table | ~30MB |
| FTS5 index | ~60MB |
| feeds + groups + models | < 1MB |
| **Total** | **~180MB** |

At 100k items, the database is large but manageable. The UI should surface database size prominently in diagnostics.

**Archival policy**: Items older than 90 days that are not saved may be automatically purged (user-configurable). This keeps the database from growing unboundedly. Purged items: DELETE from feed_items CASCADE (cascades to item_states, ai_tags, FTS5 triggers).

---

## Battery (Android)

| Scenario | Target |
|---|---|
| Background sync (Wi-Fi, 20 feeds, 15-min interval) | < 2% battery per hour |
| Foreground usage (browsing, no sync) | < 5% battery per hour |
| AI batch tagging (background) | Pause if battery < 20% |
| Model download | Wi-Fi only by default |

**Background sync**: Each sync cycle does < 10s of active work every 15 minutes. Average duty cycle: 10s/900s ≈ 1.1%. HTTP with 304 responses: minimal radio time.

**Wake lock**: Do not hold wake locks during sync. Use `JobScheduler` (via Tauri's background task API) which handles scheduling appropriately. Never use `AlarmManager` with an exact repeating alarm — it bypasses Doze mode and kills battery.

**Doze mode**: During Doze mode, sync is deferred to maintenance windows. This is acceptable — Pulse is not a real-time notification system.

---

## Measurement Plan

### Rust Benchmarks (criterion)

Location: `crates/pulse-core/benches/`

```rust
// benches/db_benchmarks.rs
fn timeline_query(c: &mut Criterion) {
    let db = setup_db_with_100k_items();
    c.bench_function("timeline_page_50", |b| {
        b.iter(|| db.timeline().get_page(None, 50, None, TimelineFilter::default()))
    });
}

fn fts_search(c: &mut Criterion) {
    let db = setup_db_with_100k_items();
    c.bench_function("fts_search_simple", |b| {
        b.iter(|| db.search().query("rust async", 20))
    });
}

fn batch_upsert_100_items(c: &mut Criterion) {
    let db = setup_db();
    c.bench_function("upsert_100", |b| {
        b.iter(|| {
            let items = generate_fake_items(100);
            db.storage().upsert_items(&items)
        })
    });
}
```

Run with: `cargo bench -p pulse-core`

### CLI Timing

```bash
# Startup time
hyperfine --warmup 3 'pulse timeline --limit 1'

# Timeline page
hyperfine --warmup 3 'pulse timeline --limit 50'

# Search
hyperfine --warmup 3 'pulse search "rust async"'
```

### Tauri Startup Timing

```typescript
// In +layout.svelte
const t0 = performance.now();
// After first timeline page renders:
console.log(`[perf] startup: ${performance.now() - t0}ms`);
```

### Tracing Spans

`pulse-core` uses the `tracing` crate with span instrumentation on all significant operations:

```rust
#[tracing::instrument(skip(conn))]
async fn upsert_items(conn: &mut Connection, items: &[FeedItem]) -> Result<usize> { ... }

#[tracing::instrument]
async fn sync_feed(feed_id: &FeedId) -> Result<SyncResult> { ... }
```

In development mode, use `tracing-subscriber` with `RUST_LOG=pulse_core=debug`. In production, use structured JSON logging to a file that the diagnostics command can read.

---

## Performance Regressions

Any PR that degrades a measured benchmark by > 10% requires explicit justification. The benchmark suite runs in CI (`cargo bench --no-run` for compilation check; full bench on main merge only due to cost).

The `pulse diag` command reports real-world performance metrics (average query times, sync durations) so users can detect regressions in their specific environment.
