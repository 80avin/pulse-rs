# Pulse — Tagging Pipeline

## Philosophy

Tagging in Pulse is deterministic and fully on-device. There are **no ML models** — the FastText / MiniLM / CLIP stack was removed in v0.6. Every tag is produced by a rule engine that matches structural signals (title prefixes, domains, score thresholds) and keyword/regex patterns against the item's title and body. Nothing leaves the device, nothing is downloaded, and there are no feature flags.

Tags answer the question *"Is this the kind of post I want to see?"* — not *"What is this post about?"*. They are **filters** designed to let users exclude noise. The tagger is the app's spam filter: a post that doesn't earn a tag is implicitly excluded by any filter that selects *for* a tag.

Core commitments:
- **Local-only** — no item content leaves the device
- **Deterministic** — the same item always gets the same tags; no confidence thresholds, no model drift
- **Transparent** — every tag has an explanation string that says exactly which pattern matched
- **Non-blocking** — tagging happens in the background after an item appears in the timeline

## The `Tagger` trait and `RulesTagger`

The pipeline is behind a small seam so a future BYO hosted-model adapter can be dropped in without touching the queue, the IPC layer, or the storage code.

```rust
// crates/pulse-core/src/ai/tagger.rs
pub trait Tagger: Send + Sync {
    fn tag(&self, item: &FeedItem, feed_type: &FeedType) -> Vec<TagResult>;
}
```

`RulesTagger` is the only built-in implementation. It wraps an `Arc<RuleEngine>` and appends the runtime `low-effort` check, which no single `TagRule` can express (it needs both the score and the body length):

```rust
pub struct RulesTagger { engine: Arc<RuleEngine> }

impl Tagger for RulesTagger {
    fn tag(&self, item: &FeedItem, feed_type: &FeedType) -> Vec<TagResult> {
        let mut tags = self.engine.evaluate(item, feed_type);
        if let Some(le) = evaluate_low_effort(item, feed_type) {
            tags.push(le);
        }
        tags
    }
}
```

## The bounded tagging queue

New items enter the queue immediately after upsert. The queue is an `mpsc` channel bounded to **200** (`TAGGER_QUEUE_SIZE`). When the channel is full, incoming item IDs are dropped and logged — the item still appears in the timeline, just without tags (tagging failures are always non-fatal). A single `tagger_task` drains the queue and runs `process_tag_request` for each item.

```
item upserted (sync) ──► tagger.tag_item(id, feed_type)   [try_send, non-blocking]
                                │  channel full → drop + warn
                                ▼
                        tagger_task (single task)
                                │
                                ▼
              process_tag_request(db, tagger, req)
                1. load item (reader pool)
                2. skip direct image URLs (i.redd.it, imgur…) — no text to match
                3. tagger.tag(item, feed_type) → Vec<TagResult>
                4. post-processing (see below)
                5. db.insert_ai_tags(item_id, tags)     [via writer actor]
```

### Post-processing (`process_tag_request`)

Two rules run after the engine, to keep tags consistent:

- **`no-context` removal** — if a *substantive* tag fires (`technical`, `tutorial`, `research`, `news`, `security`, `ai-ml`, `privacy`, `policy`, `science`, `clickbait`, `show-hn`, `ask-hn`, `job-posting`, `paywall`, `video`, `civic`, `local-rec`, `culture`, `marketplace`), the `no-context` tag is removed. A specific question is never "vague".
- **`noise` suppression** — when a `noise` tag fires with confidence ≥ 0.70, the semantic topic tags (`technical`, `security`, `ai-ml`, `policy`, `privacy`, `science`, `research`, `tutorial`) are stripped. A personal food post cannot also be technical content.

## The `RuleEngine`

The engine (`crates/pulse-core/src/ai/rules.rs`) evaluates a list of `TagRule`s. Each rule has an id, a tag, a fixed confidence, an explanation template, a scope, an AND/OR switch (`require_all`), and an enabled flag. Patterns are one of:

```rust
pub enum RulePattern {
    Keyword(String),      // case-insensitive substring
    Regex(Regex),         // precompiled regex
    DomainMatch(String),  // item URL domain contains this string
    HasScore { min: i64 },   // item score >= min (Reddit/HN)
    HasComments { min: i64 },// comment_count >= min
    FeedType(FeedType),   // only matches items from a specific source type
}
```

`RuleEngine::evaluate` does **not** short-circuit — every enabled rule runs, so an item can earn multiple tags (`show-hn` and `technical` together, for instance). Evaluation is synchronous and fast (<1ms per item against a 2000-char string), so it runs directly on the tagger task without `spawn_blocking`.

Rules are defined in `default_rules()` and are compiled-in. The CLI can list them (`pulse ai rules list`) but they are not user-editable at runtime.

## Tag vocabulary

All rules ship enabled unless noted. Confidence is the fixed value written to `ai_tags`; scope is what text the rule matches against.

| Tag | Conf | Scope | Fires on |
|---|---|---|---|
| `show-hn` | 0.99 | Title | Title starts with "Show HN:" |
| `ask-hn` | 0.99 | Title | Title starts with "Ask HN:" |
| `job-posting` | 0.90 | All | "who is hiring", "we are hiring", "join our team", "freelance", … |
| `paywall` | 0.95 | All | Known paywall domains (nytimes.com, wsj.com, ft.com, wired.com, …) |
| `video` | 0.99 | All | YouTube/Vimeo/Twitch domains + "on youtube"/"live on youtube" |
| `low-effort` | 0.70 | — | Runtime check: Reddit + score ≤ −5 + body < 50 chars |
| `technical` | 0.80 | All | Programming languages, frameworks, systems keywords (`rust`, `python`, `docker`, `linux`, `github.com`, `crates.io`, `api`, `compiler`, …) |
| `tutorial` | 0.85 | Title | "how to", "tutorial", "guide", "getting started", "deep dive", "from scratch", … |
| `research` | 0.80 | Title | "arxiv", "study", "dataset", "benchmark", "paper", DomainMatch(arxiv.org, semanticscholar.org, scholar.google.com) |
| `news` | 0.75 | Title | "announces", "releases", "launches", "raises", "breach", "outage", "$N M funding", "ipo", … |
| `security` | 0.85 | All | "vulnerability", "cve-", "zero-day", "ransomware", "malware", "phishing", "data breach", … |
| `ai-ml` | 0.85 | All / Title | ML keywords ("machine learning", "llm", "gpt", "openai", "transformer", …); bare "ai" restricted to titles |
| `privacy` | 0.85 | All | "surveillance", "facial recognition", "gdpr", "ccpa", "vpn", "tor", "end-to-end encryption", … |
| `policy` | 0.80 | All | "legislation", "regulation", "antitrust", "ftc", "congress", "digital markets act", "net neutrality", … |
| `science` | 0.80 | All | "quantum", "physics", "biology", "astronomy", "crispr", "climate change", "nasa", DomainMatch(nature.com, science.org) |
| `clickbait` | 0.85 | Title | "you won't believe", "shocking", "?!", "N reasons why", "the [x] that changed everything", … |
| `civic` | 0.85 | Title | Power/water/road/telecom infrastructure failures and complaints (incl. Hindi/Hinglish: "pani nahi", "batti/bijli", "power cut", "pothole", "bsnl") |
| `local-rec` | 0.82 | Title | Specific local service recommendations: "best dentist", "good gym", "best momos", "[service] in [City]" |
| `culture` | 0.85 | Title | Regional heritage / folk traditions: "dogra", "pahari culture", "gojri", "folk tradition", "bahu fort", "kalari" |
| `marketplace` | 0.90 | Title | "for sale", "wts/wtb/wtt", "for rent", "room available", "looking to buy", "cook needed", … |
| `no-context` | 0.82 | Title | Vague help-seeking phrases ("kya karu", "help chahiye", "any suggestions?", "need advice") — removed when a substantive tag fires |
| `inappropriate` | 0.92 | Title | Solicitation patterns ("hotel for couple", "girls dm me", "hookup", "fwb", "one night stand") |
| `noise` | 0.78 | Title | First-person personal updates ("finally had…", "my gym progress", "good morning", "beer time") — suppresses semantic tags at ≥ 0.70 |

Disabled by default (kept in the ruleset but `enabled: false`):

| Tag | Conf | Notes |
|---|---|---|
| `ragebait` | 0.50 | Opt-in only. High false-positive risk — engagement heuristics were deliberately excluded. |

> **Note on vocabulary.** The core filter vocabulary documented in CLAUDE.md lists 20 tags. The rules file additionally ships `no-context`, `inappropriate`, and `noise` (all enabled) as quality-control tags, plus a disabled `ragebait` rule. `low-effort` is produced by the runtime check, not a `TagRule`.

## Tag storage

Tags are stored in the `ai_tags` table with `tagger_source = 'rule'` and the matching `rule_id`. The insert uses `ON CONFLICT(item_id, tag, tagger_source) DO UPDATE`, so re-tagging an item refreshes confidence/explanation in place. `model_name` / `model_version` columns still exist in the schema (legacy) but are always `NULL`.

The tag distribution for the UI comes from `get_tag_stats` — a `COUNT(*) ... GROUP BY tag` over `ai_tags`.

## Earned signals — the design rule

Tags are *earned*, never default. A post only gets a tag when there is strong evidence of a specific, useful category:

- A vague question ("thoughts on X?") gets no tag. A specific service query ("best optometrist in [city]?") gets `local-rec`.
- A complaint naming an authority or utility gets `civic`. A general gripe does not.
- A listing with a price and a thing to buy/sell gets `marketplace`. Mentions of money do not.
- A post tagged `security` is about a real vulnerability or incident — not any post that mentions "privileges".

**The absence of a tag is a signal.** Filtering to `technical` or `research` implicitly excludes everything that didn't earn those tags. Every rule is tuned for precision over recall — a missed tag is an annoyance; a wrong tag erodes trust in the whole system. When in doubt, raise the threshold.

## Implementation references

- `crates/pulse-core/src/ai/rules.rs` — `RuleEngine`, `TagRule`, `RulePattern`, `default_rules()`, `evaluate_low_effort()`
- `crates/pulse-core/src/ai/tagger.rs` — `Tagger` trait, `RulesTagger`, `TaggerHandle`, `tagger_task`, `process_tag_request`
- `crates/pulse-core/src/storage/actor.rs` — `DbCommand::InsertAiTags` → `insert_ai_tags`
- `crates/pulse-core/src/storage/queries.rs` — `get_ai_tags`, `get_tag_stats`
- `crates/pulse-cli/src/commands/ai.rs` — `pulse ai run`, `pulse ai rules list`
