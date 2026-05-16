pub mod labels;
pub mod onnx;
pub mod rules;
pub mod tagger;
pub mod vision;
pub mod vision_labels;

pub use tagger::{TaggerHandle, TagRequest, tagger_task, TAGGER_QUEUE_SIZE};
pub use rules::{RuleEngine, TagRule, RulePattern, RuleScope, default_rules};
pub use onnx::OnnxTagger;
pub use vision::VisionTagger;
