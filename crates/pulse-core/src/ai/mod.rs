pub mod rules;
pub mod tagger;

pub use tagger::{TaggerHandle, TagRequest, tagger_task, TAGGER_QUEUE_SIZE};
pub use rules::{RuleEngine, TagRule, RulePattern, RuleScope, default_rules};
