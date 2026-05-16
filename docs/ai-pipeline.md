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
    - Strip HTML tags, decode entities, collapse whitespace
    - Truncate to 512 tokens / 2000 chars
    - Combine: title + " " + body_text
    │
    ▼
[2] Text Tagging (rule engine + NLI)
    │
    ├─ Rule Engine (always active)
    │    - Keyword/regex patterns, DomainMatch, structural signals
    │    - High-precision tags: show-hn, ask-hn, paywall, video, job-posting
    │
    └─ NLI Cross-Encoder (optional, requires model download)
         - DeBERTa v3 xsmall, ~83MB, entailment-based zero-shot classification
         - Semantic tags: technical, tutorial, research, news, discussion,
           security, ai-ml, privacy, policy, science, clickbait
    │
    ▼
[3] Vision Tagging (optional, requires vision model download)
    - Runs during enrichment, after text tagging
    - Only for items with image URLs (i.redd.it, imgur, etc.)
    - CLIP ViT-B/32 vision encoder, ~53MB download
    - Zero-shot: cosine similarity vs pre-computed label embeddings
    - Visual tags: meme, screenshot, photo-share, infographic
    │
    ▼
[4] Tag Storage
    - ON CONFLICT(item_id, tag, tagger_source) DO UPDATE
    - tagger_source: 'rule' | 'model' | 'vision-model'
    │
    ▼
[5] Filter Evaluation
    - Check active filter_rules against new tags
    - Auto-hide items matching hide rules
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

## Phase 2.5: Vision Tagging (CLIP Zero-Shot)

Addresses the ~40% of Reddit posts that are image-only and currently invisible to the text pipeline.

### Problem

Image posts (Reddit `i.redd.it`, galleries, Imgur) have no body text. Title is often vague ("Help", "Fun", "I miss winters", "aaj ka dinner"). The text pipeline has zero signal. Community/regional feeds are ~40% image posts.

### Model: CLIP ViT-B/32 (vision encoder only)

CLIP (Contrastive Language-Image Pre-training) trains a vision encoder and text encoder jointly so their embedding spaces are aligned. Cosine similarity between an image embedding and a text label embedding ("a meme post") produces meaningful zero-shot classification scores — the same principle as NLI entailment for text.

**Download size:** ~53MB (vision encoder, q4f16 quantized). The text encoder is NOT downloaded by users — label embeddings for our fixed tag set are pre-computed offline and bundled in the binary as const arrays (~16KB).

**Why not the Qwen3.5-0.8B vision encoder:** That model's vision encoder produces patch token sequences for an LLM decoder. Its embedding space is not aligned with text labels. Zero-shot image→text classification requires a CLIP-family model where both encoders are trained together in a shared embedding space.

### Vision Tags

| Tag | Description | Example items |
|---|---|---|
| `meme` | Image-based meme, reaction, humor post | "State mentioned!!!!", "Our city is too big BTW" |
| `screenshot` | Screenshot of tweet, article, code, error, news | "Level of Journalism in our city" |
| `photo-share` | Personal photo — food, travel, scenery, event | "aaj ka dinner", "I miss winters already" |

These tags supplement text tags — an item can be both `technical` (from text) and `screenshot` (from vision).

### Pipeline Integration

Vision tagging runs during enrichment, after text tagging. It does not run during sync (no image downloading in the sync hot path).

```
enrich_item(item):
  1. OpenGraph fetch (existing)
  2. if has_image_url(item) && vision_model_loaded:
       a. GET image URL (max 10MB, timeout 15s, skip on error)
       b. Decode JPEG/PNG → RGB pixels
       c. Resize to 224×224 (bilinear), normalize with CLIP mean/std
       d. ONNX forward pass → [1, 512] embedding
       e. L2-normalize embedding
       f. For each vision label: cosine_sim(embedding, label_embedding)
       g. Apply tag where sim ≥ threshold (per-label, tuned empirically)
       h. Store tags: tagger_source = 'vision-model'
```

`has_image_url(item)` returns true when:
- `item.url` host is `i.redd.it`, `preview.redd.it`, `imgur.com`, `i.imgur.com`
- `item.url` ends with `.jpg`, `.jpeg`, `.png`, `.webp`, `.gif`
- `item.url` host is `reddit.com/gallery` (skip — gallery has multiple images, too complex for v1)

### Pre-Computed Label Embeddings

Label embeddings are computed once offline using the CLIP text encoder with carefully chosen prompts. The prompts are designed to be visually discriminable (CLIP was trained on image-caption pairs, so visual language works better than abstract category names):

```
meme:        "a meme or funny reaction image with text overlay"
screenshot:  "a screenshot of a webpage, tweet, news article, or app"
photo-share: "a personal photograph of food, scenery, or everyday life"
```

Multiple prompts per label are averaged and L2-normalized:

```rust
// In src/ai/vision_labels.rs — generated offline, do not edit by hand
pub static VISION_LABELS: &[VisionTagLabel] = &[
    VisionTagLabel {
        tag: "meme",
        threshold: 0.24,
        // embedding: averaged CLIP text embedding for meme prompts
        embedding: &[0.0234, -0.0412, ...],  // 512 floats
    },
    ...
];
```

To regenerate (requires Python + transformers installed):
```bash
python scripts/compute_clip_labels.py > crates/pulse-core/src/ai/vision_labels.rs
```

### Image Preprocessing

```
CLIP normalization:
  mean = [0.48145466, 0.4578275, 0.40821073]
  std  = [0.26862954, 0.26130258, 0.27577711]

Steps:
  1. Decode image bytes → RGB pixels (image crate)
  2. Resize shortest side to 224, then center crop to 224×224
  3. Convert to f32, divide by 255.0
  4. Subtract mean, divide by std, per channel
  5. Arrange as [1, 3, 224, 224] NCHW tensor
```

### Model Management

Vision model is independent of the text model — different registry, separate download:

```
pulse ai download --type vision clip-vit-b32
pulse ai model list --type vision
pulse ai model set --type vision clip-vit-b32
pulse ai model unset --type vision   # disable vision tagging
```

Files stored at: `{data_dir}/models/vision/{model_name}/vision_model.onnx`

### What This Does NOT Do

- **OCR** (reading text overlaid on memes): requires a separate OCR model or VLM
- **Video frames**: `v.redd.it` posts skipped — frame extraction is out of scope
- **Gallery posts**: `reddit.com/gallery` posts skipped — multiple images, ambiguous
- **Context understanding**: CLIP classifies "what does this look like" not "what does this mean"

### Known Limitations

1. **Multilingual text on images**: CLIP's text encoder was trained on English. Text overlaid on images in Hindi/Hinglish is not decoded — only the visual appearance is classified.
2. **Threshold calibration**: Initial thresholds are approximate. Run `pulse ai vision-debug <image_url>` to inspect raw cosine similarities.
3. **Image download cost**: One JPEG download per image post during enrichment. Rate-limited by the enrichment queue. Respects existing `should_enrich()` guards.
4. **NSFW content**: CLIP will assign embeddings to NSFW images. The tagger does not add NSFW-specific tags. Community moderation remains the user's responsibility via feed selection.

---

## Known Limitations

1. **Zero-shot classification accuracy (text NLI)**: NLI cross-encoder achieves ~75-85% macro F1. The rule engine provides a high-precision floor for structural tags. Tags are fuzzy signals, not ground truth.

2. **Multilingual support**: Models are English-focused. Non-English feeds (Hinglish, Hindi) produce unreliable NLI scores near the threshold. Language detection guard is a future improvement.

3. **Context window**: BERT-family models are limited to 512 tokens. Long articles are truncated to first 400 tokens (title always included).

4. **Model staleness**: Tag categories evolve over time. Rule engine rules are user-editable; model updates via version bumps.
