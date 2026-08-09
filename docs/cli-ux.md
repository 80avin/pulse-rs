# Pulse — CLI

The CLI is a thin front-end over `pulse-core` for scripting and backend testing. Output is dense and pipe-safe: human-readable by default, valid JSON on `--json`. There is no interactive TUI.

> **Important:** on a dev machine, always pass `--data-dir .pulse-data` (or `--db <path>`) so Pulse writes to your workspace instead of the system path (`~/.local/share/pulse` or `%APPDATA%\pulse`). Everything that writes to SQLite (sync, item state, ai run, feed add, …) needs a writable data dir.

## Global options

```
pulse [OPTIONS] <COMMAND>

OPTIONS:
      --data-dir <PATH>             Override data directory (DB + settings). Takes precedence over --db.
      --db <PATH>                   Override database path (ignored if --data-dir is set)
      --reddit-client-id <ID>       Reddit OAuth2 client ID (script app). Also REDDIT_CLIENT_ID env.
      --reddit-client-secret <SEC>  Reddit OAuth2 client secret. Also REDDIT_CLIENT_SECRET env.
      --json                        Output as JSON (machine-readable)
  -h, --help
  -V, --version

COMMANDS:
  feed      Manage feed sources
  group     Manage feed groups
  timeline  Browse the unified timeline
  item      Inspect and act on individual items
  search    Search items
  sync      Control the sync engine
  enrich    Enrich items with Open Graph metadata
  ai        Tagging pipeline management (rules only)
  db        Database utilities
  diag      Diagnostics and health
```

ID shorthand: list commands print 8-character ID prefixes. Every command that takes an ID (`feed show/remove/edit/health`, `timeline --feed`, `item *`, `sync run --feed`, `ai run --feed`, `enrich --feed`) accepts a full UUID **or a unique prefix**.

---

### `pulse feed`

```
pulse feed add <URL> [--type rss|hn|reddit] [--group <name>] [--name <title>] [--interval <secs>]
pulse feed list [--group <name>] [--json]
pulse feed show <ID> [--json]
pulse feed remove <ID> [--yes]
pulse feed enable <ID>
pulse feed disable <ID>
pulse feed edit <ID> [--url <new-url>] [--interval <secs>] [--group <name>] [--name <title>]
pulse feed health [<ID>] [--json]
pulse feed import-json <PATH>
pulse feed preview <URL> [--type rss|hn|reddit] [--limit <n>]
```

- `add` auto-detects the type: `reddit.com` / `r/` → reddit; `topstories`, `askhn`, `showhn`, `hn:…` → HN; anything else → rss. Default intervals: HN 15 min, Reddit 20 min, RSS 60 min.
- `list` prints a table (ID, type, title, group, interval, health %, last sync). `--group` filters by group name.
- `health` prints per-feed success rate, average latency, failure streak, and last success.
- `import-json <PATH>` bulk-adds feeds from a JSON array of `{ "name", "url", "kind": "hn"|"reddit"|"rss", "group" }`. Unknown kinds fall back to RSS; bare HN section names collapse to `https://news.ycombinator.com`; each entry's `group` becomes (or reuses) its category group. Empty entries are skipped and counted.
- `preview <URL>` fetches a feed's current items **without subscribing** — used by the discover/onboarding flow. Nothing is persisted. Reddit preview requires `--reddit-client-id`/`--reddit-client-secret`.

Human-readable `feed list` output:

```
ID       TYPE    TITLE                            GROUP          INTERVAL  HEALTH   LAST SYNC
a1b2c3d4 rss     The Pragmatic Engineer           tech           60m       ✓ 100%   2m ago
d4e5f6g7 hn      Hacker News                      News & Comm.   15m       ✓ 99%    8m ago
g7h8i9j0 reddit  r/rust                           Rust & Systems 20m       ✓ 100%   12m ago
```

`--json` emits an enriched array with `id`, `url`, `feed_type`, `title`, `group_id`, `group_name`, `poll_interval_secs`, `failure_streak`, `success_rate_pct`, `avg_latency_ms`, `last_success_at`, `last_item_at`, `is_enabled`.

---

### `pulse group`

```
pulse group create <NAME> [--description <text>] [--color <hex>]
pulse group list [--json]
pulse group delete <NAME> [--yes]      # does not delete feeds; ungroups them
pulse group add-feed <GROUP> <FEED_ID>
```

---

### `pulse timeline`

```
pulse timeline [--limit <n>] [--unread] [--saved] [--group <name>] [--feed <id>] [--tag <tag>] [--json]
```

Cursor-based pagination over `FeedItemView` (joined item + feed + group + state + tags). Human-readable output is one compact line per item:

```
a1b2c3d4 ●  2h ago  ▲2431  r/rust          "Announcing Rust 2025 Edition"
e5f6g7h8 ★  5h ago  ★1203  HN Top          "Ask HN: What are you building?"
```

Format: `ID · state (● unread / ○ read / ★ saved / ✕ hidden) · age · score (▲ reddit, ★ HN) · feed · "title"`.

---

### `pulse item`

```
pulse item show <ID> [--json]
pulse item read <ID>
pulse item unread <ID>
pulse item save <ID>
pulse item unsave <ID>
pulse item hide <ID>
pulse item unhide <ID>
pulse item tags show <ID> [--json]
pulse item open <ID>
```

`item show` prints title, feed, URL, published time, score, comments, word count, state, and the rule-tag list with explanations. `item tags show` lists the item's tags with confidence, source (`rule`), and explanation. `item open` opens the item URL in the default browser. `save`/`unsave` drive `item_states`; hiding removes the item from timeline views.

---

### `pulse search`

```
pulse search <QUERY> [--limit <n>] [--json]
```

FTS5 full-text search over the entire database (title + body + author), default relevance ranking. Supports FTS5 match syntax: `"phrases"`, `prefix*`, `title:term`.

---

### `pulse sync`

```
pulse sync run [--feed <id>] [--detach] [--force]
pulse sync status [--json]
```

- `run` syncs all enabled feeds (or one feed with `--feed`) **blocking**, printing new-item counts per feed. `--force` clears ETag / Last-Modified / `last_seen_id` first for a full re-fetch.
- `--detach` spawns a detached child process running the blocking sync (in-process tasks would be aborted on CLI exit).
- `status` lists each feed's next sync time, last success, failure streak, and enabled state.

---

### `pulse enrich`

```
pulse enrich [--feed <id>] [--limit <n>] [--concurrency <n>] [--verbose] [--json]
```

Fetches OpenGraph metadata (title/description/image) for link posts that lack `enriched_at` in `source_meta`. Direct image URLs and non-HTTP hosts are skipped. Errors are left pending so they retry next run.

---

### `pulse ai`

The `ai` command manages the **rule engine only** — there are no models.

```
pulse ai run [--feed <id>] [--force]
pulse ai rules list [--json]
```

- `run` tags untagged items (or all items with `--force`, which clears existing tags first — use after tag-vocabulary changes). Prints items processed and tags created.
- `rules list` prints the compiled `default_rules()` table (id, tag, confidence, enabled, pattern count, scope).

---

### `pulse db`

```
pulse db migrate [--dry-run]
pulse db stats [--json]
pulse db vacuum
pulse db fix-relative-urls
```

- `migrate` is a no-op report — migrations run automatically at `PulseCore::init`. `--dry-run` states this.
- `stats` prints database path, size, and row counts (feeds, feed_items, unread, saved, ai_tags).
- `vacuum` runs `VACUUM` through the writer actor, which then rebuilds the FTS index (VACUUM can renumber the implicit rowids the external-content FTS depends on).
- `fix-relative-urls` backfills stored `body_html`: rewrites relative `src`/`href` against the item URL (fallback: feed URL) and upgrades `http` media to `https`. Same pass the M0006 migration runs automatically at startup.

---

### `pulse diag`

```
pulse diag [--json]
```

Prints a report: DB path/size, feed health buckets (healthy >90% / degraded 50–90% / failing <50%), item counts, total `ai_tags`, and the list of currently failing feeds.

---

## Output conventions

- All informational output goes to stdout; progress/errors go to stderr.
- `--json` (or per-subcommand `--json`) makes stdout valid JSON for list/show-style commands. Errors still print as human-readable text on stderr.
- JSON is the scripting interface:

```bash
pulse feed list --json | jq '.[] | select(.failure_streak > 3)'
pulse timeline --json | jq '.[] | select(.ai_tags | contains(["security"]))'
```
