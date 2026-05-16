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
    /// Index of the entailment class in the 3-class softmax output.
    /// Detected from config.json; falls back to 0 (MoritzLaurer convention).
    entailment_idx: usize,
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

        let entailment_idx = detect_entailment_idx(&model_dir.join("config.json"))
            .unwrap_or(0); // MoritzLaurer NLI models: 0=entailment, 1=neutral, 2=contradiction
        tracing::info!(entailment_idx, "NLI entailment label index");

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
            entailment_idx,
        })
    }

    fn classify(&self, item: &FeedItem, _feed_type: &FeedType) -> Result<Vec<TagResult>, TaggingError> {
        let title = item.title.as_str();
        let body = item.body_text.as_deref().unwrap_or("");
        let text = if body.is_empty() {
            title.to_string()
        } else {
            format!("{}\n\n{}", title, body)
        };

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
                self.entailment_idx,
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
                self.entailment_idx,
            )?;
            sims.push((tag.clone(), prob));
        }
        sims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(sims)
    }
}

/// Encode `(text, hypothesis)` as an NLI pair, run the cross-encoder session,
/// and return the softmax entailment probability.
///
/// Input format: `[CLS] text [SEP] hypothesis [SEP]` (tokenizer adds special tokens).
/// Output: `logits` of shape [1, 3] → softmax → entailment class probability.
#[cfg(feature = "ai-onnx")]
fn run_nli(
    session: &mut ort::session::Session,
    tokenizer: &tokenizers::Tokenizer,
    text: &str,
    hypothesis: &str,
    has_token_type_ids: bool,
    entailment_idx: usize,
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

    let entailment_prob = softmax_idx(&logits, entailment_idx);
    Ok(entailment_prob)
}

/// Softmax over `logits`, returning the probability at `idx`.
#[cfg(feature = "ai-onnx")]
fn softmax_idx(logits: &[f32], idx: usize) -> f32 {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = logits.iter().map(|x| (x - max).exp()).collect();
    let sum: f32 = exps.iter().sum();
    exps[idx] / sum
}

/// Prefer quantized ONNX to reduce memory footprint; fall back to fp32.
#[cfg(feature = "ai-onnx")]
fn resolve_model_path(model_dir: &Path) -> Option<PathBuf> {
    ["model_quantized.onnx", "model.onnx"]
        .iter()
        .map(|f| model_dir.join(f))
        .find(|p| p.exists())
}

/// Read `config.json` and return the index of the "entailment" label in id2label.
#[cfg(feature = "ai-onnx")]
fn detect_entailment_idx(config_path: &Path) -> Option<usize> {
    let content = std::fs::read_to_string(config_path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    let id2label = v.get("id2label")?.as_object()?;
    for (k, label) in id2label {
        if label.as_str()?.eq_ignore_ascii_case("entailment") {
            return k.parse().ok();
        }
    }
    None
}
