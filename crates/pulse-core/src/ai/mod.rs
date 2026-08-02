pub mod rules;
pub mod tagger;

pub use rules::{RuleEngine, RulePattern, RuleScope, TagRule, default_rules};
pub use tagger::{RulesTagger, TAGGER_QUEUE_SIZE, TagRequest, Tagger, TaggerHandle, tagger_task};
