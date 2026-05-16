use std::path::Path;
use crate::error::TaggingError;
use crate::types::TagResult;
#[cfg(feature = "ai-vision")]
use crate::types::TaggerSource;

#[cfg(feature = "ai-vision")]
const IMAGE_DOWNLOAD_TIMEOUT_SECS: u64 = 10;
#[cfg(feature = "ai-vision")]
const IMAGE_MAX_BYTES: usize = 8 * 1024 * 1024; // 8 MB

/// CLIP ViT-B/32 zero-shot image classifier.
///
/// Requires two files in the model directory:
/// - `vision_model_q4f16.onnx` — CLIP vision encoder (download via `pulse ai download clip-vit-b32`)
/// - `label_embeddings.bin`    — pre-computed CLIP text embeddings for our tag set
///                               (generate with `scripts/compute_clip_labels.py`)
///
/// When the `ai-vision` feature is disabled, `VisionTagger::load` always returns
/// `Err(TaggingError::ModelNotLoaded)` so callers can hold `Option<VisionTagger>`
/// without cfg guards.
pub struct VisionTagger {
    #[cfg(feature = "ai-vision")]
    inner: VisionTaggerInner,
}

impl VisionTagger {
    /// Load from a model directory. Fails if either required file is missing.
    pub fn load(model_dir: &Path) -> Result<Self, TaggingError> {
        #[cfg(feature = "ai-vision")]
        {
            let inner = VisionTaggerInner::load(model_dir)?;
            Ok(Self { inner })
        }
        #[cfg(not(feature = "ai-vision"))]
        {
            let _ = model_dir;
            Err(TaggingError::ModelNotLoaded)
        }
    }

    /// Download an image from `url`, run CLIP inference, and return tags above threshold.
    pub async fn classify_image_url(&self, url: &str) -> Result<Vec<TagResult>, TaggingError> {
        #[cfg(feature = "ai-vision")]
        {
            self.inner.classify_image_url(url).await
        }
        #[cfg(not(feature = "ai-vision"))]
        {
            let _ = url;
            Err(TaggingError::ModelNotLoaded)
        }
    }

    /// Return raw `(tag, cosine_similarity)` pairs for an image URL, ignoring thresholds.
    /// Used for calibration via `pulse ai vision-debug`.
    pub async fn similarities_url(&self, url: &str) -> Result<Vec<(String, f32)>, TaggingError> {
        #[cfg(feature = "ai-vision")]
        {
            self.inner.similarities_url(url).await
        }
        #[cfg(not(feature = "ai-vision"))]
        {
            let _ = url;
            Err(TaggingError::ModelNotLoaded)
        }
    }
}

// ── Feature-gated implementation ──────────────────────────────────────────────

#[cfg(feature = "ai-vision")]
struct VisionTaggerInner {
    session: std::sync::Mutex<ort::session::Session>,
    /// (tag, L2-normalized embedding [512], threshold)
    labels: Vec<(String, Vec<f32>, f32)>,
    http: reqwest::Client,
}

#[cfg(feature = "ai-vision")]
impl VisionTaggerInner {
    fn load(model_dir: &Path) -> Result<Self, TaggingError> {
        let onnx_path = model_dir.join("vision_model_q4f16.onnx");
        if !onnx_path.exists() {
            return Err(TaggingError::ModelNotLoaded);
        }

        let embeddings_path = model_dir.join("label_embeddings.bin");
        if !embeddings_path.exists() {
            tracing::warn!(
                path = %embeddings_path.display(),
                "label_embeddings.bin not found — run scripts/compute_clip_labels.py to generate it"
            );
            return Err(TaggingError::ModelNotLoaded);
        }

        tracing::info!(path = %onnx_path.display(), "Loading CLIP vision encoder");

        let session = ort::session::Session::builder()
            .map_err(|e| TaggingError::Onnx(e.to_string()))?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)
            .map_err(|e| TaggingError::Onnx(e.to_string()))?
            .commit_from_file(&onnx_path)
            .map_err(|e| TaggingError::Onnx(e.to_string()))?;

        let labels = load_label_embeddings(&embeddings_path)?;
        tracing::info!(labels = labels.len(), "CLIP vision tagger ready");

        let http = reqwest::Client::builder()
            .user_agent("Pulse/0.1 vision-tagger")
            .timeout(std::time::Duration::from_secs(IMAGE_DOWNLOAD_TIMEOUT_SECS))
            .build()
            .map_err(|e| TaggingError::ImageNetwork(e.to_string()))?;

        Ok(Self {
            session: std::sync::Mutex::new(session),
            labels,
            http,
        })
    }

    async fn fetch_image(&self, url: &str) -> Result<Vec<u8>, TaggingError> {
        let resp = self.http.get(url).send().await
            .map_err(|e| TaggingError::ImageNetwork(format!("fetch {url}: {e}")))?;

        if !resp.status().is_success() {
            return Err(TaggingError::ImageNetwork(
                format!("HTTP {} for {url}", resp.status())
            ));
        }

        let content_type = resp.headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_lowercase();

        // Only process image content types
        if !content_type.starts_with("image/") && !content_type.is_empty() {
            return Err(TaggingError::ImageDecode(
                format!("not an image content-type: {content_type}")
            ));
        }

        let bytes = resp.bytes().await
            .map_err(|e| TaggingError::ImageNetwork(format!("read {url}: {e}")))?;

        if bytes.len() > IMAGE_MAX_BYTES {
            return Err(TaggingError::ImageDecode(
                format!("image too large ({} bytes > {} limit)", bytes.len(), IMAGE_MAX_BYTES)
            ));
        }

        Ok(bytes.to_vec())
    }

    fn embed_image(&self, image_bytes: &[u8]) -> Result<Vec<f32>, TaggingError> {
        let pixel_values = preprocess_image(image_bytes)?;

        let mut session = self.session.lock()
            .map_err(|_| TaggingError::Onnx("vision session mutex poisoned".into()))?;

        let tensor = ort::value::TensorRef::from_array_view(([1usize, 3, 224, 224], &pixel_values[..]))
            .map_err(|e| TaggingError::Onnx(e.to_string()))?;

        let outputs = session.run(ort::inputs!["pixel_values" => tensor])
            .map_err(|e| TaggingError::Onnx(e.to_string()))?;

        let (_, embeds) = outputs["image_embeds"]
            .try_extract_tensor::<f32>()
            .map_err(|e| TaggingError::Onnx(format!("extract image_embeds: {e}")))?;

        let embedding: Vec<f32> = embeds.iter().copied().collect();
        Ok(l2_normalize(embedding))
    }

    async fn classify_image_url(&self, url: &str) -> Result<Vec<TagResult>, TaggingError> {
        let bytes = self.fetch_image(url).await?;
        let image_embed = self.embed_image(&bytes)?;

        let mut results = Vec::new();
        for (tag, label_embed, threshold) in &self.labels {
            let score = cosine_sim(&image_embed, label_embed);
            if score >= *threshold {
                results.push(TagResult {
                    tag: tag.clone(),
                    confidence: score,
                    explanation: format!("CLIP cosine {:.3}", score),
                    source: TaggerSource::Model,
                    rule_id: None,
                });
            }
        }

        Ok(results)
    }

    async fn similarities_url(&self, url: &str) -> Result<Vec<(String, f32)>, TaggingError> {
        let bytes = self.fetch_image(url).await?;
        let image_embed = self.embed_image(&bytes)?;

        let mut sims: Vec<(String, f32)> = self.labels.iter()
            .map(|(tag, label_embed, _)| (tag.clone(), cosine_sim(&image_embed, label_embed)))
            .collect();

        sims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(sims)
    }
}

// ── Image preprocessing ───────────────────────────────────────────────────────

/// CLIP normalization constants (ImageNet-derived, used by OpenAI CLIP)
#[cfg(feature = "ai-vision")]
const CLIP_MEAN: [f32; 3] = [0.48145466, 0.4578275, 0.40821073];
#[cfg(feature = "ai-vision")]
const CLIP_STD: [f32; 3]  = [0.26862954, 0.26130258, 0.27577711];

/// Decode, resize to 224×224, normalize, and return a flat NCHW [1, 3, 224, 224] tensor.
#[cfg(feature = "ai-vision")]
fn preprocess_image(bytes: &[u8]) -> Result<Vec<f32>, TaggingError> {
    use image::imageops::FilterType;

    let img = image::load_from_memory(bytes)
        .map_err(|e| TaggingError::ImageDecode(e.to_string()))?;

    let img = img.resize_exact(224, 224, FilterType::Lanczos3);
    let rgb = img.to_rgb8();

    let mut tensor = vec![0f32; 3 * 224 * 224];
    for (y, row) in rgb.rows().enumerate() {
        for (x, pixel) in row.enumerate() {
            let r = pixel[0] as f32 / 255.0;
            let g = pixel[1] as f32 / 255.0;
            let b = pixel[2] as f32 / 255.0;
            let base = y * 224 + x;
            tensor[base]                  = (r - CLIP_MEAN[0]) / CLIP_STD[0];
            tensor[224 * 224 + base]      = (g - CLIP_MEAN[1]) / CLIP_STD[1];
            tensor[2 * 224 * 224 + base]  = (b - CLIP_MEAN[2]) / CLIP_STD[2];
        }
    }

    Ok(tensor)
}

// ── Label embedding I/O ───────────────────────────────────────────────────────

/// Binary file format: 4-byte magic "VLAB", u32 LE num_labels, u32 LE emb_dim,
/// then num_labels × emb_dim f32 LE values (row-major, L2-normalized).
#[cfg(feature = "ai-vision")]
fn load_label_embeddings(path: &Path) -> Result<Vec<(String, Vec<f32>, f32)>, TaggingError> {
    use super::vision_labels::vision_labels;
    use std::io::Read;

    let data = std::fs::read(path)
        .map_err(|e| TaggingError::Onnx(format!("read label_embeddings.bin: {e}")))?;

    if data.len() < 12 {
        return Err(TaggingError::Onnx("label_embeddings.bin too short".into()));
    }
    if &data[0..4] != b"VLAB" {
        return Err(TaggingError::Onnx("label_embeddings.bin has invalid magic bytes".into()));
    }

    let num_labels = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
    let emb_dim    = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
    let expected_bytes = 12 + num_labels * emb_dim * 4;

    if data.len() != expected_bytes {
        return Err(TaggingError::Onnx(format!(
            "label_embeddings.bin size mismatch: expected {expected_bytes}, got {}", data.len()
        )));
    }

    let labels = vision_labels();
    if num_labels != labels.len() {
        return Err(TaggingError::Onnx(format!(
            "label_embeddings.bin has {num_labels} labels but vision_labels() defines {}",
            labels.len()
        )));
    }

    let mut result = Vec::with_capacity(num_labels);
    let mut cursor = &data[12..];

    for label in labels {
        let mut embedding = vec![0f32; emb_dim];
        for v in embedding.iter_mut() {
            let mut buf = [0u8; 4];
            cursor.read_exact(&mut buf)
                .map_err(|e| TaggingError::Onnx(format!("read embedding value: {e}")))?;
            *v = f32::from_le_bytes(buf);
        }
        result.push((label.tag.to_string(), embedding, label.threshold));
    }

    Ok(result)
}

// ── Math utilities ────────────────────────────────────────────────────────────

#[cfg(feature = "ai-vision")]
fn l2_normalize(mut v: Vec<f32>) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-8 {
        for x in v.iter_mut() { *x /= norm; }
    }
    v
}

#[cfg(feature = "ai-vision")]
fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    // Assumes both are already L2-normalized, so dot product = cosine similarity
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}
