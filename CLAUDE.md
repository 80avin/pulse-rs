# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

Pulse is a local-first, privacy-first feed reader with on-device AI classification. It aggregates Hacker News, Reddit, and RSS feeds and automatically tags items using a hybrid on-device AI pipeline — no cloud, no telemetry.

**Who uses it:** Developers and productivity-minded people who are frustrated by low signal-to-noise ratio on the internet. They know exactly what they want to read. They don't want an algorithm deciding for them — they want a tool that helps them build and maintain their own curated feed.

**What the app is for:** A powerful aggregator and spam filter. The AI exists to help users *exclude bad or unexpected posts*, not to classify subjects. A post about the latest JavaScript framework drama is correctly identified as technical — but if the user only cares about systems programming, it should be filterable out. Tags are how that filtering works. The quality and specificity of tags directly determines the app's usefulness.

The stack: Rust workspace (pulse-core library + pulse-cli + src-tauri Tauri shell) + SvelteKit frontend.

## Build & development commands

### Rust

```bash
cargo build                                # debug build (all crates)
cargo build --release                      # release build
cargo build -p pulse-cli                   # CLI only
cargo test -p pulse-core                   # core unit tests
cargo test -p pulse-core --all-features    # with all AI feature flags
cargo clippy --all                         # lint all crates
cargo fmt --all                            # format all crates
```

### Frontend

```bash
pnpm dev                   # start Vite dev server (port 1420)
pnpm build                 # production SvelteKit build
pnpm check                 # svelte-check TypeScript + template checking
pnpm check:watch           # watch mode
```

### Tauri desktop app

```bash
pnpm tauri dev             # run desktop app (starts Vite + Tauri together)
pnpm tauri build           # production desktop build
pnpm tauri android build   # Android APK
```

### CLI (after `cargo build`)

```bash
# Always pass --data-dir to avoid writing to system paths on dev machine
./target/debug/pulse --data-dir .pulse-data feed list
./target/debug/pulse --data-dir .pulse-data sync run --feed-id <id>
./target/debug/pulse --data-dir .pulse-data ai run    # batch retag (requires sandbox disabled)
./target/debug/pulse --data-dir .pulse-data timeline  # paginated browse
```

> **Note:** `pulse ai run` and anything that writes to SQLite will fail under the default sandbox. Disable the sandbox for those commands.

### Python training scripts

```bash
uv venv && source .venv/bin/activate   # always use uv, not pip
uv pip install -r scripts/requirements.txt
python scripts/train_fasttext.py
python scripts/train_miniml.py
```

## Architecture

### Crate layout

```
pulse-core/   — all business logic; zero platform I/O assumptions
pulse-cli/    — thin CLI front-end (clap) for scripting and backend testing
src-tauri/    — Tauri shell: app setup, model bundling, IPC commands
src/          — SvelteKit UI (Svelte 5, TypeScript)
```

### pulse-core internals

`PulseCore` (`crates/pulse-core/src/lib.rs`) is the single entry-point. It holds:

- **`DbHandle`** — single-writer actor + read pool (SQLite WAL mode). All writes go through typed `DbCommand` messages to `db_writer_task`; reads use `with_reader(closure)`.
- **`SyncScheduler`** — manages per-feed background tasks with exponential backoff (60s → 4h), ETag/Last-Modified caching, and health tracking (failure_streak, avg_latency_ms).
- **`TaggerHandle`** — bounded async queue (size 200) feeding `tagger_task`. Items enter the queue immediately after upsert; the task dispatches to the active model combination.
- **`TimelineService`** — cursor-based pagination over `FeedItemView` (joined: item + feed + group + state + tags). Cursor is an opaque `(published_at, item_id)` tuple. `get_items_page` is the only item query command; there is no `get_items`.
- **`SearchService`** — FTS5 full-text search backed by `fts_items` virtual table. Searches the entire database, not just the loaded page.
- **`RuleEngine`** — structural tag rules (regex + keyword) run synchronously before any ML model.

### AI tagging pipeline

Tags flow through four layers, executed in order:

1. **`RuleEngine`** — deterministic structural rules → tags like `show-hn`, `job-posting`, `paywall`, `video`, `low-effort`
2. **`FastTextTagger`** — 9.6 MB supervised .ftz classifier, <1 ms/item; bundled in the binary and extracted on first run
3. **`MiniMlTagger`** — MiniLM-L6 ONNX + 201 KB MLP head; semantic classification for nuanced categories
4. **`OnnxTagger`** (legacy) — DeBERTa NLI cross-encoder (35 MB quantized); zero-shot but slow; kept for compatibility
5. **`VisionTagger`** — CLIP ViT-B/32 for image-only posts (no text body)

Active combination is set by `TextBackend` enum in `PulseConfig`. Default is `HybridFastTextMiniMl`. Models can be hot-reloaded (`reload_*_tagger()`) without restarting the app. FastText + MiniLM MLP head are bundled as `include_bytes!` in `src-tauri/src/lib.rs`; CLIP and full MiniLM ONNX are downloaded on demand.

**Tag vocabulary (22 tags):**

| Tag | Source | Description |
|---|---|---|
| `show-hn` | rules | "Show HN:" prefix |
| `ask-hn` | rules | "Ask HN:" prefix |
| `job-posting` | rules | Hiring / job post signals |
| `paywall` | rules | Paywall indicators in title |
| `video` | rules | Video content |
| `low-effort` | rules | Minimal title, very low score |
| `technical` | ml | Engineering, systems, code |
| `tutorial` | ml | How-to, guide, walkthrough |
| `research` | ml | Papers, studies, academic content |
| `news` | ml | Factual event reports, announcements |
| `security` | ml | Vulnerabilities, exploits, privacy incidents |
| `ai-ml` | ml | Machine learning, AI systems |
| `privacy` | ml | Surveillance, data rights, tracking |
| `policy` | ml | Regulation, law, governance |
| `science` | ml | Scientific findings outside CS |
| `clickbait` | ml | Sensational, misleading framing |
| `civic` | rules+ml | Infrastructure failures, governance complaints |
| `local-rec` | rules+ml | Specific local service recommendations |
| `culture` | rules+ml | Regional heritage, folk traditions, arts |
| `marketplace` | rules+ml | Buy/sell/rent/hire listings |

### Tag design philosophy

Tags are *earned signals*, not default labels. A post only gets a tag when there is strong evidence of a specific, useful category. The goal is to give users filters they can act on — filters that meaningfully separate the posts they want from the ones they don't.

**What this means in practice:**

- A vague question ("thoughts on X?") gets no tag. A specific service query ("best optometrist in [city]?") gets `local-rec`.
- A complaint with a named authority or utility gets `civic`. A general gripe does not.
- A listing with a price and a thing to buy/sell gets `marketplace`. Mentions of money in other contexts do not.
- A post tagged `security` should be about a real vulnerability or incident — not just any post that mentions "privileges" or "access."

**The absence of a tag is a signal.** A user who filters to `technical` or `research` is implicitly excluding everything that didn't earn those tags. This is the primary mechanism by which Pulse acts as a spam filter. Every new tag must pass the question: *can a user meaningfully filter on this, and will it fire on the right posts?*

When adding or tuning tags: test both true positives (posts that should fire) and false positives (posts that must not fire). Raise thresholds when in doubt. A missed tag is an annoyance; a wrong tag erodes trust in the whole system.

### Tauri IPC layer

`src-tauri/src/commands.rs` — all `#[tauri::command]` functions. They receive `State<AppState>` (which holds `Arc<PulseCore>`) and return serializable DTOs defined in `src-tauri/src/models.rs`. Event payloads (`DownloadProgressEvent`, `TaggingProgressEvent`, `IncomingShareEvent`) are emitted via `app.emit()`.

`AppState` lives in `src-tauri/src/lib.rs` alongside:
- `extract_bundled_models()` — copies embedded FastText + MiniLM MLP head to `data_dir` on first launch, re-extracts FastText when `BUNDLED_FASTTEXT_VERSION` changes.
- `APP_HANDLE: OnceLock<AppHandle>` + `PENDING_SHARE: OnceLock<Mutex<Option<String>>>` — the JNI bridge writes to these; the setup closure drains `PENDING_SHARE` and emits `share://incoming-url` after an 800ms delay to let the WebView register its listener.

### Android share intent

When the user shares a URL from any Android app:

1. `MainActivity.kt` intercepts `ACTION_SEND` (text/plain) and `ACTION_VIEW` (http/https) intents and calls `ShareBridge.onShareUrl(url)`.
2. `ShareBridge.kt` is a Kotlin object with an `external fun` — the JNI symbol resolves to the Rust function `Java_com_avinthakur080_pulse_1rs_ShareBridge_onShareUrl` in `src-tauri/src/lib.rs`.
3. The Rust JNI function emits `share://incoming-url` via `APP_HANDLE`, or buffers the URL in `PENDING_SHARE` if the app is cold-starting.
4. `src/lib/share.svelte.ts` listens for `share://incoming-url` and calls `detect_feed` (a Tauri command).
5. `detect_feed_url` in `crates/pulse-core/src/feeds/detect.rs` tries patterns in order: Reddit → HN → well-known sites (YouTube, GitHub, Substack, Medium, Dev.to, Hashnode) → HTTP fetch → HTML `<link rel="alternate">` scraping → fallback with `no_feed_found: true`.
6. The result populates `ShareSheet.svelte`, a bottom sheet where the user can confirm, edit the URL/name/type/group, and add the feed.

### Frontend layout

The UI is responsive-bifurcated: `+page.svelte` checks `window.innerWidth` and renders either `Desktop.svelte` or `Mobile.svelte` from `src/lib/screens/`. All Tauri commands are called via `@tauri-apps/api/core` `invoke()`.

`+layout.svelte` sets up two persistent listeners: the AI tagging progress listener (from `store.svelte.ts`) and the share intent listener (from `share.svelte.ts`). It also renders `<ShareSheet />` when `shareSheet.candidate !== null`.

Desktop FTS search: a debounced `$effect` (300ms) calls `searchItems()` when the search box is non-empty and `IS_TAURI` is true. Results override the paginated `filteredItems` derived store. When the box is cleared, pagination resumes from where it left off.

`og_image` pipeline: `FeedItemView.og_image` (DB) → `FeedItemDto.og_image` (Tauri DTO) → `BackendItem.ogImage` (store adapter) → `FeedItem.ogImage` (frontend type) → trailing `<img>` in `ItemRow.svelte`. The image is hidden via `onerror` if it fails to load.

### Data model

Key types in `crates/pulse-core/src/types.rs`:

- `FeedItemView` — flattened read model for the UI (joined item + feed + group + state + tags)
- `ItemStatePatch` — partial update for read/saved/hidden
- `AiTag { tag, confidence, tagger_source, rule_id, model_name }` — stored per-item in `ai_tags` table with `ON CONFLICT DO UPDATE`
- `TimelineFilter` — optional `group_id`, `feed_id`, `is_read`, `is_saved`, `tag` predicates
- Frontend DTOs in `src-tauri/src/models.rs` use `camelCase` serde rename (Tauri convention)

### Storage schema

- `feeds` + `feed_groups` — source metadata and health
- `items` — normalized content (UUIDv5 for deterministic idempotent upserts)
- `item_states` — per-item user state (read/saved/hidden), separate table for clean separation
- `ai_tags` — tags with confidence and source attribution
- `fts_items` — FTS5 virtual table over title + body_text

### Platform data directories

Resolved by `platform_data_dir()` in `config.rs`:

- Linux/macOS: `$XDG_DATA_HOME/pulse` (or `~/.local/share/pulse`)
- Windows: `%APPDATA%\pulse`
- Android: Tauri's `app_data_dir()` — keyed to package ID, survives APK updates

## Critical development rules

### Cross-platform consistency

Every feature or fix must work on **both desktop and Android** unless a platform difference is explicitly requested. This includes: data directory resolution, model loading (bundled vs. download), sync scheduling, and all Tauri commands. `PulseConfig::is_android` gates platform-specific behavior — check it before adding any platform fork.

### Don't substitute a different feature

If the intended implementation is blocked (missing API, unclear design, model not ready), **stop and ask** — never silently implement a different feature that "works similarly." The user needs complete information to make the right call. This applies especially to: AI model substitution, sync behavior changes, and UI interaction patterns.

### Bandage fixes vs. proper design

Before proposing any fix or change, ask: is this addressing the root cause, or papering over a symptom? The architecture has clear separation (pulse-core has no I/O assumptions, the writer actor is the single DB mutator, the tagger queue is bounded). Changes that violate these invariants are bandage fixes, not improvements.

### Research → critique → implement

For non-trivial changes, use subagents to research and critique the approach before writing code. This is especially important for: schema changes, AI pipeline modifications, sync scheduling logic, and Tauri command additions. The workflow is documented in `feedback-workflow.md` memory.

### Tauri command additions

When adding a new Tauri command:
1. Add the core logic or query to `pulse-core` first (testable via CLI)
2. Add the DTO to `src-tauri/src/models.rs` with `camelCase` serde
3. Add the `#[tauri::command]` function to `src-tauri/src/commands.rs`
4. Register it in the `tauri::Builder::invoke_handler` in `src-tauri/src/lib.rs`
5. Add the frontend `invoke()` call with matching TypeScript types

### DB writes go through the actor

Never write to SQLite directly from a reader context or from outside `db_writer_task`. All mutations are `DbCommand` variants sent through the `DbHandle`. Adding a new write operation means: add a `DbCommand` variant, handle it in `db_writer_task`, and expose it as a method on `DbHandle`.

### AI model feature flags

pulse-core compiles with optional AI features: `ai-rules`, `ai-onnx`, `ai-vision`, `ai-fasttext`, `ai-miniml`. The Tauri shell enables all of them; the CLI enables only what's needed. Don't hard-require a feature in shared code paths — always gate with `#[cfg(feature = "...")]` or runtime `Option<...>`.

### Python scripts use uv

Never use `pip install` directly. Always use `uv venv` + `uv pip install`. The system Python rejects pip without `--break-system-packages`.

### Package manager: pnpm only

All JS/TS operations use `pnpm`. Never use `npm` or `yarn`.
