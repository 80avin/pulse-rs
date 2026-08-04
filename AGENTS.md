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
- **`TaggerHandle`** — bounded async queue (size 200) feeding `tagger_task`. Items enter the queue immediately after upsert; the task tags them with the active `Tagger` implementation.
- **`TimelineService`** — cursor-based pagination over `FeedItemView` (joined: item + feed + group + state + tags). Cursor is an opaque `(published_at, item_id)` tuple. `get_items_page` is the only item query command; there is no `get_items`.
- **`SearchService`** — FTS5 full-text search backed by `fts_items` virtual table. Searches the entire database, not just the loaded page.
- **`RuleEngine`** — deterministic structural tag rules (regex + keyword). This is the only tagger.

### Tagging pipeline

Tags are produced on-device by the **rule engine only** (no ML models — the FastText/MiniLM/CLIP stack was removed in v0.6). The pipeline is a small `Tagger` trait (`item → Vec<TagResult>`) implemented by `RulesTagger`, run inside a bounded async queue:

1. **`RuleEngine`** — deterministic structural rules → tags like `show-hn`, `ask-hn`, `job-posting`, `paywall`, `video`, `low-effort`, plus keyword/regex semantic rules (`technical`, `tutorial`, `research`, `news`, `security`, …).
2. **`RulesTagger`** — wraps the rule engine + the runtime `low-effort` score check behind the `Tagger` trait, so a future BYO hosted-model adapter can be added without touching the queue or IPC layer.

No models, no downloads, no feature flags (`ai-*` removed). The tag distribution for the UI comes from `get_tag_stats` (a `COUNT(*) ... GROUP BY tag` over `ai_tags`).

**Tag vocabulary (20 tags):**

| Tag | Source | Description |
|---|---|---|
| `show-hn` | rules | "Show HN:" prefix |
| `ask-hn` | rules | "Ask HN:" prefix |
| `job-posting` | rules | Hiring / job post signals |
| `paywall` | rules | Paywall indicators in title |
| `video` | rules | Video content |
| `low-effort` | rules | Minimal title, very low score |
| `technical` | rules | Engineering, systems, code |
| `tutorial` | rules | How-to, guide, walkthrough |
| `research` | rules | Papers, studies, academic content |
| `news` | rules | Factual event reports, announcements |
| `security` | rules | Vulnerabilities, exploits, privacy incidents |
| `ai-ml` | rules | Machine learning, AI systems |
| `privacy` | rules | Surveillance, data rights, tracking |
| `policy` | rules | Regulation, law, governance |
| `science` | rules | Scientific findings outside CS |
| `clickbait` | rules | Sensational, misleading framing |
| `civic` | rules | Infrastructure failures, governance complaints |
| `local-rec` | rules | Specific local service recommendations |
| `culture` | rules | Regional heritage, folk traditions, arts |
| `marketplace` | rules | Buy/sell/rent/hire listings |

> All tags now come from the deterministic rule engine. The former ML tags (`ml` source column) are produced by keyword/regex rules in `default_rules()`. 

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

`src-tauri/src/commands.rs` — all `#[tauri::command]` functions. They receive `State<AppState>` (which holds `Arc<PulseCore>`) and return serializable DTOs defined in `src-tauri/src/models.rs`. Event payloads (`TaggingProgressEvent`, `IncomingShareEvent`) are emitted via `app.emit()`.

`AppState` lives in `src-tauri/src/lib.rs` alongside:
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

The UI is responsive-bifurcated: `+page.svelte` checks `window.innerWidth` and renders either `DesktopShell.svelte` or `MobileShell.svelte` from `src/lib/screens/`. All Tauri commands are called via `@tauri-apps/api/core` `invoke()`.

`+layout.svelte` sets up two persistent listeners: the AI tagging progress listener (from `$lib/stores/ai.svelte`) and the share intent listener (from `share.svelte.ts`). It also renders `<ShareSheet />` when `shareSheet.candidate !== null`, injects accent color CSS variables via `<svelte:head>`, applies system/dark/light theme resolution, and sets `data-density` attribute on the root div.

**Styling:** Tailwind CSS v4 with a `@theme` block in `app.css`. All design tokens use `light-dark(LIGHT, DARK)` for automatic theme switching via `color-scheme`. Sizing/spacing uses inline styles (Tailwind arbitrary values don't generate from `.svelte` templates). Color utilities (`text-cyan`, `bg-bg-1`, `border-bd-0`) are standard Tailwind classes from `@theme` tokens. `tokens.ts` bridges JS→CSS vars for dynamic inline styles.

**Theming:** Settings (`settings.svelte.ts`) persist theme (`system`/`dark`/`light`), accent color (cyan/blue/green/amber/violet), and density (dense/normal/roomy). Accent is injected as `--user-accent` CSS var with `light-dark()` awareness. Density sets `--item-pad-y` CSS variable on `[data-density]` selector.

### Store architecture (hub-and-spoke)

State is decomposed into feature modules under `src/lib/stores/`:

| Module | Responsibility |
|--------|---------------|
| `data.svelte.ts` | items[], sources[], groups[], dbStats, initStore (cold-start retry loop), ALL mutation functions |
| `timeline.svelte.ts` | timelineFilter, pageCounts, fetchNextPage, filter setters |
| `ai.svelte.ts` | aiStatus, models[], aiStats, taggingProgress, model CRUD |
| `sync.svelte.ts` | syncState, doSync(), Tauri event listeners, mock sync pool |
| `search.svelte.ts` | searchItems() via FTS5 |
| `settings.svelte.ts` | User preferences persisted to localStorage + Tauri backend |

`data.svelte.ts` is the single source of truth. Feature modules import from it, never from each other. Components import directly from the module they need.

### Component structure

```
src/lib/components/
├── layout/          — DragHandle, StatusBar (extracted from DesktopShell)
├── timeline/        — ItemRow, TimelineList, FilterStrip, GroupTabs, FilterPills
├── reader/          — ReaderView
├── sources/         — SourceExplorer
├── settings/        — SettingsPanelContent, ThemeSection, SettingsSection
├── ai/              — AiPanelContent
├── shell/           — BottomTools (left rail utilities)
└── shared/          — Icon, IconBtn, Modal, ContextActions, TagChip, KeyCap, ScoreBar, Sparkline, Thumb, SourceGlyph, StatusDot, SearchView, ShareSheet, SkipLink
```

Desktop FTS search: a debounced `$effect` (300ms) calls `searchItems()` when the search box is non-empty and `IS_TAURI` is true. Results override the paginated `displayItems` derived store. When the box is cleared, pagination resumes.

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

Every feature or fix must work on **both desktop and Android** unless a platform difference is explicitly requested. This includes: data directory resolution, sync scheduling, and all Tauri commands. `PulseConfig::is_android` gates platform-specific behavior — check it before adding any platform fork.

### Don't substitute a different feature

If the intended implementation is blocked (missing API, unclear design, model not ready), **stop and ask** — never silently implement a different feature that "works similarly." The user needs complete information to make the right call. This applies especially to: AI model substitution, sync behavior changes, and UI interaction patterns.

### Bandage fixes vs. proper design

Before proposing any fix or change, ask: is this addressing the root cause, or papering over a symptom? The architecture has clear separation (pulse-core has no I/O assumptions, the writer actor is the single DB mutator, the tagger queue is bounded). Changes that violate these invariants are bandage fixes, not improvements.

### Research → critique → implement

For non-trivial changes, use subagents to research and critique the approach before writing code. This is especially important for: schema changes, AI pipeline modifications, sync scheduling logic, and Tauri command additions.

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

pulse-core no longer compiles with AI model features (`ai-*` removed with the ML stack in v0.6). Don't re-add model dependencies without a design doc.

### Python scripts use uv

Never use `pip install` directly. Always use `uv venv` + `uv pip install`. The system Python rejects pip without `--break-system-packages`.

### Package manager: pnpm only

All JS/TS operations use `pnpm`. Never use `npm` or `yarn`.

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **pulse-rs** (2216 symbols, 4187 relationships, 185 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> Index stale? Run `node .gitnexus/run.cjs analyze` from the project root — it auto-selects an available runner. No `.gitnexus/run.cjs` yet? `npx gitnexus analyze` (npm 11 crash → `npm i -g gitnexus`; #1939).

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows. For regression review, compare against the default branch: `detect_changes({scope: "compare", base_ref: "main"})`.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `query({search_query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `context({name: "symbolName"})`.
- For security review, `explain({target: "fileOrSymbol"})` lists taint findings (source→sink flows; needs `analyze --pdg`).

## Never Do

- NEVER edit a function, class, or method without first running `impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `rename` which understands the call graph.
- NEVER commit changes without running `detect_changes()` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/pulse-rs/context` | Codebase overview, check index freshness |
| `gitnexus://repo/pulse-rs/clusters` | All functional areas |
| `gitnexus://repo/pulse-rs/processes` | All execution flows |
| `gitnexus://repo/pulse-rs/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
