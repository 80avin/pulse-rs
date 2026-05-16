# Pulse — Phase 0 Critique Summary

Three subagent reviewers evaluated the Phase 0 architecture documents. This document records all findings and the disposition (fixed, deferred, accepted) for each.

---

## Systems Architecture Review

### BLOCKER-SYS-1: FTS5 INSERT trigger fires on ignored rows → phantom index entries
**Document**: data-model.md  
**Finding**: The INSERT trigger on `feed_items` fires even when `INSERT OR IGNORE` is a no-op (duplicate). `new.rowid` is populated even for ignored rows, producing FTS5 entries pointing to wrong rowids.  
**Fix applied**: Restructured FTS5 insert to happen in application code (not via trigger) after confirming `changes() = 1`. INSERT trigger removed. See revised data-model.md.

### BLOCKER-SYS-2: `Arc<Mutex<Connection>>` + `spawn_blocking` → OS thread exhaustion under load
**Document**: architecture.md  
**Finding**: With 50 concurrent feed sync tasks, each calling `spawn_blocking` which then blocks on `Mutex::lock()`, Tokio spawns up to 50 OS threads simultaneously waiting for one mutex. On Android with constrained threading this risks OOM on the blocking pool.  
**Fix applied**: Replaced with a DB writer actor: a single Tokio task that owns `Connection` directly (no Arc<Mutex>), receiving write requests via `mpsc::Sender`. Read operations use a separate read-only WAL connection. See revised architecture.md.

### BLOCKER-SYS-3: HN Firebase API — 500 individual HTTP fetches on cold sync
**Document**: data-model.md, performance-budget.md  
**Finding**: `topstories.json` returns 500 IDs. Fetching each individually at 200ms = minimum 25s cold sync. "Single feed sync < 5s" budget is violated by design.  
**Fix applied**: HN adapter fetches only top N items on first sync (default 30, configurable). On subsequent syncs, tracks highest-seen item ID to skip already-fetched items. See revised sync-model.md.

### CONCERN-SYS-1: `synchronous = NORMAL` is unsafe on Android
**Document**: data-model.md  
**Finding**: Android OOM kills mid-write are equivalent to power failure. `NORMAL` + WAL can produce an incomplete WAL file on process death, risking DB corruption.  
**Fix applied**: Added Android-specific note to use `synchronous = FULL` on Android builds. See revised data-model.md.

### CONCERN-SYS-2: FTS5 UPDATE trigger absent — body_text updates silently break index
**Document**: data-model.md  
**Finding**: Acknowledged but deferred. Future full-article-content fetching will require this.  
**Fix applied**: Added stub UPDATE trigger with an assertion that fails loudly if it fires unexpectedly, plus explicit note that adding the trigger later requires `fts5 rebuild`.

### CONCERN-SYS-3: `item_states` initialization not in same transaction as `feed_items`
**Document**: sync-model.md  
**Finding**: If process is killed between feed_items insert and item_states insert, item is orphaned and silently drops from timeline queries.  
**Fix applied**: Batch upsert wraps both in a single `BEGIN; ... COMMIT;`. See revised sync-model.md.

### CONCERN-SYS-4: Tauri Android has no WorkManager/JobScheduler equivalent
**Document**: sync-model.md  
**Finding**: Tauri 2 on Android provides no persistent background service API. Claims about "Tauri background task API" subject to JobScheduler are optimistic — Tauri runs as an Activity; when backgrounded, the process may be killed within minutes.  
**Fix applied**: Revised Android section to clearly state Phase 1-2 sync is foreground-only. Phase 3 Kotlin plugin / WorkManager integration is listed as a prerequisite for true background sync.

### CONCERN-SYS-5: Cursor pagination on NULL `published_at` silently drops items
**Document**: data-model.md  
**Finding**: `fi.published_at < ?` with NULL `published_at` always yields NULL (not TRUE), dropping those items from paginated pages 2+.  
**Fix applied**: Made `published_at INTEGER NOT NULL DEFAULT (unixepoch())`. Normalization layer enforces non-null, falling back to `fetched_at`.

### CONCERN-SYS-6: GUID reuse in poorly-formed RSS feeds causes silent drops
**Document**: data-model.md  
**Finding**: Some RSS feeds reuse GUIDs across different items. UUIDv5 collision → `INSERT OR IGNORE` silently drops the second item.  
**Fix applied**: Added feed-level diagnostic: when an `INSERT OR IGNORE` is ignored, compare content hash of incoming item vs stored. If different, log a `GUID_REUSE_DETECTED` warning in feed health.

### CONCERN-SYS-7: 256MB `mmap_size` contradicts 200MB RSS memory budget on Android
**Document**: data-model.md  
**Finding**: 256MB mmap inflates VSZ significantly and can affect Android OOM scoring. Contradicts the 200MB RSS ceiling.  
**Fix applied**: `mmap_size = 0` on Android builds (disable mmap). Keep 256MB for desktop only. Controlled via platform-conditional config at connection open time.

### SUGGESTION-SYS-1: `avg_latency_ms INTEGER` loses EMA precision
**Fix applied**: Changed to `avg_latency_ms REAL`.

### SUGGESTION-SYS-2: `filter_rules.scope` embeds FK ID in string — breaks referential integrity
**Fix applied**: Replaced `scope TEXT` with two columns: `scope_type TEXT` ('global'|'group'|'feed') and `scope_id TEXT` (nullable).

### SUGGESTION-SYS-3: Missing `PRAGMA busy_timeout`
**Fix applied**: Added `PRAGMA busy_timeout = 5000;` to connection initialization.

### SUGGESTION-SYS-4: `GROUP_CONCAT` for tags has comma-ambiguity; not accounted for in IPC budget
**Fix applied**: Changed to `json_group_array(at.tag)` in timeline query. Noted IPC serialization (~50-100KB JSON for 50 items) in performance budget.

---

## CLI/UX Review

### BLOCKER-CLI-1: `pulse sync run` non-blocking by default — no feedback, wrong default
**Document**: cli-ux.md  
**Finding**: Returning immediately with no output is the wrong default for a terminal command. Developers expect to see progress.  
**Fix applied**: Made `--wait` the default. Added `--detach` flag for background/cron use. Blocking mode emits per-feed progress lines to stderr.

### BLOCKER-CLI-2: `pulse feed edit --url` missing but referenced in error hints
**Document**: cli-ux.md  
**Finding**: Direct contradiction — the error hint says to use `--url` but the command doesn't have it.  
**Fix applied**: Added `--url <new-url>` to `pulse feed edit`.

### CONCERN-CLI-3: `item tags` vs `item tag` single-character naming collision
**Document**: cli-ux.md  
**Finding**: `tags` (show) and `tag` (add) differ by one character; `tag` with no TAG arg errors cryptically.  
**Fix applied**: Renamed show command to `pulse item tags show <ID>`. The `tag` verb stays for adding. Added `pulse item mark-read --all [--before <date>] [--group <name>]` for bulk operations.

### CONCERN-CLI-4: Timeline human output has no item ID — prevents scripting and fzf integration
**Document**: cli-ux.md  
**Finding**: Without an ID column, `--json` is mandatory for any scripting.  
**Fix applied**: Added a truncated ID prefix (8 chars) as the first column in human-readable timeline output. Added fzf usage example to docs.

### CONCERN-CLI-5: AI rule creation is interactive-only
**Document**: cli-ux.md  
**Finding**: `pulse ai rules add` with no flags breaks scripting and dotfiles setup.  
**Fix applied**: Added flag-based creation: `pulse ai rules add --tag <name> --keyword <word> [--regex] [--field title|body|both] [--confidence <float>]`. Added `pulse ai rules export` and `pulse ai rules import <file>`.

### CONCERN-CLI-6: Group commands use name as PK — inconsistent with feed ID usage
**Document**: cli-ux.md  
**Finding**: Feed commands use UUIDs; group commands use display names. After rename, scripts break.  
**Fix applied**: `pulse group list` now shows group ID. All group commands accept both name and ID. Documented quoting requirement for names with spaces.

### CONCERN-CLI-7: `h` keybinding hides items (destructive) — conflicts with vim back/close muscle memory
**Document**: cli-ux.md  
**Finding**: Vim users expect `h` = left/back. Accidental hides are hard to notice.  
**Fix applied**: Moved hide to `H` (capital). `h` now closes preview / goes back. Added `Ctrl-d` / `Ctrl-u` for half-page scrolling.

### CONCERN-CLI-8: `completions` command missing from top-level command tree
**Document**: cli-ux.md  
**Finding**: Completions feature exists but is undiscoverable.  
**Fix applied**: Added `completions` to top-level COMMANDS. Added `--install` flag.

### CONCERN-CLI-9: `--json` absent on many read-oriented commands
**Document**: cli-ux.md  
**Finding**: `pulse item show`, `pulse sync status`, `pulse diag`, `pulse db stats`, `pulse feed show` all lack `--json`.  
**Fix applied**: Added `--json` to all read-oriented commands. Defined canonical `FeedSummary` vs `FeedItemDetail` JSON types. JSON errors emit structured JSON to stderr.

### CONCERN-CLI-10: Config `[defaults]` sparse; `keybindings` is a no-op
**Document**: cli-ux.md  
**Fix applied**: Expanded `[defaults]` with `search_limit`, `open_command`, `date_format`. Added `[keybindings]` table for per-key overrides.

### SUGGESTION-CLI-11: OPML in `db` namespace — semantic mismatch
**Fix applied**: Moved `import-opml` / `export-opml` to `pulse feed import-opml` and `pulse feed export-opml`. Added `pulse db restore <PATH>`.

### SUGGESTION-CLI-12: 6-char truncated UUID IDs — collision risk and not tab-completable
**Fix applied**: Human-facing IDs use human-readable slugs derived at feed creation (e.g., `rust-r`, `pragmatic-eng`). Internal UUIDs remain as-is. Slug collision: append `-2`, `-3`. Shell completions query the DB for slug matching.

---

## AI/ML Pipeline Review

### BLOCKER-AI-1: Zero-shot embedding similarity accuracy is 55-70%, not 75-85%
**Document**: ai-pipeline.md  
**Finding**: Bi-encoder cosine similarity (what the pipeline uses) achieves 55-70% macro F1 on topic classification. The 75-85% figure comes from NLI-based zero-shot which uses an entailment head — a fundamentally different approach.  
**Fix applied**: Corrected accuracy claim to 55-70% in documentation. Added note that model tags are "fuzzy signal" tier vs. rule tags which are deterministic. UI/CLI should display model tags with a distinct lower-trust visual treatment. Alternative: NLI cross-encoder (cross-encoder/nli-MiniLM2-L6-H768) documented as a higher-accuracy option at the cost of size (~85MB).

### BLOCKER-AI-2: `ort` pseudo-code mixes incompatible API versions
**Document**: ai-pipeline.md  
**Finding**: Code mixes `ort` 1.x (`Environment`) and 2.x (no Environment) idioms. `ort::ExecutionProvider::NNAPI` is not a valid path in any version.  
**Fix applied**: Marked pseudo-code as conceptual only; added note to pin `ort = "2.0"` and link to the crate's official 2.x examples. NNAPI enablement requirements (Android AAR) documented.

### BLOCKER-AI-3: Contradictory tokenizer references (`tokenizers` vs `rust-tokenizers`)
**Document**: ai-pipeline.md, performance-budget.md  
**Finding**: Two different crates named. `rust-tokenizers` is unmaintained and uses incompatible vocabulary formats. Using the wrong tokenizer for a model produces silent garbage output.  
**Fix applied**: All references standardized to `tokenizers` (HuggingFace Rust crate). `rust-tokenizers` removed entirely. Performance budget updated.

### CONCERN-AI-4: MobileBERT inference on Snapdragon 7xx underestimated (CPU-only: 150-350ms, not 80ms)
**Document**: ai-pipeline.md, performance-budget.md  
**Finding**: 80ms is achievable on Snapdragon 8-series with NNAPI. CPU-only on 7-series is 150-350ms. NNAPI on BERT models is also unreliable (vendor-specific operator support).  
**Fix applied**: Revised model table to show realistic mobile CPU-only estimates. Performance budget changed to "< 400ms CPU-only on Snapdragon 7xx." UX design updated: tagging is always async/non-blocking, so latency is not user-visible.

### CONCERN-AI-5: Single global threshold; raw cosine as confidence is uncalibrated
**Document**: ai-pipeline.md  
**Finding**: Different tags need different thresholds. Cosine similarity is not calibrated probability.  
**Fix applied**: `THRESHOLD` is now per-label (defined in the label description table). Documented that confidence values are relative similarities, not calibrated probabilities — communicated clearly in UX.

### CONCERN-AI-6: Unbounded tagging queue with silent failure
**Document**: ai-pipeline.md  
**Finding**: Unbounded mpsc channel; ONNX session failures silently produce no tags.  
**Fix applied**: Changed to bounded channel (capacity 200). When full, incoming item IDs are logged as `tagged_status = 'skipped'` in a new `tagging_status` column. Added `pulse ai retag --pending` to re-queue skipped items. Permanent errors (corrupt model) surface immediately.

### CONCERN-AI-7: `ragebait` score+comment heuristic flags all popular content
**Document**: ai-pipeline.md  
**Finding**: `HasScore { min: 100 } AND HasComments { min: 200 }` matches every popular post, not just ragebait.  
**Fix applied**: Removed score+comment heuristic from ragebait rule. Rule now uses title-pattern matching only. Confidence lowered to 0.50; rule is opt-in (disabled by default). Users who want it enable it explicitly.

### CONCERN-AI-8: `ask-hn` in tag schema but no built-in rule; HN type detection via title regex is fragile
**Document**: ai-pipeline.md  
**Finding**: `ask-hn` tag never assigned. HN `source_meta.type` is more reliable than title regex.  
**Fix applied**: Added explicit `ask-hn` rule. Added `RulePattern::MetaField { key, value }` variant to match `source_meta` JSON. `show-hn` and `ask-hn` rules now use `MetaField("type", "show"|"ask")` as primary pattern (title regex as fallback).

### CONCERN-AI-9: No model update mechanism — versioning only checks corruption, not staleness
**Document**: ai-pipeline.md  
**Finding**: No path to update a model without manually removing and re-downloading.  
**Fix applied**: Added `schema_version INTEGER` and `released_at INTEGER` to `ai_models`. Added manifest URL concept (local file for Phase 1, remote for Phase 4). Download uses temp path + atomic rename. Old file retained until new version confirmed healthy.

### CONCERN-AI-10: `paywall` and `video` tags have no detection mechanism
**Document**: ai-pipeline.md  
**Finding**: Embedding similarity cannot detect paywalls or video content reliably.  
**Fix applied**: Added `DomainMatch` rules for known-paywall domains (NYT, WSJ, FT, The Atlantic, Wired) and video platforms (youtube.com, vimeo.com, youtu.be). `video` also detects via RSS enclosure MIME type `video/*`. Both are deterministic and require no model.

### SUGGESTION-AI-11: Batch throughput inconsistent with sequential queue
**Fix noted**: Revised target to "> 3 items/sec mobile, > 10 items/sec desktop." Batch inference (size 4-8) documented as Phase 4 optimization.

### SUGGESTION-AI-12: English-only rule engine assumption not documented at point of use
**Fix applied**: Added per-language note at the Built-In Rules section header. Added `language` field to `feeds` table in data model. Rule engine skips non-English feeds with a configurable override.

---

## Net Changes to Documents

| Document | Changes |
|---|---|
| data-model.md | FTS5 trigger fix, published_at NOT NULL, filter_rules schema, mmap_size, busy_timeout, avg_latency_ms REAL, json_group_array, FULL sync on Android, language column in feeds |
| architecture.md | DB writer actor pattern, Android background sync reality |
| sync-model.md | HN top-N strategy, explicit transaction for batch upsert, Android foreground-only |
| ai-pipeline.md | Accuracy claims, tokenizer, ort note, ragebait rule, ask-hn rule, paywall/video rules, bounded queue, model versioning |
| cli-ux.md | sync run default, --url flag, item tags rename, bulk actions, ID in timeline, fzf example, AI rules flags, group IDs, keybindings (h→H), completions in tree, --json expansion, OPML move, config expansion |
| performance-budget.md | Mobile inference times, IPC serialization note |
| workspace-structure.md | No changes needed |
| ui-adaptation-strategy.md | No changes needed |
