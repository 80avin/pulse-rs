use crate::error::TaggingError;
use crate::types::TagResult;
#[cfg(feature = "ai-vision")]
use crate::types::TaggerSource;
use std::path::Path;

#[cfg(feature = "ai-vision")]
const IMAGE_DOWNLOAD_TIMEOUT_SECS: u64 = 10;
#[cfg(feature = "ai-vision")]
const IMAGE_MAX_BYTES: usize = 8 * 1024 * 1024; // 8 MB

// CLIP ViT-B/32 ImageNet normalization constants (used when do_normalize=true).
// These are the exact values from the OpenAI CLIP paper; precision is intentional.
#[cfg(feature = "ai-vision")]
#[allow(clippy::excessive_precision)]
const CLIP_MEAN: [f32; 3] = [0.48145466, 0.4578275, 0.40821073];
#[cfg(feature = "ai-vision")]
#[allow(clippy::excessive_precision)]
const CLIP_STD: [f32; 3] = [0.26862954, 0.26130258, 0.27577711];

// ONNX filenames tried in order for both vision and text encoders
#[cfg(feature = "ai-vision")]
const VISION_MODEL_FILENAMES: &[&str] = &[
    "vision_model_quantized.onnx", // MobileCLIP int8 (new default)
    "vision_model_q4f16.onnx",     // CLIP ViT-B/32 q4f16 (legacy)
    "vision_model.onnx",           // unquantized fallback
];
#[cfg(feature = "ai-vision")]
const TEXT_MODEL_FILENAMES: &[&str] = &["text_model_quantized.onnx", "text_model.onnx"];

/// Preprocessing parameters loaded from `preprocessor_config.json` in the model directory.
/// MobileCLIP: 256×256, no normalization. CLIP ViT-B/32: 224×224, CLIP mean/std normalization.
#[cfg(feature = "ai-vision")]
#[derive(Debug, Clone)]
struct VisionPreprocessing {
    image_size: usize,
    normalize: bool,
}

#[cfg(feature = "ai-vision")]
impl VisionPreprocessing {
    fn from_model_dir(model_dir: &Path) -> Self {
        let config_path = model_dir.join("preprocessor_config.json");
        if let Ok(data) = std::fs::read_to_string(&config_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                let size = v["crop_size"]["height"].as_u64().unwrap_or(224) as usize;
                let normalize = v["do_normalize"].as_bool().unwrap_or(true);
                return Self {
                    image_size: size,
                    normalize,
                };
            }
        }
        // No preprocessor_config.json → assume CLIP ViT-B/32 defaults
        Self {
            image_size: 224,
            normalize: true,
        }
    }
}

/// MobileCLIP / CLIP zero-shot image classifier.
///
/// Requires in the model directory:
/// - `vision_model_quantized.onnx` (MobileCLIP) or `vision_model_q4f16.onnx` (CLIP ViT-B/32)
/// - `label_embeddings.bin` — pre-computed text embeddings for the vision label set
/// - `preprocessor_config.json` — controls image size and normalization (auto-detected)
///
/// Download via `pulse ai vision-download mobileclip-s2` (or mobileclip-s1).
/// Label embeddings are auto-generated on first load if the file is missing.
pub struct VisionTagger {
    #[cfg(feature = "ai-vision")]
    inner: VisionTaggerInner,
}

impl VisionTagger {
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

    /// Download an image from `url`, run vision encoder inference, and return tags above threshold.
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
    /// Used for threshold calibration via `pulse ai vision-debug`.
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

/// Compute label embeddings using the CLIP text encoder and write `label_embeddings.bin`
/// to `model_dir`. Requires a tokenizer and text model ONNX file in that directory.
/// Called automatically by `reload_vision_tagger` if the bin file is missing.
#[cfg(feature = "ai-vision")]
pub fn compute_clip_label_embeddings(model_dir: &Path) -> Result<(), TaggingError> {
    use super::vision_labels::vision_labels;
    use tokenizers::Tokenizer;

    const CLIP_MAX_LEN: usize = 77;

    let tokenizer_path = model_dir.join("tokenizer.json");
    if !tokenizer_path.exists() {
        return Err(TaggingError::Onnx(
            "tokenizer.json not found — re-download the vision model".into(),
        ));
    }

    // Try text model filenames in priority order
    let text_model_path = TEXT_MODEL_FILENAMES
        .iter()
        .map(|name| model_dir.join(name))
        .find(|p| p.exists())
        .ok_or_else(|| {
            let tried = TEXT_MODEL_FILENAMES.join(", ");
            TaggingError::Onnx(format!(
                "No text model ONNX found in {} (tried: {tried})",
                model_dir.display()
            ))
        })?;

    let tokenizer = Tokenizer::from_file(&tokenizer_path)
        .map_err(|e| TaggingError::Tokenizer(e.to_string()))?;

    let mut session = ort::session::Session::builder()
        .map_err(|e| TaggingError::Onnx(e.to_string()))?
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)
        .map_err(|e| TaggingError::Onnx(e.to_string()))?
        .commit_from_file(&text_model_path)
        .map_err(|e| TaggingError::Onnx(e.to_string()))?;

    // Detect model schema — MobileCLIP and CLIP ViT-B/32 differ in input/output names.
    // MobileCLIP text encoder: input_ids only, output varies.
    // CLIP ViT-B/32: input_ids + attention_mask → text_embeds.
    let input_names: Vec<String> = session
        .inputs()
        .iter()
        .map(|i| i.name().to_string())
        .collect();
    let output_names: Vec<String> = session
        .outputs()
        .iter()
        .map(|o| o.name().to_string())
        .collect();
    let has_attention_mask = input_names.iter().any(|n| n == "attention_mask");
    let text_output_name = output_names
        .into_iter()
        .next()
        .unwrap_or_else(|| "text_embeds".to_string());

    tracing::info!(
        model = %text_model_path.display(),
        ?input_names,
        text_output = %text_output_name,
        "Computing vision label embeddings"
    );

    let labels = vision_labels();
    let mut all_embeddings: Vec<Vec<f32>> = Vec::with_capacity(labels.len());
    // Actual embedding dim is read from the first output tensor — handles any model variant.
    let mut emb_dim: usize = 512;

    for label in labels {
        let encoding = tokenizer
            .encode(label.description, true)
            .map_err(|e| TaggingError::Tokenizer(e.to_string()))?;

        let raw_ids = encoding.get_ids();
        let actual_len = raw_ids.len().min(CLIP_MAX_LEN);

        let mut ids = vec![0i64; CLIP_MAX_LEN];
        for i in 0..actual_len {
            ids[i] = raw_ids[i] as i64;
        }

        let id_tensor = ort::value::TensorRef::from_array_view(([1usize, CLIP_MAX_LEN], &ids[..]))
            .map_err(|e| TaggingError::Onnx(e.to_string()))?;

        let outputs = if has_attention_mask {
            let raw_mask = encoding.get_attention_mask();
            let mut mask = vec![0i64; CLIP_MAX_LEN];
            for (i, &m) in raw_mask.iter().take(CLIP_MAX_LEN).enumerate() {
                mask[i] = m as i64;
            }
            let mask_tensor =
                ort::value::TensorRef::from_array_view(([1usize, CLIP_MAX_LEN], &mask[..]))
                    .map_err(|e| TaggingError::Onnx(e.to_string()))?;
            session
                .run(ort::inputs![
                    "input_ids"      => id_tensor,
                    "attention_mask" => mask_tensor,
                ])
                .map_err(|e| TaggingError::Onnx(e.to_string()))?
        } else {
            session
                .run(ort::inputs![
                    "input_ids" => id_tensor,
                ])
                .map_err(|e| TaggingError::Onnx(e.to_string()))?
        };

        let (_, embeds) = outputs[text_output_name.as_str()]
            .try_extract_tensor::<f32>()
            .map_err(|e| TaggingError::Onnx(format!("extract {text_output_name}: {e}")))?;

        // Collect the full output; dim is detected from the first label's output
        let raw: Vec<f32> = embeds.to_vec();
        if all_embeddings.is_empty() {
            emb_dim = raw.len(); // actual dim from this model
        }
        all_embeddings.push(l2_normalize(raw));
    }

    // Write VLAB binary: magic(4) + num_labels(4 LE) + emb_dim(4 LE) + flat f32 LE values
    let out_path = model_dir.join("label_embeddings.bin");
    let num_labels = labels.len();
    let mut data: Vec<u8> = Vec::with_capacity(12 + num_labels * emb_dim * 4);
    data.extend_from_slice(b"VLAB");
    data.extend_from_slice(&(num_labels as u32).to_le_bytes());
    data.extend_from_slice(&(emb_dim as u32).to_le_bytes());
    for emb in &all_embeddings {
        for &v in emb {
            data.extend_from_slice(&v.to_le_bytes());
        }
    }
    std::fs::write(&out_path, &data)
        .map_err(|e| TaggingError::Onnx(format!("write label_embeddings.bin: {e}")))?;

    tracing::info!(
        labels = num_labels,
        emb_dim,
        path = %out_path.display(),
        "Vision label embeddings written"
    );
    Ok(())
}

// ── Feature-gated implementation ──────────────────────────────────────────────

#[cfg(feature = "ai-vision")]
struct VisionTaggerInner {
    session: std::sync::Mutex<ort::session::Session>,
    /// (tag, L2-normalized embedding, threshold)
    labels: Vec<(String, Vec<f32>, f32)>,
    http: reqwest::Client,
    preprocessing: VisionPreprocessing,
}

#[cfg(feature = "ai-vision")]
impl VisionTaggerInner {
    fn load(model_dir: &Path) -> Result<Self, TaggingError> {
        let preprocessing = VisionPreprocessing::from_model_dir(model_dir);

        let onnx_path = VISION_MODEL_FILENAMES
            .iter()
            .map(|name| model_dir.join(name))
            .find(|p| p.exists())
            .ok_or_else(|| {
                let tried = VISION_MODEL_FILENAMES.join(", ");
                tracing::warn!(
                    dir = %model_dir.display(),
                    "No vision model ONNX found (tried: {tried})"
                );
                TaggingError::ModelNotLoaded
            })?;

        let embeddings_path = model_dir.join("label_embeddings.bin");
        if !embeddings_path.exists() {
            tracing::warn!(
                path = %embeddings_path.display(),
                "label_embeddings.bin not found — run `pulse ai vision-download` to regenerate"
            );
            return Err(TaggingError::ModelNotLoaded);
        }

        tracing::info!(
            path = %onnx_path.display(),
            image_size = preprocessing.image_size,
            normalize = preprocessing.normalize,
            "Loading vision encoder"
        );

        let session = ort::session::Session::builder()
            .map_err(|e| TaggingError::Onnx(e.to_string()))?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)
            .map_err(|e| TaggingError::Onnx(e.to_string()))?
            .commit_from_file(&onnx_path)
            .map_err(|e| TaggingError::Onnx(e.to_string()))?;

        let labels = load_label_embeddings(&embeddings_path)?;
        tracing::info!(labels = labels.len(), "Vision tagger ready");

        let http = reqwest::Client::builder()
            .user_agent("Pulse/0.1 vision-tagger")
            .timeout(std::time::Duration::from_secs(IMAGE_DOWNLOAD_TIMEOUT_SECS))
            .build()
            .map_err(|e| TaggingError::ImageNetwork(e.to_string()))?;

        Ok(Self {
            session: std::sync::Mutex::new(session),
            labels,
            http,
            preprocessing,
        })
    }

    async fn fetch_image(&self, url: &str) -> Result<Vec<u8>, TaggingError> {
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| TaggingError::ImageNetwork(format!("fetch {url}: {e}")))?;

        if !resp.status().is_success() {
            return Err(TaggingError::ImageNetwork(format!(
                "HTTP {} for {url}",
                resp.status()
            )));
        }

        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_lowercase();

        if !content_type.starts_with("image/") && !content_type.is_empty() {
            return Err(TaggingError::ImageDecode(format!(
                "not an image content-type: {content_type}"
            )));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| TaggingError::ImageNetwork(format!("read {url}: {e}")))?;

        if bytes.len() > IMAGE_MAX_BYTES {
            return Err(TaggingError::ImageDecode(format!(
                "image too large ({} bytes > {} limit)",
                bytes.len(),
                IMAGE_MAX_BYTES
            )));
        }

        Ok(bytes.to_vec())
    }

    fn embed_image(&self, image_bytes: &[u8]) -> Result<Vec<f32>, TaggingError> {
        let pixel_values = preprocess_image(image_bytes, &self.preprocessing)?;
        let size = self.preprocessing.image_size;

        let mut session = self
            .session
            .lock()
            .map_err(|_| TaggingError::Onnx("vision session mutex poisoned".into()))?;

        let tensor =
            ort::value::TensorRef::from_array_view(([1usize, 3, size, size], &pixel_values[..]))
                .map_err(|e| TaggingError::Onnx(e.to_string()))?;

        let outputs = session
            .run(ort::inputs!["pixel_values" => tensor])
            .map_err(|e| TaggingError::Onnx(e.to_string()))?;

        let (_, embeds) = outputs["image_embeds"]
            .try_extract_tensor::<f32>()
            .map_err(|e| TaggingError::Onnx(format!("extract image_embeds: {e}")))?;

        Ok(l2_normalize(embeds.to_vec()))
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
                    explanation: format!("MobileCLIP cosine {:.3}", score),
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

        let mut sims: Vec<(String, f32)> = self
            .labels
            .iter()
            .map(|(tag, label_embed, _)| (tag.clone(), cosine_sim(&image_embed, label_embed)))
            .collect();

        sims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(sims)
    }
}

// ── Image preprocessing ───────────────────────────────────────────────────────

/// Decode, preprocess, and return a flat NCHW tensor ready for the vision encoder.
///
/// MobileCLIP: resize shortest edge to crop_size, center crop, rescale to [0,1], no normalization.
/// CLIP ViT-B/32 (legacy): resize exact to 224×224, apply CLIP mean/std normalization.
#[cfg(feature = "ai-vision")]
fn preprocess_image(bytes: &[u8], p: &VisionPreprocessing) -> Result<Vec<f32>, TaggingError> {
    use image::imageops::FilterType;

    let img =
        image::load_from_memory(bytes).map_err(|e| TaggingError::ImageDecode(e.to_string()))?;

    let size = p.image_size as u32;

    let img = if p.image_size == 224 {
        // CLIP ViT-B/32 path: direct resize (preserves old behavior)
        img.resize_exact(size, size, FilterType::Lanczos3)
    } else {
        // MobileCLIP path: resize shortest edge to `size`, then center crop
        let (w, h) = (img.width(), img.height());
        let scale = if w <= h {
            size as f32 / w as f32
        } else {
            size as f32 / h as f32
        };
        let new_w = ((w as f32 * scale).ceil() as u32).max(size);
        let new_h = ((h as f32 * scale).ceil() as u32).max(size);
        let resized = img.resize(new_w, new_h, FilterType::Triangle);
        let x = (resized.width().saturating_sub(size)) / 2;
        let y = (resized.height().saturating_sub(size)) / 2;
        resized.crop_imm(x, y, size, size)
    };

    let rgb = img.to_rgb8();
    let s = p.image_size;
    let mut tensor = vec![0f32; 3 * s * s];

    for (y, row) in rgb.rows().enumerate() {
        for (x, pixel) in row.enumerate() {
            let r = pixel[0] as f32 / 255.0;
            let g = pixel[1] as f32 / 255.0;
            let b = pixel[2] as f32 / 255.0;
            let base = y * s + x;
            if p.normalize {
                tensor[base] = (r - CLIP_MEAN[0]) / CLIP_STD[0];
                tensor[s * s + base] = (g - CLIP_MEAN[1]) / CLIP_STD[1];
                tensor[2 * s * s + base] = (b - CLIP_MEAN[2]) / CLIP_STD[2];
            } else {
                // MobileCLIP: just rescale to [0, 1]
                tensor[base] = r;
                tensor[s * s + base] = g;
                tensor[2 * s * s + base] = b;
            }
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
        return Err(TaggingError::Onnx(
            "label_embeddings.bin has invalid magic bytes".into(),
        ));
    }

    let num_labels = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
    let emb_dim = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
    let expected_bytes = 12 + num_labels * emb_dim * 4;

    if data.len() != expected_bytes {
        return Err(TaggingError::Onnx(format!(
            "label_embeddings.bin size mismatch: expected {expected_bytes}, got {} bytes. \
             Delete it and re-run `pulse ai vision-download` to regenerate.",
            data.len()
        )));
    }

    let labels = vision_labels();
    if num_labels != labels.len() {
        return Err(TaggingError::Onnx(format!(
            "label_embeddings.bin has {num_labels} labels but vision_labels() defines {} — \
             delete it and re-run `pulse ai vision-download` to regenerate.",
            labels.len()
        )));
    }

    let mut result = Vec::with_capacity(num_labels);
    let mut cursor = &data[12..];

    for label in labels {
        let mut embedding = vec![0f32; emb_dim];
        for v in embedding.iter_mut() {
            let mut buf = [0u8; 4];
            cursor
                .read_exact(&mut buf)
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
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
    v
}

#[cfg(feature = "ai-vision")]
fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    // Both are L2-normalized, so dot product = cosine similarity
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}
