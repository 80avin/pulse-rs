# Pulse — AI Pipeline

## Philosophy

The AI system in Pulse is local filtering infrastructure, not a chatbot or recommendation engine. It answers the question: "What kind of content is this?" — not "What should I show the user next?"

Core commitments:
- **Local-only**: no item content leaves the device
- **Optional**: the app functions fully with AI disabled
- **Transparent**: every tag has an explanation the user can read
- **Deterministic Phase 1**: rule-based tagging before model inference is introduced
- **No engagement optimization**: tags describe content type, not engagement probability

## Pipeline Stages

```
Feed Item (new)
    │
    ▼
[1] Text Normalization
    - Strip HTML tags
    - Decode HTML entities
    - Collapse whitespace
    - Truncate to 512 tokens (for model compat) / 2000 chars (for rules)
    - Combine: title + " " + body_text
    │
    ▼
[2] Tagging Engine (dispatched by feature flag)
    │
    ├─ Phase 1: Rule Engine
    │    - Evaluate ordered list of TagRules against normalized text
    │    - Each rule: {pattern: Regex, tag: String, confidence: f32, explanation: String}
    │    - Multiple rules can match; all matching tags are stored
    │
    └─ Phase 4: ONNX Model Inference
         - Tokenize text using model's tokenizer (rust-tokenizers or tokenizers crate)
         - Run forward pass via ort (ONNX Runtime)
         - Get embeddings or classification logits
         - Map logits/cosine-similarity to tag labels with confidence scores
         - Generate explanation from top activated features
    │
    ▼
[3] Tag Storage
    - INSERT OR REPLACE into ai_tags
    - Upsert per (item_id, tag, tagger_source) unique constraint
    │
    ▼
[4] Filter Evaluation
    - Check active filter_rules against new tags
    - Auto-hide items matching hide rules
    - Emit notification for highlight rules
```

## Phase 1: Rule Engine

The rule engine is the Phase 1 implementation. It is entirely deterministic and requires no model download.

### Rule Schema

```rust
pub struct TagRule {
    pub id: String,
    pub tag: String,
    pub confidence: f32,                 // fixed confidence (e.g., 0.95 for exact match)
    pub explanation_template: String,    // "matched keyword: {matched_text}"
    pub patterns: Vec<RulePattern>,
    pub scope: RuleScope,               // All | TitleOnly | BodyOnly
    pub require_all: bool,              // AND vs OR across patterns
}

pub enum RulePattern {
    Keyword(String),                    // case-insensitive substring
    Regex(Regex),                       // precompiled regex
    DomainMatch(String),                // matches url domain
    HasScore { min: i64 },             // score >= threshold (Reddit/HN)
    HasComments { min: i64 },          // comment_count >= threshold
    FeedType(FeedType),                // only matches items from specific source type
}
```

### Built-In Rules (Defaults)

```
Tag: "technical"
  Patterns: keywords ["github.com", "crates.io", "npm", "docker", "kubernetes",
                       "api", "framework", "library", "algorithm", "performance",
                       "rust", "python", "typescript", "golang", "sql", "linux"]
  Scope: All
  Confidence: 0.80

Tag: "tutorial"
  Patterns: keywords ["how to", "tutorial", "guide", "step by step", "getting started",
                       "introduction to", "beginner", "learn", "walkthrough"]
  Scope: TitleOnly (title is most reliable signal for tutorials)
  Confidence: 0.85

Tag: "research"
  Patterns: keywords ["arxiv.org", "paper", "study", "research", "findings",
                       "dataset", "benchmark", "evaluation", "methodology"]
  Scope: All
  Confidence: 0.80

Tag: "news"
  Patterns: keywords ["announces", "releases", "launches", "acquires", "raises",
                       "funding", "partnership", "breach", "outage"]
  Scope: TitleOnly
  Confidence: 0.75

Tag: "discussion"
  Patterns: keywords ["ask hn:", "ask reddit:", "discussion:", "thoughts on",
                       "what do you think", "opinion"]
  Scope: TitleOnly
  Confidence: 0.90

Tag: "clickbait"
  Patterns: keywords ["you won't believe", "shocking", "mind-blowing", "game-changing",
                       "destroyed", "killed", "obliterated", "?!", "!!!"]
  Patterns: regex [r"\d+ reasons why", r"the [a-z]+ that changed everything"]
  Scope: TitleOnly
  Confidence: 0.85

Tag: "low-effort"
  Patterns: HasScore { min: -5 } AND body_text length < 50
  FeedType: Reddit
  Confidence: 0.70

Tag: "ragebait"
  -- DISABLED BY DEFAULT (opt-in). High false-positive risk.
  -- Score/comment-count heuristics intentionally excluded: high engagement
  -- is NOT a reliable ragebait signal — legitimate popular content scores just as high.
  Patterns: keywords ["they want to destroy", "is destroying our", "BREAKING ALERT:"]
  Scope: TitleOnly
  Confidence: 0.50 (low — ragebait is semantically ambiguous; use as soft signal only)

Tag: "job-posting"
  Patterns: keywords ["who is hiring", "hiring", "job opening", "we're looking for",
                       "join our team"]
  FeedType: HN | Reddit
  Confidence: 0.90

Tag: "show-hn"
  Patterns: MetaField("type", "show") [primary — from source_meta.type]
  Patterns: keywords ["show hn:"] [fallback — title regex]
  Scope: TitleOnly
  FeedType: HN
  Confidence: 0.99

Tag: "ask-hn"
  Patterns: MetaField("type", "ask") [primary — from source_meta.type]
  Patterns: keywords ["ask hn:"] [fallback — title regex]
  Scope: TitleOnly
  FeedType: HN
  Confidence: 0.99

Tag: "paywall"
  Patterns: DomainMatch(["nytimes.com", "wsj.com", "ft.com", "theatlantic.com",
                          "wired.com", "technologyreview.com", "economist.com",
                          "bloomberg.com/news"])
  Confidence: 0.95 (domain list is deterministic)

Tag: "video"
  Patterns: DomainMatch(["youtube.com", "youtu.be", "vimeo.com", "twitch.tv"])
  Patterns: RSS enclosure MIME type matching "video/*"
  Confidence: 0.99
```

Users can add, edit, and disable individual rules via the CLI or UI.

### Rule Evaluation

Rules are evaluated in priority order. The engine does not short-circuit: all rules are evaluated to allow multiple tags per item. This differs from typical rule engines that stop on first match.

Performance: evaluating 20 rules against a 2000-char string takes < 1ms per item. Rule evaluation is synchronous and runs on the tagging task without `spawn_blocking`.

## Phase 4: ONNX Model Inference

ONNX inference is deferred to Phase 4. The infrastructure (model table, tagging pipeline, tag storage) is built in Phase 1 but the ONNX branch is behind the `ai-onnx` feature flag.

### Model Selection

All models serve as text embedders or zero-shot classifiers. We use zero-shot classification via cosine similarity between item embeddings and label embeddings ("this article is about [tag]").

| Model | Size (q8) | Desktop inference | Mobile (Snap 7xx, CPU-only) | Memory (runtime) | Notes |
|---|---|---|---|---|---|
| MobileBERT-uncased | ~25MB | ~30ms | ~150-300ms | ~80-100MB | Recommended default |
| MiniLM-L6-v2 | ~23MB | ~20ms | ~100-200ms | ~60-80MB | Fastest |
| TinyBERT | ~17MB | ~15ms | ~80-150ms | ~50-70MB | Most efficient; lower accuracy |
| DistilBERT-base | ~45MB | ~60ms | ~250-400ms | ~120-150MB | Better accuracy; heavy for mobile |

Mobile inference times are CPU-only estimates for Snapdragon 7-series (SM7325). NNAPI acceleration is possible but unreliable for BERT attention ops on non-Qualcomm-8-series chips — do not depend on NNAPI being available. All mobile inference must be asynchronous and non-blocking; latency is not user-visible since tagging happens in the background after items appear in the timeline.

Models above 50MB on-device are impractical as defaults. DistilBERT is opt-in for desktop users who prioritize accuracy.

### ONNX Runtime Integration

Pin `ort = "2.0"` in Cargo.toml. The `ort` 2.x API removed `Environment`; sessions are self-contained. The pseudo-code below uses 2.x idioms — do not mix with 1.x examples (which still appear in many blog posts and docs). NNAPI on Android requires the `onnxruntime-android` AAR bundled via Gradle and explicit execution provider registration; it is not automatic.

```rust
// Conceptual pseudo-code for ort 2.x integration (Phase 4)
// Pin to specific ort version; API has changed significantly between major versions.
use ort::{Session, SessionBuilder};

pub struct OnnxTagger {
    session: Session,
    tokenizer: Tokenizer,
    label_embeddings: Vec<(String, Vec<f32>)>,  // pre-computed label embeddings
}

impl OnnxTagger {
    pub fn tag(&self, text: &str) -> Vec<TagResult> {
        let encoding = self.tokenizer.encode(text, 512);
        let input_ids = Value::from_array(encoding.input_ids);
        let attention_mask = Value::from_array(encoding.attention_mask);

        let outputs = self.session.run(ort::inputs![input_ids, attention_mask]).unwrap();
        let embeddings: Vec<f32> = outputs[0].extract_raw_tensor()...;

        // Cosine similarity against each label embedding
        self.label_embeddings.iter().filter_map(|(label, label_emb)| {
            let sim = cosine_similarity(&embeddings, label_emb);
            if sim > THRESHOLD {
                Some(TagResult {
                    tag: label.clone(),
                    confidence: sim,
                    explanation: format!("Model similarity: {:.2}", sim),
                    source: TaggerSource::Model { name: self.model_name.clone() },
                })
            } else {
                None
            }
        }).collect()
    }
}
```

### Label Embedding Pre-Computation

For each tag, we define a set of natural language descriptions. These are embedded once at model load time and cached in memory.

```rust
static TAG_DESCRIPTIONS: &[(&str, &[&str])] = &[
    ("technical", &[
        "programming tutorial or software documentation",
        "technical article about software development",
        "code library or framework announcement",
    ]),
    ("research", &[
        "academic paper or research findings",
        "scientific study or empirical analysis",
    ]),
    // ...
];
```

Each tag's label embedding is the mean of its description embeddings. Cosine similarity between the item embedding and each label embedding produces a score.

### Tokenizer

Use the `tokenizers` crate (HuggingFace's Rust tokenizer library) for fast, accurate tokenization. This crate consumes the `tokenizer.json` configuration files that ship alongside HuggingFace ONNX models — which is exactly what the download flow already retrieves.

**Do not use `rust-tokenizers`** — that is a different, unmaintained community crate with an incompatible API and vocabulary file format. Using the wrong tokenizer for a model produces silent garbage output (the vocabulary mapping is wrong, so all token sequences are incorrect).

## Tag Schema

Canonical tag names (lowercase, hyphenated):

| Tag | Meaning |
|---|---|
| `technical` | Technical/developer content |
| `tutorial` | How-to guides and walkthroughs |
| `research` | Academic papers, studies, empirical results |
| `news` | Announcements, events, current affairs |
| `discussion` | Community discussion threads, opinion pieces |
| `job-posting` | Job listings, hiring announcements |
| `show-hn` | Show HN posts |
| `ask-hn` | Ask HN posts |
| `clickbait` | Sensationalized or misleading titles |
| `ragebait` | Content designed to provoke anger/outrage |
| `low-effort` | Short, low-quality posts |
| `meme` | Memes, humor, image posts |
| `paywall` | Content behind a paywall |
| `video` | Video content (YouTube, etc.) |
| `podcast` | Podcast episodes |

Tags are strings, not an enum, to allow user-defined custom tags without code changes.

## Model Management

### Download Flow

```
1. User selects model (CLI: pulse ai model download mobilebert)
2. Verify model is in ai_models table
3. Show storage size warning (e.g., "MobileBERT requires ~25MB")
4. Download ONNX file to platform data dir: {data_dir}/models/{model_id}/model.onnx
5. Download tokenizer config: {data_dir}/models/{model_id}/tokenizer.json
6. Verify SHA256 checksum
7. Update ai_models.is_downloaded = 1, downloaded_at, file_path
8. Optionally set as active: UPDATE ai_models SET is_active = 0; UPDATE ... SET is_active = 1
```

### Model Removal

```
1. Check that model is not is_active (or prompt to switch first)
2. Delete files from {data_dir}/models/{model_id}/
3. UPDATE ai_models SET is_downloaded = 0, file_path = NULL, is_active = 0
```

### Model Versioning

Each model row has a fixed `sha256` checksum. On startup, if `is_downloaded = 1`, verify the file checksum. If it fails (file corrupted), reset `is_downloaded = 0` and notify the user.

Model format: ONNX with int8 quantization (`q8`). This halves model size vs float32 with <5% accuracy loss on classification tasks.

## Tagging Queue

The tagging pipeline runs as a singleton Tokio task driven by a **bounded** mpsc channel:

```rust
// Bounded to 200 — when full, excess items are logged as 'skipped'
// and can be re-queued later via `pulse ai retag --pending`
let (tx, mut rx) = mpsc::channel::<ItemId>(200);

pub async fn tagging_task(mut rx: mpsc::Receiver<ItemId>, core: Arc<PulseCore>) {
    while let Some(item_id) = rx.recv().await {
        match core.tag_item(&item_id).await {
            Ok(tags) => tracing::debug!(?item_id, ?tags, "tagged"),
            Err(TaggingError::ModelNotLoaded) => {
                // Permanent failure — surface to user, don't keep retrying
                tracing::error!("AI model not available; disabling tagging. Run 'pulse ai model download'");
                break;  // exit task; will be restarted when model is configured
            }
            Err(e) => tracing::warn!(?item_id, ?e, "tagging failed (non-fatal)"),
        }
    }
}
```

When the channel is full (backlog of 200 items during a large sync), incoming item IDs are dropped from tagging. The item appears in the timeline immediately without tags. `pulse ai retag --pending` re-queues items with no AI tags.

Tagging failures are non-fatal: items without tags are displayed normally. A model error never blocks the feed. Permanent errors (corrupt model file, model not downloaded) cause the task to exit cleanly — the user is notified via `pulse ai status`.

## Transparency Requirements

Every `ai_tags` row **must** have a non-empty `explanation` string. This is enforced at the application layer (not DB constraint, since the content is user-visible prose):

- Rule-based: `"matched keyword 'github.com' in title"`
- Model-based: `"high similarity (0.87) to technical content patterns"`
- User-assigned: `"manually tagged by user"`

The CLI and UI always surface explanations on demand (`pulse item tags <id>`).

## Filter Integration

AI tags power the filter rule system. Example filter rules:

```json
{
  "name": "Hide clickbait",
  "action": "hide",
  "conditions": [
    {"field": "ai_tag", "op": "contains", "value": "clickbait"},
    {"field": "ai_tag_confidence", "op": "gte", "value": 0.8}
  ]
}
```

```json
{
  "name": "Highlight research papers",
  "action": "highlight",
  "conditions": [
    {"field": "ai_tag", "op": "contains", "value": "research"}
  ]
}
```

Filter rules evaluate after tagging completes. They can be applied retroactively by re-running the filter engine over all items (useful when a new rule is added).

## Inference on Android

Android-specific considerations for ONNX inference:

1. **Thread management**: ONNX Runtime creates its own thread pool. Configure `inter_op_num_threads = 2` to avoid overwhelming mobile CPUs.
2. **Power mode**: On battery saver, skip AI tagging during sync; tag in batch when plugged in or when the user explicitly requests it.
3. **Model storage**: Models live in app-internal storage (not SD card). App-internal storage is always available, no permissions required.
4. **Memory**: ONNX session keeps the model in memory while active. On low-memory events (Android's `onTrimMemory`), unload the session and reload on next use.
5. **NNAPI**: ONNX Runtime supports Android's Neural Networks API for hardware acceleration. This can reduce inference time by 2-4x on modern Snapdragon chips. Enable via `ort::ExecutionProvider::NNAPI` when available.

## Known Limitations

1. **Zero-shot classification accuracy**: The bi-encoder cosine-similarity approach (document embedding vs. label description embedding) achieves ~55-70% macro F1 on topic classification tasks. The commonly cited 75-85% figures apply to NLI-based zero-shot models (which use an entailment head — a different approach). Mitigation: (a) the rule engine provides a deterministic, high-precision floor for structural tags; (b) model tags are presented as "fuzzy signal" in the UI with a visually distinct treatment from rule tags; (c) high-accuracy alternative: `cross-encoder/nli-MiniLM2-L6-H768` uses NLI and achieves ~80% but is ~85MB. Document this clearly so users have correct expectations.

2. **Multilingual support**: Models above are English-focused. Non-English feeds will have lower accuracy. Future: add multilingual model option (paraphrase-multilingual-MiniLM).

3. **Context window**: BERT-family models are limited to 512 tokens. Long articles are truncated. Mitigation: use the first 400 tokens of body text, always including the full title.

4. **Model staleness**: Tag categories evolve as the internet does. "Ragebait" patterns from 2024 may not match 2026 patterns. Mitigation: rule engine rules are user-editable; model updates via model version bumps.
