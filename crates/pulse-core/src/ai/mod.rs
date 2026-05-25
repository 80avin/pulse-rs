pub mod fasttext;
pub mod labels;
pub mod miniml;
pub mod model_handle;
pub mod onnx;
pub mod rules;
pub mod tagger;
pub mod vision;
pub mod vision_labels;

pub use fasttext::FastTextTagger;
pub use miniml::MiniMlTagger;
pub use model_handle::ModelHandle;
pub use onnx::OnnxTagger;
pub use rules::{RuleEngine, RulePattern, RuleScope, TagRule, default_rules};
pub use tagger::{TAGGER_QUEUE_SIZE, TagRequest, TaggerHandle, tagger_task};
pub use vision::VisionTagger;
