use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub fn build_input_text(title: &str, url: Option<&str>) -> String {
    let host = url.and_then(|u| {
        let after_scheme = u.split("://").nth(1)?;
        let host_part = after_scheme.split('/').next()?;
        let host = host_part.strip_prefix("www.").unwrap_or(host_part);
        Some(host.to_lowercase())
    });

    match host {
        // Reddit and HN posts: use title only. Appending any aggregator marker
        // (domain:reddit.com or source:reddit) causes the model to associate the
        // marker itself with content categories, producing massive false positives
        // for short titles ("Food", "Fun") where the title has no n-gram signal.
        Some(h) if h == "reddit.com" || h == "redd.it" => title.to_string(),
        Some(h) if h == "news.ycombinator.com" => title.to_string(),
        // Article domains (arxiv, github, youtube, etc.) carry real signal — keep them.
        Some(h) if !h.is_empty() => format!("{} domain:{}", title, h),
        _ => title.to_string(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabeledItem {
    pub item_id: String,
    pub text: String,
    pub tags: Vec<String>,
    pub labeled_at: i64,
}

#[derive(Debug, Serialize)]
pub struct LabelStats {
    pub total: usize,
    pub tag_counts: HashMap<String, usize>,
}

pub struct LabelStore {
    path: PathBuf,
}

impl LabelStore {
    pub fn new(training_dir: &Path) -> anyhow::Result<Self> {
        fs::create_dir_all(training_dir)?;
        Ok(Self {
            path: training_dir.join("labels.jsonl"),
        })
    }

    pub fn load_all(&self) -> anyhow::Result<Vec<LabeledItem>> {
        if !self.path.exists() {
            return Ok(vec![]);
        }

        let file = fs::File::open(&self.path)?;
        let reader = std::io::BufReader::new(file);
        let mut items = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<LabeledItem>(&line) {
                Ok(item) => items.push(item),
                Err(e) => eprintln!("warning: skipping unparseable label line: {}", e),
            }
        }

        Ok(items)
    }

    pub fn labeled_ids(&self) -> anyhow::Result<HashSet<String>> {
        let items = self.load_all()?;
        Ok(items.into_iter().map(|i| i.item_id).collect())
    }

    pub fn upsert(&self, item: LabeledItem) -> anyhow::Result<()> {
        let mut items = self.load_all()?;
        if let Some(existing) = items.iter_mut().find(|i| i.item_id == item.item_id) {
            *existing = item;
        } else {
            items.push(item);
        }
        self.write_all(&items)
    }

    pub fn upsert_batch(&self, new_items: Vec<LabeledItem>) -> anyhow::Result<usize> {
        let existing = self.load_all()?;
        let mut map: HashMap<String, LabeledItem> =
            existing.into_iter().map(|i| (i.item_id.clone(), i)).collect();

        for item in new_items {
            map.insert(item.item_id.clone(), item);
        }

        let all: Vec<LabeledItem> = map.into_values().collect();
        let count = all.len();
        self.write_all(&all)?;
        Ok(count)
    }

    pub fn stats(&self) -> anyhow::Result<LabelStats> {
        let items = self.load_all()?;
        let total = items.len();
        let mut tag_counts: HashMap<String, usize> = HashMap::new();

        for item in &items {
            for tag in &item.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        Ok(LabelStats { total, tag_counts })
    }

    pub fn export_fasttext(&self, dest: &Path) -> anyhow::Result<usize> {
        let items = self.load_all()?;
        let file = fs::File::create(dest)?;
        let mut writer = BufWriter::new(file);
        let mut count = 0;

        for item in &items {
            if item.tags.is_empty() {
                continue;
            }
            let mut sorted_tags = item.tags.clone();
            sorted_tags.sort();
            let label_str: String = sorted_tags
                .iter()
                .map(|t| format!("__label__{}", t))
                .collect::<Vec<_>>()
                .join(" ");
            writeln!(writer, "{} {}", label_str, item.text)?;
            count += 1;
        }

        Ok(count)
    }

    pub fn export_jsonl(&self, dest: &Path) -> anyhow::Result<usize> {
        let items = self.load_all()?;
        let file = fs::File::create(dest)?;
        let mut writer = BufWriter::new(file);
        let mut count = 0;

        for item in &items {
            let obj = serde_json::json!({
                "text": item.text,
                "labels": item.tags,
            });
            writeln!(writer, "{}", serde_json::to_string(&obj)?)?;
            count += 1;
        }

        Ok(count)
    }

    fn write_all(&self, items: &[LabeledItem]) -> anyhow::Result<()> {
        let tmp_path = self.path.with_extension("tmp");
        {
            let file = fs::File::create(&tmp_path)?;
            let mut writer = BufWriter::new(file);
            for item in items {
                writeln!(writer, "{}", serde_json::to_string(item)?)?;
            }
        }
        fs::rename(&tmp_path, &self.path)?;
        Ok(())
    }
}
