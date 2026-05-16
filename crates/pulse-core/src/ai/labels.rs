/// A tag with its NLI hypothesis and entailment threshold.
pub struct TagLabel {
    pub tag: &'static str,
    /// Complete hypothesis sentence fed to the NLI cross-encoder as the second sequence.
    /// Phrased so that entailment means "this article belongs to this category."
    pub description: &'static str,
    /// Minimum softmax entailment probability to apply this tag.
    /// Calibrated for NLI cross-encoder (0.0–1.0 scale). Use `pulse ai debug`
    /// to inspect raw probabilities and tune these per-category.
    pub threshold: f32,
}

pub fn tag_labels() -> &'static [TagLabel] {
    &[
        // ── Semantic tags — NLI cross-encoder handles nuanced intent ──────────
        TagLabel {
            tag: "technical",
            description: "This is a technical article about software engineering or programming.",
            threshold: 0.55,
        },
        TagLabel {
            tag: "tutorial",
            description: "This is a tutorial, how-to guide, or step-by-step walkthrough.",
            threshold: 0.55,
        },
        TagLabel {
            tag: "research",
            description: "This is an academic research paper or peer-reviewed scientific study.",
            threshold: 0.55,
        },
        TagLabel {
            tag: "news",
            description: "This is a news article about a company, government, or organization — acquisitions, layoffs, product launches by established companies, or regulatory events.",
            threshold: 0.65,
        },
        TagLabel {
            tag: "discussion",
            description: "This is an opinion piece or community discussion seeking views.",
            threshold: 0.55,
        },
        TagLabel {
            tag: "security",
            description: "This is about a cybersecurity vulnerability, exploit, or data breach.",
            threshold: 0.60,
        },
        TagLabel {
            tag: "ai-ml",
            description: "This is about artificial intelligence, machine learning, or large language models.",
            threshold: 0.55,
        },
        TagLabel {
            tag: "privacy",
            description: "This is about user privacy, personal data collection, or surveillance.",
            threshold: 0.60,
        },
        TagLabel {
            tag: "policy",
            description: "This is about government regulation, legislation, or public policy.",
            threshold: 0.55,
        },
        TagLabel {
            tag: "science",
            description: "This is about a scientific discovery, experiment, or research finding.",
            threshold: 0.55,
        },
        TagLabel {
            tag: "clickbait",
            description: "This headline is sensationalist, misleading, or designed to provoke outrage.",
            threshold: 0.65,
        },
        // ── Structural tags — rules handle these; NLI entries kept for completeness ──
        TagLabel {
            tag: "show-hn",
            description: "The author is sharing their own project with the Hacker News community.",
            threshold: 0.75,
        },
        TagLabel {
            tag: "ask-hn",
            description: "This is a question directed at the Hacker News community.",
            threshold: 0.75,
        },
        TagLabel {
            tag: "job-posting",
            description: "This is a job listing or hiring announcement.",
            threshold: 0.70,
        },
        TagLabel {
            tag: "paywall",
            description: "This article is behind a subscription paywall.",
            threshold: 0.70,
        },
        TagLabel {
            tag: "video",
            description: "This is a link to a YouTube video or other video content.",
            threshold: 0.70,
        },
    ]
}
