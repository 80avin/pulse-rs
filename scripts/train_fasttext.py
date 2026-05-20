#!/usr/bin/env python3
"""
Train a FastText supervised classifier on labeled data and export weights to
the PFTM custom binary format for Rust inference.

PFTM Binary Format (all little-endian):
  Header (32 bytes):
    [0..4]   magic: b"PFTM"
    [4..8]   version: u32 = 1
    [8..12]  num_labels: u32
    [12..16] embedding_dim: u32
    [16..20] num_buckets: u32
    [20..24] min_ngram: u32
    [24..28] max_ngram: u32
    [28..32] reserved: u32 = 0

  Label table (num_labels entries):
    [len: u16 LE][utf-8 bytes]  × num_labels

  Char n-gram embedding matrix (char n-grams only — NOT word vocabulary):
    num_buckets × embedding_dim × f32, row-major, LE

  Output weight matrix:
    num_labels × embedding_dim × f32, row-major, LE

  Output bias (OVA has no separate bias — write num_labels zeros):
    num_labels × f32, LE

# Requirements: pip install fasttext-wheel numpy
"""

import argparse
import json
import math
import random
import struct
import sys
from pathlib import Path
from typing import List, Tuple

import numpy as np


def _patch_fasttext_numpy2():
    """Monkey-patch NumPy 2.x + fasttext compatibility.

    NumPy 2.x changed np.array(obj, copy=False) to raise ValueError instead of
    silently allowing a copy. fasttext's Python wrapper calls this pattern.
    We patch np.array itself so the copy=False path uses np.asarray instead.
    """
    try:
        import numpy as _np
        _orig_array = _np.array

        def _compat_array(obj, dtype=None, copy=True, **kwargs):
            if copy is False:
                return _np.asarray(obj, dtype=dtype,
                                   **{k: v for k, v in kwargs.items() if k != "copy"})
            return _orig_array(obj, dtype=dtype, copy=copy, **kwargs)

        _np.array = _compat_array
    except Exception:
        pass  # best-effort


def parse_fasttext_file(path: str):
    """
    Read FastText format: __label__a __label__b text\n
    Yields (labels: List[str], text: str) tuples.
    Labels are tokens starting with '__label__', prefix stripped.
    Text is the remaining tokens joined with space.
    """
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line.strip():
                continue
            tokens = line.split()
            labels = []
            text_tokens = []
            for token in tokens:
                if token.startswith("__label__"):
                    labels.append(token[len("__label__"):])
                else:
                    text_tokens.append(token)
            yield labels, " ".join(text_tokens)


def sigmoid(x: np.ndarray) -> np.ndarray:
    return 1.0 / (1.0 + np.exp(-np.clip(x, -30, 30)))


def calibrate_thresholds(
    model,
    val_items: List[Tuple[List[str], str]],
    output_dir: Path,
) -> None:
    """
    For each label, sweep thresholds [0.1..0.8] and pick the one that
    maximises F1 on val_items. Save to {output_dir}/fasttext_thresholds.json.
    """
    try:
        import fasttext as ft  # noqa: F401 — already imported at call site
    except ImportError:
        pass  # model is already loaded; import only needed above

    all_labels = model.get_labels()
    # Strip __label__ prefix that fastText adds
    label_names = [lbl.replace("__label__", "") for lbl in all_labels]

    label_index = {lbl: i for i, lbl in enumerate(label_names)}

    # Build val arrays: y_true [N, L] and raw scores [N, L]
    N = len(val_items)
    L = len(label_names)

    y_true = np.zeros((N, L), dtype=np.float32)
    scores = np.zeros((N, L), dtype=np.float32)

    for i, (labels, text) in enumerate(val_items):
        for lbl in labels:
            if lbl in label_index:
                y_true[i, label_index[lbl]] = 1.0

        # get_output_matrix gives weight vectors; use predict with k=-1
        # for probabilities across all labels
        preds, probs = model.predict(text, k=-1)
        for pred_lbl, prob in zip(preds, probs):
            name = pred_lbl.replace("__label__", "")
            if name in label_index:
                scores[i, label_index[name]] = float(prob)

    thresholds_sweep = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8]
    result = {}
    label_counts = y_true.sum(axis=0)  # positive examples per label in val

    for j, lbl in enumerate(label_names):
        if label_counts[j] < 3:
            result[lbl] = 0.5
            continue

        best_f1 = -1.0
        best_thresh = 0.5
        for thresh in thresholds_sweep:
            preds_j = (scores[:, j] >= thresh).astype(np.float32)
            tp = float(np.sum(preds_j * y_true[:, j]))
            fp = float(np.sum(preds_j * (1 - y_true[:, j])))
            fn = float(np.sum((1 - preds_j) * y_true[:, j]))
            prec = tp / (tp + fp) if (tp + fp) > 0 else 0.0
            rec = tp / (tp + fn) if (tp + fn) > 0 else 0.0
            f1 = (2 * prec * rec / (prec + rec)) if (prec + rec) > 0 else 0.0
            if f1 > best_f1:
                best_f1 = f1
                best_thresh = thresh

        result[lbl] = best_thresh

    out_path = output_dir / "fasttext_thresholds.json"
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2)
    print(f"Thresholds saved to {out_path}")
    print(f"  {'Label':<30} {'Threshold':>10}")
    print(f"  {'-'*30} {'-'*10}")
    for lbl, thresh in result.items():
        print(f"  {lbl:<30} {thresh:>10.1f}")


def evaluate_val(model, val_items: List[Tuple[List[str], str]]) -> None:
    """Print per-label F1 table and micro-averaged F1 on val_items."""
    all_labels = model.get_labels()
    label_names = [lbl.replace("__label__", "") for lbl in all_labels]
    label_index = {lbl: i for i, lbl in enumerate(label_names)}
    L = len(label_names)
    N = len(val_items)

    y_true = np.zeros((N, L), dtype=np.float32)
    scores = np.zeros((N, L), dtype=np.float32)

    for i, (labels, text) in enumerate(val_items):
        for lbl in labels:
            if lbl in label_index:
                y_true[i, label_index[lbl]] = 1.0
        preds, probs = model.predict(text, k=-1)
        for pred_lbl, prob in zip(preds, probs):
            name = pred_lbl.replace("__label__", "")
            if name in label_index:
                scores[i, label_index[name]] = float(prob)

    # Use 0.5 as default evaluation threshold
    y_pred = (scores >= 0.5).astype(np.float32)

    print("\nValidation evaluation (threshold=0.5):")
    print(f"  {'Label':<30} {'P':>6} {'R':>6} {'F1':>6} {'Support':>8}")
    print(f"  {'-'*30} {'-'*6} {'-'*6} {'-'*6} {'-'*8}")

    micro_tp = micro_fp = micro_fn = 0.0

    for j, lbl in enumerate(label_names):
        tp = float(np.sum(y_pred[:, j] * y_true[:, j]))
        fp = float(np.sum(y_pred[:, j] * (1 - y_true[:, j])))
        fn = float(np.sum((1 - y_pred[:, j]) * y_true[:, j]))
        support = int(y_true[:, j].sum())
        prec = tp / (tp + fp) if (tp + fp) > 0 else 0.0
        rec = tp / (tp + fn) if (tp + fn) > 0 else 0.0
        f1 = (2 * prec * rec / (prec + rec)) if (prec + rec) > 0 else 0.0
        micro_tp += tp
        micro_fp += fp
        micro_fn += fn
        print(f"  {lbl:<30} {prec:>6.3f} {rec:>6.3f} {f1:>6.3f} {support:>8}")

    micro_p = micro_tp / (micro_tp + micro_fp) if (micro_tp + micro_fp) > 0 else 0.0
    micro_r = micro_tp / (micro_tp + micro_fn) if (micro_tp + micro_fn) > 0 else 0.0
    micro_f1 = (2 * micro_p * micro_r / (micro_p + micro_r)) if (micro_p + micro_r) > 0 else 0.0
    print(f"\n  Micro-averaged F1: {micro_f1:.4f}")


def export_pftm(model, output_dir: Path, args) -> None:
    """Export the trained FastText model to PFTM binary format."""
    import numpy as np

    all_labels_raw = model.get_labels()
    label_names = [lbl.replace("__label__", "") for lbl in all_labels_raw]
    num_labels = len(label_names)

    # Embedding dim from model
    dim = model.get_dimension()

    # Extract char n-gram embeddings:
    # input_matrix has shape [vocab_size + num_buckets, dim]
    # We only export the last num_buckets rows (char n-gram part)
    input_matrix = model.get_input_matrix()  # numpy array
    vocab_size = len(model.get_words())
    num_buckets = args.bucket

    if input_matrix.shape[0] < vocab_size + num_buckets:
        print(
            f"WARNING: input_matrix rows ({input_matrix.shape[0]}) < "
            f"vocab_size ({vocab_size}) + num_buckets ({num_buckets}). "
            f"Adjusting num_buckets to {input_matrix.shape[0] - vocab_size}."
        )
        num_buckets = input_matrix.shape[0] - vocab_size

    ngram_matrix = input_matrix[vocab_size:vocab_size + num_buckets, :]
    ngram_matrix = ngram_matrix.astype(np.float32)

    # Output weight matrix [num_labels, dim]
    output_matrix = model.get_output_matrix()
    output_matrix = output_matrix.astype(np.float32)

    # Bias: OVA has no separate bias — write zeros
    bias = np.zeros(num_labels, dtype=np.float32)

    out_path = output_dir / "fasttext.pftm"
    with open(out_path, "wb") as f:
        # Header (32 bytes)
        f.write(b"PFTM")
        f.write(struct.pack("<I", 1))             # version
        f.write(struct.pack("<I", num_labels))
        f.write(struct.pack("<I", dim))
        f.write(struct.pack("<I", num_buckets))
        f.write(struct.pack("<I", args.minn))
        f.write(struct.pack("<I", args.maxn))
        f.write(struct.pack("<I", 0))             # reserved

        # Label table
        for lbl in label_names:
            lbl_bytes = lbl.encode("utf-8")
            f.write(struct.pack("<H", len(lbl_bytes)))
            f.write(lbl_bytes)

        # Char n-gram embedding matrix: [num_buckets, dim]
        f.write(ngram_matrix.astype("<f4").tobytes())

        # Output weight matrix: [num_labels, dim]
        f.write(output_matrix.astype("<f4").tobytes())

        # Output bias: [num_labels]
        f.write(bias.astype("<f4").tobytes())

    file_size = out_path.stat().st_size
    print(f"\nExported {out_path} ({file_size // 1024} KB)")
    print(f"  num_labels:   {num_labels}")
    print(f"  embedding_dim:{dim}")
    print(f"  num_buckets:  {num_buckets}")
    print(f"  minn:         {args.minn}  maxn: {args.maxn}")
    print(f"  ngram matrix: {ngram_matrix.shape}  output matrix: {output_matrix.shape}")


def main():
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--input", required=True, help="FastText train.txt path")
    parser.add_argument("--output", required=True, help="Output directory")
    parser.add_argument("--dim", type=int, default=50, help="Embedding dimension (default: 50)")
    parser.add_argument("--epoch", type=int, default=50, help="Training epochs (default: 50)")
    parser.add_argument("--lr", type=float, default=0.5, help="Learning rate (default: 0.5)")
    parser.add_argument("--bucket", type=int, default=50000, help="Number of char n-gram buckets (default: 50000)")
    parser.add_argument("--minn", type=int, default=2, help="Min char n-gram length (default: 2)")
    parser.add_argument("--maxn", type=int, default=5, help="Max char n-gram length (default: 5)")
    parser.add_argument("--val-split", type=float, default=0.1, help="Fraction of data for val/threshold calibration (default: 0.1)")
    args = parser.parse_args()

    try:
        import fasttext
        _patch_fasttext_numpy2()
    except ImportError:
        print("ERROR: fasttext is required.")
        print("Install: pip install fasttext-wheel")
        sys.exit(1)

    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)

    # Read all items
    print(f"Reading data from {args.input} ...")
    all_items = list(parse_fasttext_file(args.input))
    if not all_items:
        print("ERROR: no data found in input file.")
        sys.exit(1)

    print(f"  Total examples: {len(all_items)}")

    # Shuffle and split
    rng = random.Random(42)
    rng.shuffle(all_items)
    val_count = max(1, int(len(all_items) * args.val_split))
    train_items = all_items[:-val_count]
    val_items = all_items[-val_count:]
    print(f"  Train: {len(train_items)}  Val: {len(val_items)}")

    # Write train file to a temp location for fasttext
    import tempfile, os
    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".txt", delete=False, encoding="utf-8"
    ) as tmp:
        train_path = tmp.name
        for labels, text in train_items:
            label_str = " ".join(f"__label__{lbl}" for lbl in labels)
            tmp.write(f"{label_str} {text}\n")

    try:
        print(f"\nTraining FastText (OVA) — dim={args.dim}, epoch={args.epoch}, lr={args.lr} ...")
        model = fasttext.train_supervised(
            input=train_path,
            dim=args.dim,
            epoch=args.epoch,
            lr=args.lr,
            bucket=args.bucket,
            minn=args.minn,
            maxn=args.maxn,
            loss="ova",
            verbose=2,
        )
    finally:
        os.unlink(train_path)

    export_pftm(model, output_dir, args)
    calibrate_thresholds(model, val_items, output_dir)
    evaluate_val(model, val_items)

    print("\nDone.")


if __name__ == "__main__":
    main()
