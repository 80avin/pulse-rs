use std::path::Path;
#[cfg(feature = "ai-onnx")]
use std::path::PathBuf;
use crate::error::TaggingError;
use crate::types::{FeedItem, FeedType, TagResult};
#[cfg(feature = "ai-onnx")]
use crate::types::TaggerSource;
#[cfg(feature = "ai-onnx")]
use super::labels::tag_labels;

#[cfg(feature = "ai-onnx")]
const MAX_SEQ_LEN: usize = 512;

// ── Structural tags handled by rules regardless of model ──────────────────────
// Pattern-based (title prefix, URL domain) — NLI cross-encoder is overkill here.
const RULE_ONLY_TAGS: &[&str] = &["show-hn", "ask-hn", "paywall", "video", "job-posting", "low-effort"];

#[cfg(feature = "ai-onnx")]
struct NliIndices {
    entailment: usize,
    contradiction: usize,
}

pub fn is_rule_only_tag(tag: &str) -> bool {
    RULE_ONLY_TAGS.contains(&tag)
}

/// ONNX-backed tagger using NLI cross-encoder zero-shot classification.
///
/// Inference model: for each (article, label) pair the cross-encoder receives
/// "[CLS] article [SEP] label_hypothesis [SEP]" and returns a 3-class logit
/// vector [contradiction, neutral, entailment]. Softmax(logits)[entailment_idx]
/// is the confidence score. Runs one forward pass per label per article.
///
/// When the `ai-onnx` feature is disabled, `OnnxTagger::load` always returns
/// `Err(TaggingError::ModelNotLoaded)` so callers can hold `Option<OnnxTagger>`
/// without cfg guards.
pub struct OnnxTagger {
    #[cfg(feature = "ai-onnx")]
    inner: OnnxTaggerInner,
}

impl OnnxTagger {
    /// Load an NLI cross-encoder from a model directory containing:
    /// - `model_quantized.onnx` or `model.onnx`
    /// - `tokenizer.json`
    /// - `config.json` (optional — used to detect entailment label index)
    pub fn load(model_dir: &Path) -> Result<Self, TaggingError> {
        #[cfg(feature = "ai-onnx")]
        {
            let inner = OnnxTaggerInner::load(model_dir)?;
            Ok(Self { inner })
        }
        #[cfg(not(feature = "ai-onnx"))]
        {
            let _ = model_dir;
            Err(TaggingError::ModelNotLoaded)
        }
    }

    /// Classify an item via NLI. Returns semantic tags only; structural tags
    /// (show-hn, paywall, etc.) are handled by the rule engine separately.
    pub fn classify(&self, item: &FeedItem, feed_type: &FeedType) -> Result<Vec<TagResult>, TaggingError> {
        #[cfg(feature = "ai-onnx")]
        {
            self.inner.classify(item, feed_type)
        }
        #[cfg(not(feature = "ai-onnx"))]
        {
            let _ = (item, feed_type);
            Err(TaggingError::ModelNotLoaded)
        }
    }

    /// Return raw (tag, entailment_probability) pairs for a text, ignoring thresholds.
    /// Used for calibration via `pulse ai debug`.
    pub fn similarities(&self, text: &str) -> Result<Vec<(String, f32)>, TaggingError> {
        #[cfg(feature = "ai-onnx")]
        {
            self.inner.similarities(text)
        }
        #[cfg(not(feature = "ai-onnx"))]
        {
            let _ = text;
            Err(TaggingError::ModelNotLoaded)
        }
    }
}

// ── Feature-gated implementation ──────────────────────────────────────────────

#[cfg(feature = "ai-onnx")]
struct OnnxTaggerInner {
    session: std::sync::Mutex<ort::session::Session>,
    tokenizer: tokenizers::Tokenizer,
    /// (tag, hypothesis_sentence, threshold) built at load time from tag_labels().
    labels: Vec<(String, String, f32)>,
    has_token_type_ids: bool,
    /// Class indices detected from config.json id2label.
    nli: NliIndices,
}

#[cfg(feature = "ai-onnx")]
impl OnnxTaggerInner {
    fn load(model_dir: &Path) -> Result<Self, TaggingError> {
        let model_path = resolve_model_path(model_dir)
            .ok_or(TaggingError::ModelNotLoaded)?;
        let tokenizer_path = model_dir.join("tokenizer.json");

        tracing::info!(path = %model_path.display(), "Loading NLI cross-encoder");

        let session = ort::session::Session::builder()
            .map_err(|e| TaggingError::Onnx(e.to_string()))?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)
            .map_err(|e| TaggingError::Onnx(e.to_string()))?
            .commit_from_file(&model_path)
            .map_err(|e| TaggingError::Onnx(e.to_string()))?;

        let has_token_type_ids = session.inputs().iter()
            .any(|i| i.name() == "token_type_ids");

        let output_names: Vec<&str> = session.outputs().iter().map(|o| o.name()).collect();
        tracing::info!(outputs = ?output_names, has_token_type_ids, "NLI model inputs/outputs");

        let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| TaggingError::Tokenizer(e.to_string()))?;

        let nli = detect_nli_indices(&model_dir.join("config.json"));
        tracing::info!(entailment = nli.entailment, contradiction = nli.contradiction, "NLI class indices");

        let labels: Vec<(String, String, f32)> = tag_labels()
            .iter()
            .filter(|l| !is_rule_only_tag(l.tag))
            .map(|l| (l.tag.to_string(), l.description.to_string(), l.threshold))
            .collect();

        tracing::info!(labels = labels.len(), "NLI tagger ready");

        Ok(Self {
            session: std::sync::Mutex::new(session),
            tokenizer,
            labels,
            has_token_type_ids,
            nli,
        })
    }

    fn classify(&self, item: &FeedItem, _feed_type: &FeedType) -> Result<Vec<TagResult>, TaggingError> {
        let title = item.title.as_str();
        let body = item.body_text.as_deref().unwrap_or("");
        // Use title only for NLI: body text dilutes scores and introduces noise.
        // Title is the clearest signal; body enrichment can be added per feed-type later.
        let text = title.to_string();

        // NLI is unreliable for very short text — model lacks enough context to distinguish
        // entailment from contradiction and produces near-random logits. Min 5 words.
        if text.split_whitespace().count() < 5 {
            return Ok(vec![]);
        }

        // Skip primarily non-ASCII text (non-English feeds): the English NLI model
        // cannot interpret non-English premises and may output biased logits.
        let non_ascii = text.chars().filter(|c| !c.is_ascii()).count();
        if non_ascii * 4 > text.len() {
            return Ok(vec![]);
        }

        // Skip transliterated Hindi/Urdu (Latin-script) which bypasses the non-ASCII
        // filter but produces wildly wrong NLI scores (e.g. "maintenance" in a Dogri
        // sentence scores 0.67 for security). Detected via unambiguous function words
        // that appear in Romanized Hindi/Urdu but almost never in English text.
        if is_transliterated_indic(&text) {
            return Ok(vec![]);
        }

        let _ = body; // body retained in struct for future per-feed-type use

        let mut session = self.session.lock()
            .map_err(|_| TaggingError::Onnx("session mutex poisoned".into()))?;

        let mut results = Vec::new();
        for (tag, hypothesis, threshold) in &self.labels {
            let prob = run_nli(
                &mut session,
                &self.tokenizer,
                &text,
                hypothesis,
                self.has_token_type_ids,
                &self.nli,
            )?;
            if prob >= *threshold {
                results.push(TagResult {
                    tag: tag.clone(),
                    confidence: prob,
                    explanation: format!("NLI entailment {:.3}", prob),
                    source: TaggerSource::Model,
                    rule_id: None,
                });
            }
        }

        Ok(results)
    }

    fn similarities(&self, text: &str) -> Result<Vec<(String, f32)>, TaggingError> {
        let mut session = self.session.lock()
            .map_err(|_| TaggingError::Onnx("session mutex poisoned".into()))?;

        let mut sims: Vec<(String, f32)> = Vec::with_capacity(self.labels.len());
        for (tag, hypothesis, _) in &self.labels {
            let prob = run_nli(
                &mut session,
                &self.tokenizer,
                text,
                hypothesis,
                self.has_token_type_ids,
                &self.nli,
            )?;
            sims.push((tag.clone(), prob));
        }
        sims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(sims)
    }
}

/// Encode `(text, hypothesis)` as an NLI pair, run the cross-encoder session, and return
/// the zero-shot classification score: `P(entailment) / (P(entailment) + P(contradiction))`.
///
/// The neutral class is excluded from normalization — this is the standard approach used by
/// HuggingFace's zero-shot classification pipeline. Including neutral suppresses entailment
/// probabilities toward zero for short/telegraphic text, making thresholds impossible to
/// calibrate. The 2-class ratio yields reliable ordinal scores across diverse content types.
///
/// Input format: `[CLS] text [SEP] hypothesis [SEP]` (tokenizer adds special tokens).
/// Output: `logits` of shape [1, 3] → entailment / (entailment + contradiction).
#[cfg(feature = "ai-onnx")]
fn run_nli(
    session: &mut ort::session::Session,
    tokenizer: &tokenizers::Tokenizer,
    text: &str,
    hypothesis: &str,
    has_token_type_ids: bool,
    nli: &NliIndices,
) -> Result<f32, TaggingError> {
    use tokenizers::{EncodeInput, InputSequence};

    let encoding = tokenizer.encode(
        EncodeInput::Dual(
            InputSequence::from(text),
            InputSequence::from(hypothesis),
        ),
        true,
    ).map_err(|e| TaggingError::Tokenizer(e.to_string()))?;

    let ids: Vec<i64> = encoding.get_ids().iter().take(MAX_SEQ_LEN).map(|&x| x as i64).collect();
    let mask: Vec<i64> = encoding.get_attention_mask().iter().take(MAX_SEQ_LEN).map(|&x| x as i64).collect();
    let seq_len = ids.len();

    let id_tensor = ort::value::TensorRef::from_array_view(([1usize, seq_len], &ids[..]))
        .map_err(|e| TaggingError::Onnx(e.to_string()))?;
    let mask_tensor = ort::value::TensorRef::from_array_view(([1usize, seq_len], &mask[..]))
        .map_err(|e| TaggingError::Onnx(e.to_string()))?;

    let outputs = if has_token_type_ids {
        // Use actual segment ids from the tokenizer (0=premise, 1=hypothesis)
        let type_ids: Vec<i64> = encoding.get_type_ids().iter().take(MAX_SEQ_LEN).map(|&x| x as i64).collect();
        let type_tensor = ort::value::TensorRef::from_array_view(([1usize, seq_len], &type_ids[..]))
            .map_err(|e| TaggingError::Onnx(e.to_string()))?;
        session.run(ort::inputs![
            "input_ids" => id_tensor,
            "attention_mask" => mask_tensor,
            "token_type_ids" => type_tensor,
        ]).map_err(|e| TaggingError::Onnx(e.to_string()))?
    } else {
        session.run(ort::inputs![
            "input_ids" => id_tensor,
            "attention_mask" => mask_tensor,
        ]).map_err(|e| TaggingError::Onnx(e.to_string()))?
    };

    let (_, logits) = outputs["logits"]
        .try_extract_tensor::<f32>()
        .map_err(|e| TaggingError::Onnx(e.to_string()))?;

    let logits: Vec<f32> = logits.iter().copied().collect();
    if logits.len() < 3 {
        return Err(TaggingError::Onnx(format!(
            "expected 3 NLI logits, got {}; check model is an NLI cross-encoder",
            logits.len()
        )));
    }

    let score = score_nli(&logits, nli);
    Ok(score)
}

/// Zero-shot NLI classification score: geometric mean of 3-class and 2-class probabilities.
///
/// Two terms combined:
/// - s3 = softmax([all logits])[entailment]  — the "safety valve": stays near 0 when the
///         model outputs high neutral probability (uncertain or non-English text), preventing
///         random-looking logits from producing false high scores.
/// - s2 = exp(entailment) / (exp(entailment) + exp(contradiction))  — the "signal amplifier":
///         boosts genuine entailment that 3-class softmax would suppress due to neutral mass.
///
/// Geometric mean (√(s3 × s2)) requires BOTH to be nonzero, which separates:
/// - Genuine entailment: s3 ≈ 0.03–0.15 and s2 ≈ 0.85–0.99 → score ≈ 0.16–0.39
/// - Uncertain/irrelevant text: s3 ≈ 0.001 and s2 ≈ 0.5–0.97 → score ≈ 0.02–0.03
/// - Clear entailment (tutorial): s3 ≈ 0.85 and s2 ≈ 0.99 → score ≈ 0.92
#[cfg(feature = "ai-onnx")]
fn score_nli(logits: &[f32], nli: &NliIndices) -> f32 {
    // 3-class softmax at entailment index
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = logits.iter().map(|x| (x - max).exp()).collect();
    let sum: f32 = exps.iter().sum();
    let s3 = exps[nli.entailment] / sum;

    // 2-class (entailment vs contradiction, ignoring neutral)
    let e = (logits[nli.entailment] as f64).exp();
    let c = (logits[nli.contradiction] as f64).exp();
    let s2 = (e / (e + c)) as f32;

    (s3 * s2).sqrt()
}

/// Prefer quantized ONNX to reduce memory footprint; fall back to fp32.
#[cfg(feature = "ai-onnx")]
fn resolve_model_path(model_dir: &Path) -> Option<PathBuf> {
    ["model_quantized.onnx", "model.onnx"]
        .iter()
        .map(|f| model_dir.join(f))
        .find(|p| p.exists())
}

/// Read `config.json` id2label and return entailment + contradiction indices.
/// Falls back to the two most common conventions if the config is missing or malformed.
#[cfg(feature = "ai-onnx")]
fn detect_nli_indices(config_path: &Path) -> NliIndices {
    let try_detect = || -> Option<NliIndices> {
        let content = std::fs::read_to_string(config_path).ok()?;
        let v: serde_json::Value = serde_json::from_str(&content).ok()?;
        let id2label = v.get("id2label")?.as_object()?;
        let mut entailment = None;
        let mut contradiction = None;
        for (k, label) in id2label {
            let idx: usize = k.parse().ok()?;
            match label.as_str()?.to_ascii_lowercase().as_str() {
                "entailment"    => entailment    = Some(idx),
                "contradiction" => contradiction = Some(idx),
                _ => {}
            }
        }
        Some(NliIndices { entailment: entailment?, contradiction: contradiction? })
    };
    // Fallback: Xenova/nli-deberta-v3 models use [contradiction=0, entailment=1, neutral=2].
    try_detect().unwrap_or(NliIndices { entailment: 1, contradiction: 0 })
}

/// Detect transliterated Hindi/Urdu written in Latin (Roman) script.
///
/// Such text bypasses the non-ASCII filter but produces nonsense NLI scores —
/// the English NLI model has no way to interpret it and produces high-confidence
/// false matches (e.g. a sentence about a power outage scoring 0.67 for "security").
///
/// Detection: whole-word match against a compact set of Hindi/Urdu function words
/// and common verbs that are unambiguous in standard English text. Single false
/// positive risk is extremely low because these tokens don't appear as standalone
/// words in English sentences.
#[cfg(feature = "ai-onnx")]
fn is_transliterated_indic(text: &str) -> bool {
    const MARKERS: &[&str] = &[
        // Hindi/Urdu grammatical particles (postpositions, conjunctions)
        "ka", "ki", "ke", "ko", "se", "mein", "hai", "hain",
        "nahi", "nahin", "koi", "kya", "aur", "lekin", "par",
        // Common pronouns / address forms
        "yaar", "yar", "bhai", "aap", "hum", "tum",
        // Frequent verb forms in Roman Hindi
        "aagyi", "aaya", "aaye", "gaya", "gayi", "karo", "karna",
        "maine", "banaya", "sunae", "logon", "shuru",
        // Everyday Hindi vocabulary that appears frequently in regional feeds
        "aaj", "abhi", "bahut", "accha", "sahi", "wala", "wali",
        "garniya", "janwari",
    ];

    // Split on non-alphanumeric boundaries so punctuation doesn't prevent a match.
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .any(|w| MARKERS.contains(&w))
}
