use crate::error::TaggingError;
use crate::types::TagResult;
#[cfg(feature = "ai-miniml")]
use crate::types::TaggerSource;
use std::path::Path;

/// MiniLM sentence embedding (ONNX) + custom MLP classifier head (PMLP binary format).
///
/// Used as the semantic classifier for categories like research, discussion, clickbait.
/// Runs all-MiniLM-L6-v2 to produce 384-dim sentence embeddings, then applies a
/// two-layer MLP head loaded from a `mlp_head.pmlp` file.
///
/// When the `ai-miniml` feature is disabled, `MiniMlTagger::load` always returns
/// `Err(TaggingError::ModelNotLoaded)` so callers can hold `Option<MiniMlTagger>`
/// without cfg guards.
pub struct MiniMlTagger {
    #[cfg(feature = "ai-miniml")]
    inner: MiniMlTaggerInner,
}

impl MiniMlTagger {
    /// Load a MiniLM+MLP classifier from a model directory containing:
    /// - `model.onnx` or `model_quantized.onnx`   (all-MiniLM-L6-v2)
    /// - `tokenizer.json`
    /// - `mlp_head.pmlp`                           (trained MLP weights, PMLP binary format)
    /// - `miniml_thresholds.json`                  (optional; default threshold 0.5)
    pub fn load(model_dir: &Path) -> Result<Self, TaggingError> {
        #[cfg(feature = "ai-miniml")]
        {
            let inner = MiniMlTaggerInner::load(model_dir)?;
            Ok(Self { inner })
        }
        #[cfg(not(feature = "ai-miniml"))]
        {
            let _ = model_dir;
            Err(TaggingError::ModelNotLoaded)
        }
    }

    /// Classify text and return tags whose MLP probability meets the per-label threshold.
    pub fn classify(&self, text: &str) -> Result<Vec<TagResult>, TaggingError> {
        #[cfg(feature = "ai-miniml")]
        {
            self.inner.classify(text)
        }
        #[cfg(not(feature = "ai-miniml"))]
        {
            let _ = text;
            Err(TaggingError::ModelNotLoaded)
        }
    }

    /// Return `(label, probability)` pairs sorted descending by probability.
    /// Ignores thresholds — intended for calibration and debug tooling.
    pub fn scores(&self, text: &str) -> Result<Vec<(String, f32)>, TaggingError> {
        #[cfg(feature = "ai-miniml")]
        {
            self.inner.scores(text)
        }
        #[cfg(not(feature = "ai-miniml"))]
        {
            let _ = text;
            Err(TaggingError::ModelNotLoaded)
        }
    }

    /// Produce the raw 384-dim MiniLM sentence embedding for `text`.
    /// Useful for downstream similarity search or clustering.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, TaggingError> {
        #[cfg(feature = "ai-miniml")]
        {
            self.inner.embed(text)
        }
        #[cfg(not(feature = "ai-miniml"))]
        {
            let _ = text;
            Err(TaggingError::ModelNotLoaded)
        }
    }
}

// ── Feature-gated implementation ──────────────────────────────────────────────

#[cfg(feature = "ai-miniml")]
struct MiniMlTaggerInner {
    session: std::sync::Mutex<ort::session::Session>,
    tokenizer: tokenizers::Tokenizer,
    /// Whether the ONNX model expects `token_type_ids` as an input.
    has_token_type_ids: bool,
    /// Label strings in the same order as the MLP output neurons.
    labels: Vec<String>,
    /// Per-label classification thresholds (same length as `labels`).
    thresholds: Vec<f32>,
    // MLP weight tensors (row-major, f32)
    w1: Vec<f32>, // [hidden_dim × input_dim]
    b1: Vec<f32>, // [hidden_dim]
    w2: Vec<f32>, // [num_labels × hidden_dim]
    b2: Vec<f32>, // [num_labels]
    input_dim: usize,
    hidden_dim: usize,
}

/// Maximum sequence length fed to the ONNX model.
#[cfg(feature = "ai-miniml")]
const MAX_SEQ_LEN: usize = 128;

#[cfg(feature = "ai-miniml")]
impl MiniMlTaggerInner {
    fn load(model_dir: &Path) -> Result<Self, TaggingError> {
        // 1. Resolve ONNX model path: prefer model.onnx, fall back to quantized.
        let model_path = resolve_model_path(model_dir).ok_or(TaggingError::ModelNotLoaded)?;
        let tokenizer_path = model_dir.join("tokenizer.json");
        let pmlp_path = model_dir.join("mlp_head.pmlp");
        let thresholds_path = model_dir.join("miniml_thresholds.json");

        tracing::info!(path = %model_path.display(), "Loading MiniLM ONNX session");

        // 2. Load ONNX session with Level3 graph optimization.
        let session = ort::session::Session::builder()
            .map_err(|e| TaggingError::Onnx(e.to_string()))?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)
            .map_err(|e| TaggingError::Onnx(e.to_string()))?
            .commit_from_file(&model_path)
            .map_err(|e| TaggingError::Onnx(e.to_string()))?;

        // 3. Detect optional `token_type_ids` input.
        let has_token_type_ids = session
            .inputs()
            .iter()
            .any(|i| i.name() == "token_type_ids");

        let output_names: Vec<&str> = session.outputs().iter().map(|o| o.name()).collect();
        tracing::info!(
            outputs = ?output_names,
            has_token_type_ids,
            "MiniLM model inputs/outputs"
        );

        // 4. Load tokenizer.
        let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| TaggingError::Tokenizer(e.to_string()))?;

        // 5. Parse PMLP binary.
        let (labels, w1, b1, w2, b2, input_dim, hidden_dim) = load_pmlp(&pmlp_path)?;
        let num_labels = labels.len();

        tracing::info!(
            labels = num_labels,
            input_dim,
            hidden_dim,
            "MiniLM MLP head loaded"
        );

        // 6. Load per-label thresholds (default 0.5 if file absent or label missing).
        let thresholds = load_thresholds(&thresholds_path, &labels);

        Ok(Self {
            session: std::sync::Mutex::new(session),
            tokenizer,
            has_token_type_ids,
            labels,
            thresholds,
            w1,
            b1,
            w2,
            b2,
            input_dim,
            hidden_dim,
        })
    }

    fn classify(&self, text: &str) -> Result<Vec<TagResult>, TaggingError> {
        let embedding = self.embed(text)?;
        let probs = self.mlp_forward(&embedding);

        let mut results = Vec::new();
        for (i, prob) in probs.iter().enumerate() {
            if *prob >= self.thresholds[i] {
                results.push(TagResult {
                    tag: self.labels[i].clone(),
                    confidence: *prob,
                    explanation: format!("MiniLM {:.3}", prob),
                    source: TaggerSource::Model,
                    rule_id: None,
                });
            }
        }
        Ok(results)
    }

    fn scores(&self, text: &str) -> Result<Vec<(String, f32)>, TaggingError> {
        let embedding = self.embed(text)?;
        let probs = self.mlp_forward(&embedding);

        let mut pairs: Vec<(String, f32)> = self.labels.iter().cloned().zip(probs).collect();
        pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(pairs)
    }

    /// Tokenize `text`, run the MiniLM ONNX session, and return a 384-dim
    /// mean-pooled sentence embedding.
    fn embed(&self, text: &str) -> Result<Vec<f32>, TaggingError> {
        // Tokenize as a single sequence (no pair).
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| TaggingError::Tokenizer(e.to_string()))?;

        let ids: Vec<i64> = encoding
            .get_ids()
            .iter()
            .take(MAX_SEQ_LEN)
            .map(|&x| x as i64)
            .collect();
        let mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .take(MAX_SEQ_LEN)
            .map(|&x| x as i64)
            .collect();
        let seq_len = ids.len();

        let id_tensor = ort::value::TensorRef::from_array_view(([1usize, seq_len], &ids[..]))
            .map_err(|e| TaggingError::Onnx(e.to_string()))?;
        let mask_tensor = ort::value::TensorRef::from_array_view(([1usize, seq_len], &mask[..]))
            .map_err(|e| TaggingError::Onnx(e.to_string()))?;

        let mut session = self
            .session
            .lock()
            .map_err(|_| TaggingError::Onnx("session mutex poisoned".into()))?;

        let outputs = if self.has_token_type_ids {
            let type_ids: Vec<i64> = vec![0i64; seq_len];
            let type_tensor =
                ort::value::TensorRef::from_array_view(([1usize, seq_len], &type_ids[..]))
                    .map_err(|e| TaggingError::Onnx(e.to_string()))?;
            session
                .run(ort::inputs![
                    "input_ids"      => id_tensor,
                    "attention_mask" => mask_tensor,
                    "token_type_ids" => type_tensor,
                ])
                .map_err(|e| TaggingError::Onnx(e.to_string()))?
        } else {
            session
                .run(ort::inputs![
                    "input_ids"      => id_tensor,
                    "attention_mask" => mask_tensor,
                ])
                .map_err(|e| TaggingError::Onnx(e.to_string()))?
        };

        // Extract last_hidden_state: shape [1, seq_len, 384].
        let (_, tensor_data) = outputs["last_hidden_state"]
            .try_extract_tensor::<f32>()
            .map_err(|e| TaggingError::Onnx(e.to_string()))?;
        let hidden: Vec<f32> = tensor_data.to_vec();

        // hidden has seq_len * embed_dim elements.
        // Infer embed_dim from the total size.
        let embed_dim = hidden.len().checked_div(seq_len).unwrap_or(0);
        if embed_dim == 0 {
            return Err(TaggingError::Onnx(
                "last_hidden_state has zero elements".into(),
            ));
        }

        // Mean pooling over attended positions (mask[i] == 1).
        let mut embedding = vec![0.0f32; embed_dim];
        let mut count = 0u32;
        for (i, &m) in mask.iter().take(seq_len).enumerate() {
            if m == 1 {
                let offset = i * embed_dim;
                for j in 0..embed_dim {
                    embedding[j] += hidden[offset + j];
                }
                count += 1;
            }
        }
        if count > 0 {
            let inv = 1.0 / count as f32;
            for v in &mut embedding {
                *v *= inv;
            }
        }

        Ok(embedding)
    }

    /// Two-layer MLP forward pass with ReLU hidden activation and sigmoid output.
    ///
    /// Architecture:
    ///   h = relu(W1 · embedding + b1)       shape: [hidden_dim]
    ///   p = sigmoid(W2 · h + b2)            shape: [num_labels]
    fn mlp_forward(&self, embedding: &[f32]) -> Vec<f32> {
        let hidden_dim = self.hidden_dim;
        let num_labels = self.labels.len();

        // Layer 1: h[i] = relu(dot(W1[i], embedding) + b1[i])
        let mut h = Vec::with_capacity(hidden_dim);
        for i in 0..hidden_dim {
            let row = &self.w1[i * self.input_dim..(i + 1) * self.input_dim];
            let dot: f32 = row.iter().zip(embedding.iter()).map(|(w, x)| w * x).sum();
            h.push(f32::max(0.0, dot + self.b1[i]));
        }

        // Layer 2: logit[j] = dot(W2[j], h) + b2[j], then sigmoid.
        let mut probs = Vec::with_capacity(num_labels);
        for j in 0..num_labels {
            let row = &self.w2[j * hidden_dim..(j + 1) * hidden_dim];
            let logit: f32 = row.iter().zip(h.iter()).map(|(w, x)| w * x).sum::<f32>() + self.b2[j];
            probs.push(1.0 / (1.0 + (-logit).exp()));
        }

        probs
    }
}

// ── PMLP binary parser ────────────────────────────────────────────────────────

/// Parse a `mlp_head.pmlp` file and return
/// `(labels, w1, b1, w2, b2, input_dim, hidden_dim)`.
///
/// PMLP binary layout (all integers little-endian):
/// ```text
/// Header (16 bytes):
///   [0..4]   magic: b"PMLP"
///   [4..8]   version: u32 = 1
///   [8..12]  input_dim: u32
///   [12..16] num_labels: u32
///
/// Label table (num_labels entries):
///   [len: u16][utf-8 bytes × len]
///
/// Layer 1:
///   hidden_dim: u32
///   W1: hidden_dim × input_dim × f32  (row-major)
///   b1: hidden_dim × f32
///
/// Layer 2:
///   W2: num_labels × hidden_dim × f32 (row-major)
///   b2: num_labels × f32
/// ```
#[cfg(feature = "ai-miniml")]
fn load_pmlp(
    path: &Path,
) -> Result<
    (
        Vec<String>,
        Vec<f32>,
        Vec<f32>,
        Vec<f32>,
        Vec<f32>,
        usize,
        usize,
    ),
    TaggingError,
> {
    use std::io::Read;

    let mut f = std::fs::File::open(path)
        .map_err(|e| TaggingError::Onnx(format!("cannot open mlp_head.pmlp: {e}")))?;

    // ── Header ────────────────────────────────────────────────────────────────
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic)
        .map_err(|e| TaggingError::Onnx(format!("pmlp read magic: {e}")))?;
    if &magic != b"PMLP" {
        return Err(TaggingError::Onnx(format!(
            "pmlp bad magic: expected b\"PMLP\", got {:?}",
            magic
        )));
    }

    let version = read_u32_le(&mut f)?;
    if version != 1 {
        return Err(TaggingError::Onnx(format!(
            "pmlp unsupported version: {version}"
        )));
    }

    let input_dim = read_u32_le(&mut f)? as usize;
    let num_labels = read_u32_le(&mut f)? as usize;

    // ── Label table ───────────────────────────────────────────────────────────
    let mut labels = Vec::with_capacity(num_labels);
    for _ in 0..num_labels {
        let len = read_u16_le(&mut f)? as usize;
        let mut buf = vec![0u8; len];
        f.read_exact(&mut buf)
            .map_err(|e| TaggingError::Onnx(format!("pmlp read label bytes: {e}")))?;
        let label = String::from_utf8(buf)
            .map_err(|e| TaggingError::Onnx(format!("pmlp label utf-8: {e}")))?;
        labels.push(label);
    }

    // ── Layer 1 ───────────────────────────────────────────────────────────────
    let hidden_dim = read_u32_le(&mut f)? as usize;

    let w1 = read_f32s(&mut f, hidden_dim * input_dim, "W1")?;
    let b1 = read_f32s(&mut f, hidden_dim, "b1")?;

    // ── Layer 2 ───────────────────────────────────────────────────────────────
    let w2 = read_f32s(&mut f, num_labels * hidden_dim, "W2")?;
    let b2 = read_f32s(&mut f, num_labels, "b2")?;

    Ok((labels, w1, b1, w2, b2, input_dim, hidden_dim))
}

/// Read a little-endian u32 from `r`.
#[cfg(feature = "ai-miniml")]
fn read_u32_le(r: &mut impl std::io::Read) -> Result<u32, TaggingError> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)
        .map_err(|e| TaggingError::Onnx(format!("pmlp read u32: {e}")))?;
    Ok(u32::from_le_bytes(buf))
}

/// Read a little-endian u16 from `r`.
#[cfg(feature = "ai-miniml")]
fn read_u16_le(r: &mut impl std::io::Read) -> Result<u16, TaggingError> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)
        .map_err(|e| TaggingError::Onnx(format!("pmlp read u16: {e}")))?;
    Ok(u16::from_le_bytes(buf))
}

/// Read `n` little-endian f32s from `r` into a Vec.
#[cfg(feature = "ai-miniml")]
fn read_f32s(
    r: &mut impl std::io::Read,
    n: usize,
    field: &'static str,
) -> Result<Vec<f32>, TaggingError> {
    let mut buf = vec![0u8; n * 4];
    r.read_exact(&mut buf)
        .map_err(|e| TaggingError::Onnx(format!("pmlp read {field}: {e}")))?;
    Ok(buf
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
}

// ── Model path resolution ─────────────────────────────────────────────────────

/// Prefer `model.onnx`; fall back to `model_quantized.onnx`.
#[cfg(feature = "ai-miniml")]
fn resolve_model_path(model_dir: &Path) -> Option<std::path::PathBuf> {
    ["model.onnx", "model_quantized.onnx"]
        .iter()
        .map(|f| model_dir.join(f))
        .find(|p| p.exists())
}

// ── Threshold loader ──────────────────────────────────────────────────────────

/// Load per-label thresholds from `miniml_thresholds.json`.
///
/// Expected format (same as fasttext_thresholds.json):
/// ```json
/// { "research": 0.6, "discussion": 0.55 }
/// ```
/// Any label not present in the JSON, or if the file is absent/malformed,
/// defaults to 0.5.
#[cfg(feature = "ai-miniml")]
fn load_thresholds(path: &Path, labels: &[String]) -> Vec<f32> {
    let try_load = || -> Option<serde_json::Map<String, serde_json::Value>> {
        let content = std::fs::read_to_string(path).ok()?;
        let v: serde_json::Value = serde_json::from_str(&content).ok()?;
        v.as_object().cloned()
    };

    let map = try_load().unwrap_or_default();

    labels
        .iter()
        .map(|label| {
            map.get(label)
                .and_then(|v| v.as_f64())
                .map(|f| f as f32)
                .unwrap_or(0.5)
        })
        .collect()
}
