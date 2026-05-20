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
    // Thresholds are calibrated for the geometric mean NLI score:
    //   score = sqrt(softmax3[entailment] × softmax2[entailment_vs_contradiction])
    // Typical ranges: clear positives 0.15–0.93, false positives (short/non-English) 0.02–0.05.
    &[
        // ── Semantic tags — NLI cross-encoder handles nuanced intent ──────────
        TagLabel {
            tag: "technical",
            description: "This is a technical article about software engineering, programming, or computing.",
            threshold: 0.55,
        },
        TagLabel {
            tag: "tutorial",
            description: "This is a tutorial, how-to guide, or step-by-step walkthrough.",
            threshold: 0.60,
        },
        TagLabel {
            tag: "research",
            description: "This is a published academic paper, peer-reviewed study, or preprint from a research institution.",
            threshold: 0.30,
        },
        TagLabel {
            tag: "news",
            description: "This reports a factual event — a government action, local development, official announcement, product launch, or organizational news.",
            threshold: 0.25,
        },
        TagLabel {
            tag: "civic",
            description: "This is a complaint, discussion, or report about local government services, public infrastructure failure, or a civic issue affecting residents.",
            threshold: 0.30,
        },
        TagLabel {
            tag: "local-rec",
            description: "The author is seeking specific local recommendations for a service, place, business, or institution from people with local knowledge.",
            threshold: 0.28,
        },
        TagLabel {
            tag: "culture",
            description: "This is about regional culture, folk traditions, local heritage, arts, or community cultural identity.",
            threshold: 0.25,
        },
        TagLabel {
            tag: "marketplace",
            description: "This is a post to buy, sell, rent, hire, or trade goods or services.",
            threshold: 0.40,
        },
        TagLabel {
            tag: "security",
            // NLI covers incident-style articles (hacking, breaches, backdoors).
            // Rule engine covers keyword-matched articles (vulnerability, CVE, ransomware, etc.).
            // Threshold lowered from 0.32: alt-C hypothesis produces lower absolute scores.
            description: "This reports a real-world cyberattack, system compromise, or security incident affecting real users or organizations.",
            threshold: 0.25,
        },
        TagLabel {
            tag: "ai-ml",
            // NLI hypothesis is weak for short AI news headlines; rules engine covers keyword matching.
            description: "This is about AI, machine learning, deep learning, LLMs, GPT, neural networks, or AI products.",
            threshold: 0.25,
        },
        TagLabel {
            tag: "privacy",
            description: "This is about user privacy, personal data collection, surveillance, or tracking.",
            threshold: 0.35,
        },
        TagLabel {
            tag: "policy",
            // Raised: "100 Best Novels", "reading code", "hosting on microcontroller" all scored 0.25–0.33 incorrectly.
            description: "This is about government regulation, legislation, law, or public policy.",
            threshold: 0.30,
        },
        TagLabel {
            tag: "science",
            description: "This reports a new scientific discovery, study, or experiment in biology, physics, medicine, chemistry, or astronomy.",
            threshold: 0.35,
        },
        TagLabel {
            tag: "clickbait",
            // Raised: legitimate articles about discoveries/breaches score 0.25–0.31 incorrectly.
            description: "This headline is sensationalist, misleading, or designed to provoke outrage.",
            threshold: 0.40,
        },
        // ── Structural tags — rules handle these; NLI entries kept for completeness ──
        TagLabel {
            tag: "show-hn",
            description: "The author is sharing their own project with the Hacker News community.",
            threshold: 0.50,
        },
        TagLabel {
            tag: "ask-hn",
            description: "This is a question directed at the Hacker News community.",
            threshold: 0.50,
        },
        TagLabel {
            tag: "job-posting",
            description: "This is a job listing or hiring announcement.",
            threshold: 0.50,
        },
        TagLabel {
            tag: "paywall",
            description: "This article is behind a subscription paywall.",
            threshold: 0.50,
        },
        TagLabel {
            tag: "video",
            description: "This is a link to a YouTube video or other video content.",
            threshold: 0.50,
        },
        // ── Quality tags — signal post worth filtering out ─────────────────────
        // These fire when strong signals indicate the post has no community value.
        // Absence of all tags means "unclassified" (new/unknown), not "bad".
        TagLabel {
            tag: "no-context",
            description: "This post asks a question or seeks help but provides no specific context, making it impossible to give a useful answer.",
            threshold: 0.40,
        },
        TagLabel {
            tag: "inappropriate",
            description: "This post seeks romantic partners, sexual encounters, couples accommodation, or contains solicitation of an adult nature.",
            threshold: 0.45,
        },
        TagLabel {
            tag: "noise",
            description: "This is a personal update, lifestyle share, or casual observation with no question and no community value.",
            threshold: 0.40,
        },
    ]
}
