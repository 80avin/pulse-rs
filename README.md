# Pulse

[![Rust](https://img.shields.io/badge/Rust-1.75+-orange?logo=rust)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-blue?logo=tauri)](https://tauri.app/)
[![SvelteKit](https://img.shields.io/badge/SvelteKit-5.x-ff3e00?logo=svelte)](https://kit.svelte.dev/)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows%20%7C%20Android-lightgrey)](#platform-support)

A local-first feed reader with on-device AI filtering. Built for people who know what they want to read and are tired of algorithmic noise.

Pulse aggregates Hacker News, Reddit, and RSS feeds, then uses a hybrid on-device AI pipeline to tag and filter posts — no cloud, no telemetry, no subscription.

---

## Screenshots

<!-- TODO: add screenshots -->
*Main feed · Reader · Settings*

---

## Features

**Feeds**
- Hacker News, Reddit subreddits, and any RSS/Atom feed
- Source grouping, per-source sync status and health indicators
- Cursor-based pagination — loads fast regardless of database size
- Full-text search across the entire database (SQLite FTS5)
- og_image thumbnails, crosspost detection, score/comment metadata

**AI filtering**
- On-device only — nothing leaves your device
- Hybrid pipeline: deterministic rules → FastText (9.6 MB, <1 ms/item) → MiniLM semantic classifier → CLIP vision tagger for image posts
- Tags are *filters*, not categories — designed to let you exclude noise, not just label subjects
- 20 tags across three tiers: structural (`show-hn`, `job-posting`, `paywall`, `video`, `low-effort`), semantic (`technical`, `research`, `ai-ml`, `security`, `news`, `clickbait`, …), and community (`civic`, `local-rec`, `culture`, `marketplace`)
- Models hot-reloadable without restart; FastText bundled, larger models downloaded on demand

**Android**
- Share any URL from any app → Pulse detects the feed and shows an add-feed sheet
- Detects YouTube channels/playlists, GitHub repos, Substack, Medium, Dev.to, and generic RSS/Atom without leaving the app

**Reader**
- Distraction-free reader view with sanitized HTML body
- Keyboard navigation (j/k, m, s, o, ?)
- Mark read on open, save for later, hide

---

## Platform support

| Platform | Status |
|---|---|
| Linux | ✅ |
| macOS | ✅ |
| Windows | ✅ |
| Android | ✅ APK |

---

## Building

**Prerequisites:** Rust 1.75+, Node.js 20+, pnpm, Tauri CLI v2

```bash
# Clone
git clone https://github.com/avinthakur080/pulse-rs
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

**CLI only** (useful for backend testing without the UI):

```bash
cargo build -p pulse-cli
./target/debug/pulse --data-dir .pulse-data feed list
./target/debug/pulse --data-dir .pulse-data sync run --feed-id <id>
./target/debug/pulse --data-dir .pulse-data timeline
```

---

## AI tagging

The goal of the tagging system is **spam filtering**, not subject classification. Tags exist to answer the question: *"Is this the kind of post I want to see?"* — not *"What is this post about?"*

A post can be correctly identified as being about technology and still be low-effort noise. The tagger is designed to surface those distinctions:

| Tag | Fires on | Skips |
|---|---|---|
| `low-effort` | Single-word titles, score ≤ −3 | Any post with substantive content |
| `local-rec` | "Best dentist in [city]?" | "Help with anything?" |
| `marketplace` | "Selling my laptop — ₹40k" | News, complaints, art |
| `civic` | "Power outage — no water for 3 days" | Travel, food, marketplace |
| `clickbait` | "You won't believe what X did" | Straightforward news |

Lazy or vague posts get no tags. **The absence of a tag is itself a filter signal.** If you filter your feed to only show `technical` or `research` posts, everything without those tags is implicitly excluded.

Full tag reference and pipeline details: [CLAUDE.md](CLAUDE.md#ai-tagging-pipeline)

---

## Architecture

```
pulse-core/   — all business logic; no platform I/O assumptions
pulse-cli/    — thin CLI for scripting and backend testing
src-tauri/    — Tauri shell: IPC commands, model bundling, Android bridge
src/          — SvelteKit 5 UI
```

Pulse uses a single-writer actor for all SQLite writes (WAL mode), a bounded async tagger queue, and cursor-based timeline pagination. The Tauri IPC layer is a thin mapping from Tauri commands to `PulseCore` methods — no business logic lives in the shell.

See [CLAUDE.md](CLAUDE.md) for the full architecture reference.

---

## Data & privacy

All data is stored locally in SQLite:

- Linux/macOS: `~/.local/share/pulse/`
- Windows: `%APPDATA%\pulse\`
- Android: app-private data directory (survives updates)

No accounts, no sync, no analytics.

---

## License

MIT
