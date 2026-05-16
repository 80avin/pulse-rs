/// A vision label for CLIP zero-shot image classification.
pub struct VisionLabel {
    pub tag: &'static str,
    /// Text description used to compute the label embedding via CLIP text encoder.
    /// Run `scripts/compute_clip_labels.py` to regenerate `label_embeddings.bin` when changing these.
    pub description: &'static str,
    /// Minimum cosine similarity to the image embedding required to apply this tag.
    /// CLIP ViT-B/32 cosine scores for matching concepts typically fall in 0.20–0.35.
    /// Calibrate with `pulse ai vision-debug <url>`.
    pub threshold: f32,
}

pub fn vision_labels() -> &'static [VisionLabel] {
    &[
        VisionLabel {
            tag: "meme",
            description: "a meme or humorous image with text overlay",
            threshold: 0.22,
        },
        VisionLabel {
            tag: "screenshot",
            description: "a screenshot of a website, application, or software user interface",
            threshold: 0.22,
        },
        VisionLabel {
            tag: "photo-share",
            description: "a photograph of a real-world scene, landscape, person, or place",
            threshold: 0.19,
        },
    ]
}
