#!/usr/bin/env python3
"""
Migrate the training label store for the tag vocabulary redesign:
  - Remove 'discussion' from all existing labels
  - Add weak-labeled examples for: civic, local-rec, culture, marketplace
    sourced from reddit_raw.jsonl using the same rule patterns as rules.rs

Usage:
    python scripts/update_labels_for_new_tags.py \
        --labels ~/.local/share/pulse/training/labels.jsonl \
        --reddit reddit_raw.jsonl \
        [--dry-run]
"""

import argparse
import json
import re
import time
from pathlib import Path


# ── Rule patterns (mirror crates/pulse-core/src/ai/rules.rs) ──────────────────

CIVIC_KW = [
    "electricity", "power cut", "power outage", "load shedding",
    "jkpdd", "jkpowerco", "light nhi", "light nahi",
    "municipal", "jmc ", "civic sense", "civic body", "smart city",
    "water supply", "no water", "pani nahi", "sewage", "drainage",
    "pothole", "road condition", "bsnl",
]
CIVIC_RE = [
    re.compile(r"(?i)(batti|bijli).{0,20}(nhi|nahi|kab|kyu)"),
    re.compile(r"(?i)no\s+electricity"),
]

LOCAL_REC_KW = [
    "therapist in jammu", "therapist for", "good therapist",
    "dermatologist in jammu", "good dermatologist", "best dermatologist",
    "dentist in jammu", "good dentist", "best dentist",
    "good physician", "best doctor", "good doctor",
    "urologist", "neurologist", "gynecologist", "psychiatrist",
    "best momos", "good momos", "best dhaba", "best dhabha",
    "suggest best dhabha", "suggest best dhaba",
    "restaurant in jammu", "best restaurant", "good restaurant",
    "cafe in jammu", "cabin cafe", "good cafe", "best cafe",
    "halal spot", "veg spot",
    "gym in jammu", "good gym", "best gym",
    "suggest good gym", "suggest good gyms", "gym fees",
    "gcet jammu", "gcet ", "smvdu", "iim jammu", "miet jammu",
    "mbs college", "pmsss college", "colleges in pmsss",
    "college in pmsss", "good colleges through pmsss",
    "best coaching", "good coaching", "coaching in jammu",
    "jkssb classes", "jkssb coaching",
    "good lawyer", "lawyer in jammu", "advocate in jammu",
    "hotel in jammu", "good hotel", "review hotel", "hostel in jammu",
]
LOCAL_REC_RE = [
    re.compile(r"(?i)\b(best|good|any\s+good|recommend|suggest)\b.{0,40}\bin\s+jammu\b"),
    re.compile(r"(?i)\bhow\s+is\b.{0,30}\b(jkssb|gcet|smvdu|pmsss|iim\s+jammu)\b"),
    re.compile(r"(?i)\bcentral university of jammu\b"),
]

CULTURE_KW = [
    "pahari culture", "gojri", "folk tradition", "folk culture",
    "local heritage", "dogra heritage", "dogra dynasty", "dogra kingdom",
    "dogra sadar", "bahu fort", "anchali", "rasonth", "kalari", "kaladi",
    "dogri film", "dogri poem", "dogri song", "dogri music",
    "jammu culture", "cultural identity", "local tradition",
]
CULTURE_RE = [
    re.compile(r"(?i)\bdogr[ia]\b"),
    re.compile(r"(?i)\bmaharaj(a)?\b"),
]

MARKETPLACE_KW = [
    "for sale", " wts ", " wtb ", " wtt ",
    "looking to sell", "anyone selling", "looking to buy", "anyone buying",
    "room for rent", "flat for rent", "for rent", "rental",
    "room available", "pg available", "accommodation available",
    "cook needed", "maid needed", "care giver", "caregiver needed",
    "driver needed", "looking for cook", "need a cook",
    "vacancy", "hiring for",
]
MARKETPLACE_RE = [
    re.compile(r"(?i)\bsell(ing)?\b"),
    re.compile(r"(?i)\bfor\s+rent\b"),
]


def classify(title: str) -> list[str]:
    t = title.lower()
    tags = []

    if any(k in t for k in CIVIC_KW) or any(r.search(title) for r in CIVIC_RE):
        tags.append("civic")

    if any(k in t for k in LOCAL_REC_KW) or any(r.search(title) for r in LOCAL_REC_RE):
        tags.append("local-rec")

    if any(k in t for k in CULTURE_KW) or any(r.search(title) for r in CULTURE_RE):
        tags.append("culture")

    if any(k in t for k in MARKETPLACE_KW) or any(r.search(title) for r in MARKETPLACE_RE):
        tags.append("marketplace")

    return tags


def extract_title(text: str) -> str:
    """Strip 'domain:...' suffix added by build_input_text()."""
    idx = text.rfind(" domain:")
    return text[:idx] if idx != -1 else text


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--labels", required=True, help="Path to labels.jsonl")
    parser.add_argument("--reddit", required=True, help="Path to reddit_raw.jsonl (unused after migration, kept for interface)")
    parser.add_argument("--dry-run", action="store_true", help="Print changes without writing")
    args = parser.parse_args()

    labels_path = Path(args.labels)

    # ── Step 1: Load existing labels ──────────────────────────────────────────
    # For items that had 'discussion':
    #   - Remove 'discussion' tag
    #   - Re-classify the text with the new rules to add replacement tags
    existing: list[dict] = []
    removed_discussion = 0
    added_new = {"civic": 0, "local-rec": 0, "culture": 0, "marketplace": 0}

    with open(labels_path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            item = json.loads(line)
            old_tags = set(item.get("tags", []))
            new_tags = old_tags - {"discussion"}
            had_discussion = len(new_tags) != len(old_tags)

            if had_discussion:
                removed_discussion += 1
                # Re-classify to add new tags
                title = extract_title(item.get("text", ""))
                replacement_tags = classify(title)
                for t in replacement_tags:
                    if t not in new_tags:
                        new_tags.add(t)
                        added_new[t] = added_new.get(t, 0) + 1

            item["tags"] = sorted(new_tags)
            existing.append(item)

    print(f"Existing items:             {len(existing)}")
    print(f"Removed 'discussion' from:  {removed_discussion} items")
    print(f"New tags added to existing items:")
    for tag, count in sorted(added_new.items(), key=lambda x: -x[1]):
        if count > 0:
            print(f"  {count:4d}  {tag}")

    # ── Step 2: Inject reddit_raw.jsonl posts not yet in the store ────────────
    existing_ids = {item["item_id"] for item in existing}
    new_items: list[dict] = []
    reddit_path = Path(args.reddit)

    if reddit_path.exists():
        skipped_no_tags = skipped_duplicate = 0
        with open(reddit_path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                post = json.loads(line)
                item_id = post.get("id", "")
                title = post.get("title", "").strip()
                if not title or not item_id:
                    continue
                if item_id in existing_ids:
                    skipped_duplicate += 1
                    continue
                tags = classify(title)
                if not tags:
                    skipped_no_tags += 1
                    continue
                new_items.append({
                    "item_id": item_id,
                    "text": title,
                    "tags": tags,
                    "labeled_at": int(time.time()),
                })
        print(f"\nReddit posts not yet in store: {len(new_items)} new "
              f"(skipped {skipped_duplicate} dup, {skipped_no_tags} no-tags)")

    # ── Step 3: Summary ───────────────────────────────────────────────────────
    from collections import Counter
    all_items = existing + new_items
    all_counts = Counter(t for item in all_items for t in item["tags"])
    print("\nFull label distribution after migration:")
    for tag, count in sorted(all_counts.items(), key=lambda x: -x[1]):
        print(f"  {count:4d}  {tag}")

    if args.dry_run:
        print("\n[dry-run] No files written.")
        return

    # ── Step 4: Write back ────────────────────────────────────────────────────
    tmp_path = labels_path.with_suffix(".tmp")
    with open(tmp_path, "w", encoding="utf-8") as f:
        for item in all_items:
            f.write(json.dumps(item, ensure_ascii=False) + "\n")
    tmp_path.rename(labels_path)

    print(f"\nWrote {len(all_items)} items to {labels_path}")
    print("\nNext steps:")
    print("  pulse ai train export-fasttext")
    print("  python scripts/train_fasttext.py --input ~/.local/share/pulse/training/train.txt --output ~/.local/share/pulse/models/fasttext-v2/")
    print("  pulse ai train export-jsonl")
    print("  python scripts/train_miniml.py --input ~/.local/share/pulse/training/train.jsonl \\")
    print("      --model-dir ~/.local/share/pulse/models/miniml-v1/ \\")
    print("      --output ~/.local/share/pulse/models/miniml-v1/")


if __name__ == "__main__":
    main()
