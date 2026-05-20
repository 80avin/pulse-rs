use std::path::Path;
use crate::error::TaggingError;
use crate::types::TagResult;
#[cfg(feature = "ai-fasttext")]
use crate::types::TaggerSource;

/// Pure-Rust FastText inference from the PFTM binary format.
///
/// The model is trained in Python and exported to PFTM via the companion
/// training script. No C++ FFI is used; inference is pure-Rust over std.
///
/// Architecture: bag-of-char-ngrams (no word vocabulary). Each token is padded
/// as `<token>`, all char byte n-grams of length [min_ngram..=max_ngram] are
/// hashed into a shared bucket table via FNV-1a, and the corresponding embedding
/// rows are averaged. The averaged embedding is projected through an output weight
/// matrix and bias, then sigmoid-activated per label (multilabel classification).
///
/// When the `ai-fasttext` feature is disabled, `FastTextTagger::load` always
/// returns `Err(TaggingError::ModelNotLoaded)` so callers can hold
/// `Option<FastTextTagger>` without cfg guards.
pub struct FastTextTagger {
    #[cfg(feature = "ai-fasttext")]
    inner: FastTextTaggerInner,
}

impl FastTextTagger {
    /// Load a FastText tagger from a model directory containing:
    /// - `fasttext.pftm`       — PFTM binary model (required)
    /// - `fasttext_thresholds.json` — per-label confidence thresholds (optional, default 0.5)
    pub fn load(model_dir: &Path) -> Result<Self, TaggingError> {
        #[cfg(feature = "ai-fasttext")]
        {
            let inner = FastTextTaggerInner::load(model_dir)?;
            Ok(Self { inner })
        }
        #[cfg(not(feature = "ai-fasttext"))]
        {
            let _ = model_dir;
            Err(TaggingError::ModelNotLoaded)
        }
    }

    /// Classify `text` and return all labels whose sigmoid probability meets
    /// their per-label threshold.
    pub fn classify(&self, text: &str) -> Result<Vec<TagResult>, TaggingError> {
        #[cfg(feature = "ai-fasttext")]
        {
            self.inner.classify(text)
        }
        #[cfg(not(feature = "ai-fasttext"))]
        {
            let _ = text;
            Err(TaggingError::ModelNotLoaded)
        }
    }

    /// Return all `(label, probability)` pairs sorted descending by probability.
    /// Useful for calibration and debugging — thresholds are not applied.
    pub fn scores(&self, text: &str) -> Result<Vec<(String, f32)>, TaggingError> {
        #[cfg(feature = "ai-fasttext")]
        {
            self.inner.scores(text)
        }
        #[cfg(not(feature = "ai-fasttext"))]
        {
            let _ = text;
            Err(TaggingError::ModelNotLoaded)
        }
    }
}

// ── Feature-gated implementation ──────────────────────────────────────────────

#[cfg(feature = "ai-fasttext")]
struct FastTextTaggerInner {
    /// Flat row-major char n-gram embedding matrix: num_buckets × embedding_dim.
    embeddings: Vec<f32>,
    /// Flat row-major output weight matrix: num_labels × embedding_dim.
    output_weights: Vec<f32>,
    /// Output bias: num_labels.
    output_bias: Vec<f32>,
    /// Label names in model-file order.
    labels: Vec<String>,
    /// Per-label confidence threshold, same order as `labels`.
    thresholds: Vec<f32>,
    embedding_dim: usize,
    num_buckets: usize,
    min_ngram: usize,
    max_ngram: usize,
}

#[cfg(feature = "ai-fasttext")]
impl FastTextTaggerInner {
    fn load(model_dir: &Path) -> Result<Self, TaggingError> {
        use std::io::Read;

        let model_path = model_dir.join("fasttext.pftm");
        let mut f = std::fs::File::open(&model_path)
            .map_err(|e| TaggingError::Onnx(format!("cannot open fasttext.pftm: {e}")))?;

        // ── Header (32 bytes) ─────────────────────────────────────────────────

        let mut magic = [0u8; 4];
        f.read_exact(&mut magic)
            .map_err(|e| TaggingError::Onnx(format!("read magic: {e}")))?;
        if &magic != b"PFTM" {
            return Err(TaggingError::Onnx(format!(
                "invalid PFTM magic: {:?}",
                magic
            )));
        }

        let version = read_u32_le(&mut f)?;
        if version != 1 {
            return Err(TaggingError::Onnx(format!(
                "unsupported PFTM version {version}; expected 1"
            )));
        }

        let num_labels     = read_u32_le(&mut f)? as usize;
        let embedding_dim  = read_u32_le(&mut f)? as usize;
        let num_buckets    = read_u32_le(&mut f)? as usize;
        let min_ngram      = read_u32_le(&mut f)? as usize;
        let max_ngram      = read_u32_le(&mut f)? as usize;
        let _reserved      = read_u32_le(&mut f)?; // must skip the 4 reserved bytes

        // Sanity-check header values to produce readable errors instead of
        // runaway allocations on corrupt files.
        if embedding_dim == 0 {
            return Err(TaggingError::Onnx("embedding_dim is 0".into()));
        }
        if num_buckets == 0 {
            return Err(TaggingError::Onnx("num_buckets is 0".into()));
        }
        if min_ngram == 0 || max_ngram < min_ngram {
            return Err(TaggingError::Onnx(format!(
                "invalid ngram range [{min_ngram}, {max_ngram}]"
            )));
        }

        // ── Label table ───────────────────────────────────────────────────────

        let mut labels = Vec::with_capacity(num_labels);
        for i in 0..num_labels {
            let len = read_u16_le(&mut f)? as usize;
            let mut buf = vec![0u8; len];
            f.read_exact(&mut buf)
                .map_err(|e| TaggingError::Onnx(format!("read label {i} bytes: {e}")))?;
            let label = String::from_utf8(buf)
                .map_err(|e| TaggingError::Onnx(format!("label {i} is not valid UTF-8: {e}")))?;
            labels.push(label);
        }

        // ── Embedding matrix ──────────────────────────────────────────────────

        let embeddings = read_f32_vec(&mut f, num_buckets * embedding_dim, "embeddings")?;

        // ── Output weight matrix ──────────────────────────────────────────────

        let output_weights = read_f32_vec(&mut f, num_labels * embedding_dim, "output_weights")?;

        // ── Output bias ───────────────────────────────────────────────────────

        let output_bias = read_f32_vec(&mut f, num_labels, "output_bias")?;

        // ── Per-label thresholds ──────────────────────────────────────────────

        let thresholds = load_thresholds(model_dir, &labels);

        tracing::info!(
            labels = num_labels,
            embedding_dim,
            num_buckets,
            min_ngram,
            max_ngram,
            "FastText PFTM model loaded"
        );

        Ok(Self {
            embeddings,
            output_weights,
            output_bias,
            labels,
            thresholds,
            embedding_dim,
            num_buckets,
            min_ngram,
            max_ngram,
        })
    }

    fn classify(&self, text: &str) -> Result<Vec<TagResult>, TaggingError> {
        let probs = self.infer(text);
        let mut results = Vec::new();
        for (i, prob) in probs.iter().enumerate() {
            if *prob >= self.thresholds[i] {
                results.push(TagResult {
                    tag: self.labels[i].clone(),
                    confidence: *prob,
                    explanation: format!("FastText {:.3}", prob),
                    source: TaggerSource::Model,
                    rule_id: None,
                });
            }
        }
        Ok(results)
    }

    fn scores(&self, text: &str) -> Result<Vec<(String, f32)>, TaggingError> {
        let probs = self.infer(text);
        let mut out: Vec<(String, f32)> = self
            .labels
            .iter()
            .zip(probs.iter())
            .map(|(label, &prob)| (label.clone(), prob))
            .collect();
        out.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(out)
    }

    /// Core inference: bag-of-char-ngrams → averaged embedding → sigmoid logits.
    fn infer(&self, text: &str) -> Vec<f32> {
        let lowercased = text.to_lowercase();

        // Tokenize: split on whitespace and ASCII punctuation except '-' and '_'.
        let tokens: Vec<&str> = lowercased
            .split(|c: char| c.is_whitespace() || (c.is_ascii_punctuation() && c != '-' && c != '_'))
            .filter(|t| !t.is_empty())
            .collect();

        let dim = self.embedding_dim;
        let mut sum = vec![0.0f32; dim];
        let mut total_ngrams: usize = 0;

        for token in &tokens {
            let indices = extract_ngram_indices(
                token,
                self.min_ngram,
                self.max_ngram,
                self.num_buckets,
            );
            for bucket in indices {
                let row = &self.embeddings[bucket * dim..(bucket + 1) * dim];
                for (s, &v) in sum.iter_mut().zip(row.iter()) {
                    *s += v;
                }
                total_ngrams += 1;
            }
        }

        // Average the accumulated embedding.
        if total_ngrams > 0 {
            let scale = 1.0 / total_ngrams as f32;
            for s in &mut sum {
                *s *= scale;
            }
        }

        // Project through output weight matrix and bias, then apply sigmoid.
        let num_labels = self.labels.len();
        let mut probs = Vec::with_capacity(num_labels);
        for i in 0..num_labels {
            let w = &self.output_weights[i * dim..(i + 1) * dim];
            let dot: f32 = w.iter().zip(sum.iter()).map(|(&a, &b)| a * b).sum();
            let logit = dot + self.output_bias[i];
            probs.push(sigmoid(logit));
        }

        probs
    }
}

// ── Helpers (feature-gated) ───────────────────────────────────────────────────

/// FNV-1a 32-bit hash.
#[cfg(feature = "ai-fasttext")]
fn fnv1a_hash(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 2166136261;
    for &b in bytes {
        hash ^= b as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

/// Pad `token` as `<token>` then extract all byte-level char n-gram bucket
/// indices for lengths in `[min_ngram, max_ngram]`.
#[cfg(feature = "ai-fasttext")]
fn extract_ngram_indices(
    token: &str,
    min_ngram: usize,
    max_ngram: usize,
    num_buckets: usize,
) -> Vec<usize> {
    let padded = format!("<{}>", token);
    let bytes = padded.as_bytes();
    let len = bytes.len();

    // Pre-allocate a reasonable upper bound.
    let mut indices = Vec::new();

    for start in 0..len {
        for ngram_len in min_ngram..=max_ngram {
            if start + ngram_len <= len {
                let bucket = fnv1a_hash(&bytes[start..start + ngram_len]) as usize % num_buckets;
                indices.push(bucket);
            }
        }
    }

    indices
}

/// Sigmoid activation.
#[cfg(feature = "ai-fasttext")]
#[inline]
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Read a single little-endian u32 from a reader.
#[cfg(feature = "ai-fasttext")]
fn read_u32_le(r: &mut impl std::io::Read) -> Result<u32, TaggingError> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)
        .map_err(|e| TaggingError::Onnx(format!("read u32: {e}")))?;
    Ok(u32::from_le_bytes(buf))
}

/// Read a single little-endian u16 from a reader.
#[cfg(feature = "ai-fasttext")]
fn read_u16_le(r: &mut impl std::io::Read) -> Result<u16, TaggingError> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)
        .map_err(|e| TaggingError::Onnx(format!("read u16: {e}")))?;
    Ok(u16::from_le_bytes(buf))
}

/// Read `count` little-endian f32 values from a reader into a `Vec<f32>`.
/// Uses a loop with `read_exact` — no unsafe code.
#[cfg(feature = "ai-fasttext")]
fn read_f32_vec(
    r: &mut impl std::io::Read,
    count: usize,
    label: &str,
) -> Result<Vec<f32>, TaggingError> {
    let mut buf = [0u8; 4];
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        r.read_exact(&mut buf).map_err(|e| {
            TaggingError::Onnx(format!("read {label}[{i}]: {e}"))
        })?;
        out.push(f32::from_le_bytes(buf));
    }
    Ok(out)
}

/// Load per-label thresholds from `model_dir/fasttext_thresholds.json`.
///
/// The JSON is expected to be a flat object mapping label name → f32 threshold.
/// Labels not present in the file default to 0.5. If the file is absent or
/// cannot be parsed the entire threshold vector defaults to 0.5.
#[cfg(feature = "ai-fasttext")]
fn load_thresholds(model_dir: &Path, labels: &[String]) -> Vec<f32> {
    let path = model_dir.join("fasttext_thresholds.json");

    let try_load = || -> Option<std::collections::HashMap<String, f32>> {
        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    };

    match try_load() {
        Some(map) => {
            labels
                .iter()
                .map(|label| map.get(label.as_str()).copied().unwrap_or(0.5))
                .collect()
        }
        None => {
            if path.exists() {
                tracing::warn!(
                    path = %path.display(),
                    "fasttext_thresholds.json could not be parsed; defaulting all thresholds to 0.5"
                );
            }
            vec![0.5; labels.len()]
        }
    }
}
