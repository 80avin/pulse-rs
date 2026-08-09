# Pulse

[![Rust](https://img.shields.io/badge/Rust-1.95+-orange?logo=rust)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-blue?logo=tauri)](https://tauri.app/)
[![SvelteKit](https://img.shields.io/badge/SvelteKit-5.x-ff3e00?logo=svelte)](https://kit.svelte.dev/)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20Android-lightgrey)](#platform-support)

A local-first, privacy-first feed reader with on-device, deterministic rule-based tagging — **no AI/ML models, no cloud, no telemetry**.

Pulse aggregates Hacker News, Reddit, and RSS feeds, then tags each post with a deterministic on-device rule engine so you can filter the signal from the noise. Everything runs on your device; nothing ever leaves it.

---

## Screenshots

| Feeds                                                                        | Sources                                                                      |
| ---------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| [![](./docs/screenshots/01-feeds.png)](./docs/screenshots/01-feeds.png)      | [![](./docs/screenshots/02-sources.png)](./docs/screenshots/02-sources.png)  |
| Home / Overview                                                              | Saved (by save-month)                                                         |
| [![](./docs/screenshots/03-home.png)](./docs/screenshots/03-home.png)        | [![](./docs/screenshots/04-saved.png)](./docs/screenshots/04-saved.png)      |
| Reader                                                                       |
| [![](./docs/screenshots/05-reader.png)](./docs/screenshots/05-reader.png)    |

---

## Features

**Feeds**

- Hacker News, Reddit subreddits, and any RSS/Atom feed
- **68-feed curated catalog** for onboarding/discovery, grouped by category (Rust & Systems, Security, Compilers & PL, Mailing Lists — flagged experimental, …)
- **Feed preview** before subscribing — see a feed's current items without adding it
- Bulk import from a JSON export (`feed import-json`)
- Source grouping, per-source sync status and health indicators (success rate, latency, failure streak)
- ETag/Last-Modified caching and exponential backoff (60s → 4h) for failing feeds
- Cursor-based pagination — loads fast regardless of database size
- Full-text search across the entire database (SQLite FTS5)

**Home & Overview**

- The search tab is a **Home** screen: a global search bar plus a grid of per-group cards with recent items, total/unread counts, and one-tap drill-in
- Desktop shell toggles between **Overview** and **Feeds** in the left rail

**Saved & Reading**

- Save items with an optional note; the Saved view is **grouped by the month you saved them** (newest saved first)
- Distraction-free reader with sanitized HTML body
- Feed-body links open **externally** (never in the webview) with hover/long-press **URL preview**
- Relative feed-image URLs are resolved at ingestion (http media → https), with an automatic backfill for previously-synced items
- Keyboard navigation (j/k, m, s, o, x, ?) on desktop

**Tagging** _(on-device, deterministic, rules-only)_

- **No models, no downloads** — tags come from a deterministic rule engine (`pulse-core/src/ai/rules.rs`)
- Structural rules (`show-hn`, `ask-hn`, `job-posting`, `paywall`, `video`, `low-effort`) + keyword/regex semantic rules (`technical`, `tutorial`, `research`, `news`, `security`, `ai-ml`, `privacy`, `policy`, `science`, `clickbait`, `civic`, `local-rec`, `culture`, `marketplace`)
- Tags are _filters_, not categories — designed to let you exclude noise, not just label subjects
- Every tag carries a plain-text explanation of which pattern matched

**Android**

- Share any URL from any app → Pulse detects the feed and shows an add-feed sheet
- Detects YouTube channels/playlists, GitHub repos, Substack, Medium, Dev.to, and generic RSS/Atom without leaving the app

---

## Platform support

| Platform | Status |
| -------- | ------ |
| Linux    | ✅     |
| Android  | ✅ APK |

---

## Building

**Prerequisites:** Rust 1.95+, Node.js 22+, pnpm, Tauri CLI v2

```bash
# Clone
git clone https://github.com/80avin/pulse-rs
cd pulse-rs

# Install frontend deps
pnpm install

# Desktop dev server (hot reload)
pnpm tauri dev

# Desktop release build
pnpm tauri build

# Android APK (requires Android SDK + NDK)
pnpm tauri android build
```

**Frontend only:**

```bash
pnpm dev            # Vite dev server (port 1420)
pnpm build          # production SvelteKit build
pnpm check          # svelte-check (types + templates)
```

**Rust:**

```bash
cargo build -p pulse-cli          # CLI only
cargo test -p pulse-core          # core unit tests
cargo clippy --all                # lint
cargo fmt --all                   # format
```

**CLI** (useful for backend testing without the UI — always pass `--data-dir` on a dev machine):

```bash
cargo build -p pulse-cli
./target/debug/pulse --data-dir .pulse-data feed list
./target/debug/pulse --data-dir .pulse-data feed add https://example.com/feed.xml --group tech
./target/debug/pulse --data-dir .pulse-data feed preview https://lobste.rs/rss   # preview without subscribing
./target/debug/pulse --data-dir .pulse-data sync run --feed <id>
./target/debug/pulse --data-dir .pulse-data timeline
./target/debug/pulse --data-dir .pulse-data search "rust async"
./target/debug/pulse --data-dir .pulse-data ai run        # (re)tag items with the rule engine
./target/debug/pulse --data-dir .pulse-data db stats
```

---

## Tagging

Tags are produced by a deterministic rule engine — no models, no downloads, no confidence thresholds. Tune the rules in `pulse-core/src/ai/rules.rs`; run `pulse ai rules list` to inspect them and `pulse ai run` to (re)tag items.

The goal of the tagging system is **spam filtering**, not subject classification. Tags exist to answer the question: _"Is this the kind of post I want to see?"_ — not _"What is this post about?"_

A post can be correctly identified as being about technology and still be low-effort noise. The tagger is designed to surface those distinctions:

| Tag           | Fires on                             | Skips                             |
| ------------- | ------------------------------------ | --------------------------------- |
| `low-effort`  | Single-word titles, score ≤ −5       | Any post with substantive content |
| `local-rec`   | "Best dentist in [city]?"            | "Help with anything?"             |
| `marketplace` | "Selling my laptop — ₹40k"           | News, complaints, art             |
| `civic`       | "Power outage — no water for 3 days" | Travel, food, marketplace         |
| `clickbait`   | "You won't believe what X did"       | Straightforward news              |

Lazy or vague posts get no tags. **The absence of a tag is itself a filter signal.** If you filter your feed to only show `technical` or `research` posts, everything without those tags is implicitly excluded.

Full tag reference and pipeline details: [docs/tagging.md](docs/tagging.md)

---

## Architecture

```
pulse-core/   — all business logic; no platform I/O assumptions
pulse-cli/    — thin CLI for scripting and backend testing
src-tauri/    — Tauri shell: IPC commands, Android bridge
src/          — SvelteKit 5 UI
```

Pulse uses sqlx + SQLite (WAL) with a single-writer actor for all writes, a bounded async tagger queue (rules-only), cursor-based timeline pagination, and an FTS5 search service. The Tauri IPC layer is a thin mapping from Tauri commands to `PulseCore` methods — no business logic lives in the shell.

See [docs/architecture.md](docs/architecture.md) for the full system reference, and [docs/data-model.md](docs/data-model.md) for the schema.

---

## Data & privacy

All data is stored locally in SQLite:

- Linux/macOS: `$XDG_DATA_HOME/pulse` (fallback `~/.local/share/pulse`)
- Windows: `%APPDATA%\pulse`
- Android: app-private data directory (survives updates)

No accounts, no cloud sync, no analytics, no telemetry.

---

## License

MIT
