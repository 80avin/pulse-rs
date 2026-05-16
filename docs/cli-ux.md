# Pulse — CLI UX Design

## Design Philosophy

The CLI has two modes that serve different users:

1. **Scriptable subcommands** — for automation, shell scripting, piping to `jq`, cron jobs
2. **Interactive TUI** — for humans who want a terminal feed reader (primary Phase 1 interface)

Both modes share the same `pulse-core` backend. The CLI is a thin translation layer.

The CLI aesthetic mirrors the product philosophy: dense, fast, explicit. No spinner animations on short operations. No color by default (pipe-safe). Color opt-in via `--color always` or auto-detected via `isatty()`.

## Command Tree

```
pulse [OPTIONS] <COMMAND>

OPTIONS:
  --db <PATH>       Override database path
  --color <when>    Color output: auto|always|never [default: auto]
  --json            Output as JSON (machine-readable)
  --quiet           Suppress informational output
  --verbose         Show debug information
  -h, --help

COMMANDS:
  feed         Manage feed sources
  group        Manage feed groups
  timeline     Browse the unified timeline
  item         Inspect and act on individual items
  search       Search items
  sync         Control the sync engine
  ai           AI tagging pipeline management
  db           Database utilities
  diag         Diagnostics and health
  tui          Launch interactive TUI
  completions  Generate shell completion scripts
```

---

### `pulse feed`

```
pulse feed import-opml <PATH>     Import feeds from OPML file
pulse feed export-opml <PATH>     Export feeds as OPML file

pulse feed add <URL> [OPTIONS]
  --type <type>       Force feed type: rss|hn|reddit (auto-detected if omitted)
  --group <name>      Add to group (creates group if it doesn't exist)
  --interval <secs>   Override default poll interval
  --name <title>      Override feed title

pulse feed list [OPTIONS]
  --group <name>      Filter by group
  --json              Machine-readable output
  --health            Include health metrics columns

pulse feed show <ID>              Full details for one feed
pulse feed remove <ID> [--yes]    Remove feed and all its items
pulse feed enable <ID>
pulse feed disable <ID>
pulse feed edit <ID> [OPTIONS]    Edit feed settings
  --url <new-url>               Change the fetch URL (for feeds that have moved)
  --interval <secs>
  --group <name>
  --name <title>
pulse feed health [<ID>]          Health summary (all feeds or one)
```

**`pulse feed list` output (human-readable, default)**:

```
ID       TYPE    TITLE                           GROUP      INTERVAL  HEALTH  LAST SYNC
a1b2c3   rss     The Pragmatic Engineer          tech       60m       ✓ 100%  2m ago
d4e5f6   hn      Hacker News Top Stories         -          15m       ✓ 99%   8m ago
g7h8i9   reddit  r/rust                          tech       20m       ✓ 100%  12m ago
j0k1l2   rss     Import AI                       ai         60m       ✗ 40%   3h ago (failing)
```

Color coding (when enabled): green=healthy, yellow=stale, red=failing.

**`--json` output**:

```json
[
  {
    "id": "a1b2c3",
    "url": "https://newsletter.pragmaticengineer.com/feed",
    "feed_type": "rss",
    "title": "The Pragmatic Engineer",
    "group_id": "tech-group-id",
    "group_name": "tech",
    "poll_interval_secs": 3600,
    "failure_streak": 0,
    "success_rate_pct": 100.0,
    "avg_latency_ms": 423,
    "last_success_at": 1716123456,
    "last_item_at": 1716100000,
    "is_enabled": true
  }
]
```

---

### `pulse group`

```
pulse group create <NAME> [--description <text>] [--color <hex>]
pulse group list [--json]
pulse group show <NAME>
pulse group delete <NAME> [--yes]    Does not delete feeds; ungroups them
pulse group rename <NAME> <NEW_NAME>
pulse group add-feed <GROUP> <FEED_ID>
pulse group remove-feed <GROUP> <FEED_ID>
```

---

### `pulse timeline`

```
pulse timeline [OPTIONS]
  --group <name>      Filter to a group
  --feed <id>         Filter to a single feed
  --limit <n>         Items to show [default: 50]
  --unread            Show only unread items
  --saved             Show only saved items
  --tag <tag>         Filter by AI tag
  --after <datetime>  Only items after this time (ISO 8601 or relative "1d")
  --before <datetime>
  --json
```

**Human-readable output** (compact, one line per item):

```
[unread] 2h ago  ★ 847   The Pragmatic Engineer  "On Being a Staff Engineer"
[unread] 3h ago  ▲2431  r/rust                  "Announcing Rust 2025 Edition"
[read]   4h ago  ▲ 156  r/programming           "Why Rust's borrow checker works"
[saved]  5h ago  ★1203  HN Top                  "Ask HN: What are you building?"
```

Fields: `[state]  age  score  feed-title  "item-title"`

`★` = HN score (stars = karma), `▲` = Reddit upvotes. Score shows as `-` if unavailable.

---

### `pulse item`

```
pulse item show <ID>          Full item details (all fields)
pulse item open <ID>          Open URL in default browser
pulse item read <ID>          Mark as read
pulse item unread <ID>        Mark as unread
pulse item save <ID>          Save item
pulse item unsave <ID>        Remove from saved
pulse item hide <ID>          Hide item from timeline
pulse item unhide <ID>
pulse item tags show <ID>     Show AI tags with explanations (note: 'tags show', not 'tags')
pulse item tag <ID> <TAG>     Manually add a user tag
pulse item untag <ID> <TAG>   Remove a user tag
```

**`pulse item show` output**:

```
Title:      On Being a Staff Engineer
Feed:       The Pragmatic Engineer (rss)
URL:        https://newsletter.pragmaticengineer.com/p/staff-engineer
Published:  2026-05-14 09:32 UTC (2 days ago)
Score:      -
Comments:   -
Word count: ~2400 words (~12 min read)
State:      unread

AI Tags:
  technical   (0.82)  "matched keyword 'engineering' in title and body"
  research    (0.71)  "matched keyword 'study' in body"

User Tags:   (none)

Body (preview):
  The transition from senior engineer to staff engineer is one of the most
  underappreciated career shifts in software. Unlike the move from mid-level
  to senior, which is largely about technical depth...
```

---

### `pulse search`

```
pulse search <QUERY> [OPTIONS]
  --group <name>      Limit to group
  --feed <id>         Limit to feed
  --limit <n>         [default: 20]
  --json
```

Uses FTS5 full-text search. Query syntax: SQLite FTS5 match syntax (supports phrase matching with `"quotes"`, prefix with `*`, column filter with `title:term`).

```
$ pulse search "staff engineer"
$ pulse search "rust* performance"
$ pulse search 'title:"Show HN"'
```

---

### `pulse sync`

```
pulse sync run [OPTIONS]          Run sync and wait for completion (blocking by default)
  --feed <id>                     Sync only this feed
  --detach                        Return immediately; sync runs in background (for cron/scripts)

pulse sync status                 Current sync state for all feeds
pulse sync schedule               Show next_fetch_at for all feeds
pulse sync pause                  Pause all background sync
pulse sync resume
```

**`pulse sync run`** default behavior: triggers async sync (non-blocking). Use `--wait` to block until complete.

---

### `pulse ai`

```
pulse ai run [OPTIONS]            Run AI tagging on untagged items
  --feed <id>
  --limit <n>

pulse ai status                   Tagging queue depth, active model, etc.

pulse ai model list [--json]      List available models
pulse ai model download <MODEL>   Download a model
pulse ai model use <MODEL>        Set active model
pulse ai model remove <MODEL>     Remove downloaded model

pulse ai rules list [--json]
pulse ai rules show <RULE_ID>
pulse ai rules add [OPTIONS]      Add a rule (interactive if no flags; flag-based for scripting)
  --tag <name>
  --keyword <word>              (repeatable)
  --regex <pattern>             (repeatable)
  --field title|body|both       [default: both]
  --confidence <0.0-1.0>        [default: 0.80]
  --disable                     Add rule but disable it immediately
pulse ai rules edit <RULE_ID>
pulse ai rules disable <RULE_ID>
pulse ai rules enable <RULE_ID>

pulse ai rules export [--json] [<PATH>]   Export all rules to file
pulse ai rules import <PATH>              Import rules from file

pulse ai retag [OPTIONS]          Re-run tagging on all/filtered items
  --feed <id>
  --pending                       Only re-tag items with no AI tags (queue overflow recovery)
  --clear                         Clear existing AI tags before retagging
```

**`pulse ai model list` output**:

```
MODEL            SIZE    STATUS        DESKTOP   MOBILE (CPU)
mobilebert-q8    25MB    ● active      ~30ms     ~150-300ms
minilm-q8        23MB    ✓ downloaded  ~20ms     ~100-200ms
tinybert-q8      17MB    ○ available   ~15ms     ~80-150ms
distilbert-q8    45MB    ○ available   ~60ms     ~250-400ms
```

---

### `pulse db`

```
pulse db migrate [--dry-run]      Run pending migrations
pulse db stats                    Database size, row counts, index sizes
pulse db vacuum                   VACUUM the database (reclaim space)
pulse db export <PATH>            Export database as JSON (backup)
pulse db restore <PATH> [--dry-run]  Restore from JSON backup
```

**`pulse db stats` output**:

```
Database:     ~/.local/share/pulse/pulse.db
Size:         23.4 MB
WAL file:     1.2 MB

Table               Rows      Index Size
──────────────────────────────────────────
feed_items          84,231    12.1 MB
item_states         84,231     4.2 MB
feed_items_fts      84,231     5.8 MB (FTS index)
ai_tags            247,892     3.1 MB
feeds                   42    12 KB
feed_groups              8     4 KB
```

---

### `pulse diag`

```
pulse diag                        Full system diagnostics
pulse diag feed <id>              Feed-specific diagnostics
pulse diag sync                   Sync engine state
pulse diag ai                     AI pipeline state
pulse diag db                     Database health (integrity check)
```

**`pulse diag` output**:

```
Pulse Diagnostic Report — 2026-05-16 14:32:07 UTC

System:
  DB path:          ~/.local/share/pulse/pulse.db
  DB size:          23.4 MB
  Migration ver:    8 (current)
  DB integrity:     OK

Feeds:
  Total:            42
  Enabled:          40
  Healthy (>90%):   37
  Degraded (50-90%):  2
  Failing (<50%):    1

Sync Engine:
  Status:           running
  Active tasks:     40
  Next sync:        in 4 min (r/rust)
  Last sync:        14:28 UTC (d4e5f6 — HN Top Stories, 12 new items)

AI Pipeline:
  Active model:     mobilebert-q8 (downloaded)
  Tagging queue:    0 pending
  Tags applied:     247,892 total
  Last tagged:      14:29 UTC

Errors (last 24h):
  j0k1l2 (Import AI RSS): 3 failures — HTTP 503 Service Unavailable
```

---

### `pulse tui`

Launches the interactive terminal UI. No arguments; configuration comes from the main config.

```
pulse tui [--group <name>]        Start TUI focused on group
```

---

## Interactive TUI Design

The TUI uses `ratatui` and is styled for maximum information density. Default layout:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ PULSE  [tech]  84 unread  sync: 4m ago   /search  q:quit  ?:help          │
├───────────────────────────────────────────────────────────────────────────┬─┤
│ ● 2h  ▲2431  r/rust          Announcing Rust 2025 Edition               │ │
│ ● 2h  ★ 847  Pragmatic Eng   On Being a Staff Engineer                  │▓│
│ ● 3h  ▲ 156  r/programming   Why Rust's borrow checker works             │▓│
│ ● 3h  ★  89  HN Top          Show HN: I built a local feed reader in Rust│ │
│   4h  ▲  43  r/rust          Tokio 2.0 release candidate                 │ │
│   5h  ★1203  HN Top          Ask HN: What are you building in 2026?      │ │
│   6h  ▲  12  r/programming   TIL: SQLite WAL mode significantly improves │ │
│   8h  ★ 234  HN Top          The case against LLM-first development      │ │
│   9h  ▲  88  r/rust          Async Rust finally makes sense to me        │ │
│  12h  ★  67  Pragmatic Eng   Platform teams: what actually works         │ │
│  14h  ▲ 341  r/programming   I traced a memory leak for 3 weeks          │ │
│  16h  ★  29  HN Top          SQLite: the most deployed database          │ │
│                                                                           │ │
└───────────────────────────────────────────────────────────────────────────┴─┘
```

**Column layout** (left to right):
- State dot: `●` = unread, ` ` = read, `★` = saved, `✕` = hidden
- Age: relative (`2h`, `3d`, `1w`)
- Score: `▲NNN` for Reddit, `★NNN` for HN, `   -` if unavailable
- Feed name (truncated to 16 chars)
- Item title (fills remaining width)
- Scrollbar (right edge)

### Keybindings

```
Navigation:
  j / ↓           Move down
  k / ↑           Move up
  g               Go to top
  G               Go to last item
  PgDn / Ctrl-f   Page down
  PgUp / Ctrl-b   Page up
  Ctrl-d          Half-page down (vim convention)
  Ctrl-u          Half-page up

Item Actions:
  Enter / l       Open item preview (right pane or full-screen)
  h               Close preview / go back (mirrors vim: h=left/back)
  o               Open URL in browser
  r               Toggle read/unread
  s               Toggle saved
  H               Hide item (capital H — prevents accidental hide via vim muscle memory)
  t               Show tags for item

View:
  /               Enter search mode
  Esc             Exit search / close preview / back
  Tab             Switch between groups
  ]               Next group
  [               Previous group
  f               Toggle feed filter (show feed selector)
  u               Toggle unread-only filter

Sync:
  R               Refresh current feed / all feeds
  Ctrl-r          Force full sync

UI:
  ?               Toggle help overlay
  q               Quit
  :               Command mode (for less-common commands)
```

### Preview Mode

Pressing `Enter` on an item opens a preview:

**Single-pane (narrow terminals)**:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ← Back(Esc)  On Being a Staff Engineer — The Pragmatic Engineer            │
│ 2026-05-14 09:32 UTC · 2400 words · ~12min                                 │
│ Tags: technical (0.82)  research (0.71)                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ The transition from senior engineer to staff engineer is one of the most   │
│ underappreciated career shifts in software. Unlike the move from            │
│ mid-level to senior, which is largely about technical depth...              │
│                                                                             │
│ [j/k to scroll · o to open in browser · s to save · Esc to return]        │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Dual-pane (wide terminals, ≥140 columns)**:

Left: scrollable item list. Right: auto-updates to show selected item's content.

### Status Bar

The top bar always shows:
- App name
- Active group (or "All")
- Unread count
- Sync status and time since last sync
- Quick command hints

The status bar changes during search mode:
```
│ SEARCH: rust async  [Enter:confirm  Esc:cancel]  12 results               │
```

### Color Scheme

Default: Terminal's own colors (no hardcoded palette). The TUI respects the user's terminal theme.

With `--color always` or in a color-capable terminal:
- Unread items: bold
- Saved items: bold + yellow accent
- Failing feeds (in feed list): red
- Stale feeds: yellow
- AI tags in preview: dim (de-emphasized — informational, not primary UI)

No background colors, no gradients, no decorative borders beyond functional separators.

## Output Format

### Error Format

All errors to stderr. Format:

```
error: feed not found: a1b2c3
  hint: run 'pulse feed list' to see available feeds
```

```
error: HTTP 404 fetching https://example.com/feed.xml
  hint: the feed URL may have changed; use 'pulse feed edit <id> --url <new-url>'
```

Errors have an actionable `hint` where possible.

### JSON Schema

When `--json` is used, all output is valid JSON to stdout. Lists are JSON arrays; single items are JSON objects. Errors still go to stderr as human-readable text (not JSON). This allows:

```bash
pulse timeline --json | jq '.[] | select(.ai_tags | contains(["technical"]))'
pulse feed list --json | jq '.[] | select(.failure_streak > 3)'
```

## Configuration

Config file: `~/.config/pulse/config.toml` (platform-appropriate path).

```toml
[defaults]
timeline_limit = 50
color = "auto"     # auto | always | never

[sync]
min_interval_secs = 300
max_backoff_secs = 14400
user_agent = "Pulse/0.1"

[tui]
dual_pane_threshold = 140    # columns
keybindings = "default"      # future: "vim" | "emacs" | "custom"

[ai]
enabled = true
model = "mobilebert-q8"
min_confidence = 0.7          # minimum confidence to show a tag
auto_hide_threshold = 0.85    # confidence above which auto-hide rules fire
```

### `pulse completions`

```
pulse completions <shell>         Print completion script to stdout (bash|zsh|fish|powershell)
pulse completions <shell> --install  Write completion script to the appropriate location
                                     (~/.bash_completion.d/, ~/.zfunc/, etc.)
```

Shell completions are essential for a CLI where feed IDs and group names are not memorable. Completions query the database dynamically for feed slugs and group names. The `pulse diag` output notes whether completions are installed.
