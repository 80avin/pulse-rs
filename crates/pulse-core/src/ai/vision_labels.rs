/// A vision label for MobileCLIP / CLIP zero-shot image classification.
pub struct VisionLabel {
    pub tag: &'static str,
    /// Text description used to compute the label embedding via the CLIP text encoder.
    /// Regenerate `label_embeddings.bin` via `pulse ai vision-download` when changing these.
    pub description: &'static str,
    /// Minimum cosine similarity to the image embedding required to apply this tag.
    /// Calibrate with `pulse ai vision-debug <url>`.
    pub threshold: f32,
}

pub fn vision_labels() -> &'static [VisionLabel] {
    &[
        // ── Semantic labels ──────────────────────────────────────────────────────
        // These overlap with the text pipeline tag vocabulary so vision and text
        // signals can reinforce each other on mixed-content items (og_image present).
        // Thresholds calibrated for CLIP ViT-B/32 q4f16; calibrate with vision-debug.
        VisionLabel {
            tag: "technical",
            description: "a screenshot of code, a technical diagram, or a developer tool interface",
            threshold: 0.24,
        },
        VisionLabel {
            tag: "research",
            description: "a scientific research paper, academic publication, or data visualization chart",
            threshold: 0.24,
        },
        VisionLabel {
            tag: "ai-ml",
            description: "an image related to artificial intelligence, machine learning, or neural networks",
            threshold: 0.23,
        },
        VisionLabel {
            tag: "news",
            description: "a news article headline, news website, or breaking news screenshot",
            threshold: 0.23,
        },
        VisionLabel {
            tag: "security",
            description: "an image related to cybersecurity, hacking, or software vulnerability",
            threshold: 0.24,
        },
        VisionLabel {
            tag: "science",
            description: "a scientific diagram, biology illustration, or laboratory photograph",
            threshold: 0.23,
        },
        VisionLabel {
            tag: "tutorial",
            description: "a step-by-step instructional guide or how-to tutorial screenshot",
            threshold: 0.23,
        },
        VisionLabel {
            tag: "privacy",
            description: "an image about online privacy, data tracking, or digital surveillance",
            threshold: 0.23,
        },
        VisionLabel {
            tag: "policy",
            description: "a government policy document, legal regulation, or political statement",
            threshold: 0.23,
        },
        VisionLabel {
            tag: "clickbait",
            description: "a sensationalist clickbait thumbnail designed to attract clicks",
            threshold: 0.22,
        },
        // ── Visual style labels ──────────────────────────────────────────────────
        // These describe the image format/style rather than topic.
        VisionLabel {
            tag: "meme",
            description: "an internet meme or humorous image with text overlay",
            threshold: 0.24,
        },
        VisionLabel {
            tag: "screenshot",
            description: "a screenshot of a website, application, or software user interface",
            threshold: 0.24,
        },
        VisionLabel {
            tag: "photo-share",
            description: "a real-world photograph of a landscape, person, object, or place",
            threshold: 0.21,
        },
    ]
}
