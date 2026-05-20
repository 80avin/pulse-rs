#!/usr/bin/env python3
"""
Train a 2-layer MLP classifier head on frozen MiniLM embeddings and export
to PMLP binary format for Rust inference.

PMLP Binary Format (all little-endian):
  Header (16 bytes):
    [0..4]   magic: b"PMLP"
    [4..8]   version: u32 = 1
    [8..12]  input_dim: u32   (384)
    [12..16] num_labels: u32

  Label table: [len: u16 LE][utf-8 bytes] × num_labels

  Layer 1:
    hidden_dim: u32
    W1: hidden_dim × input_dim × f32, row-major, LE
    b1: hidden_dim × f32, LE

  Layer 2:
    W2: num_labels × hidden_dim × f32, row-major, LE
    b2: num_labels × f32, LE

# Requirements: pip install onnxruntime numpy tokenizers
"""

import argparse
import json
import struct
import sys
from pathlib import Path
from typing import List, Tuple

import numpy as np


# ── Adam optimizer ────────────────────────────────────────────────────────────

def adam_update(param, grad, m, v, t, lr=0.001, beta1=0.9, beta2=0.999, eps=1e-8):
    m = beta1 * m + (1 - beta1) * grad
    v = beta2 * v + (1 - beta2) * grad ** 2
    m_hat = m / (1 - beta1 ** t)
    v_hat = v / (1 - beta2 ** t)
    param -= lr * m_hat / (np.sqrt(v_hat) + eps)
    return param, m, v


# ── Data loading ──────────────────────────────────────────────────────────────

def load_jsonl(path: str) -> Tuple[List[str], List[List[str]]]:
    """Read JSONL file: {"text": "...", "labels": [...]} per line."""
    texts = []
    labels = []
    with open(path, "r", encoding="utf-8") as f:
        for lineno, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError as e:
                print(f"WARNING: skipping malformed JSON on line {lineno}: {e}")
                continue
            texts.append(obj["text"])
            labels.append(obj.get("labels", []))
    return texts, labels


# ── Tokenization ──────────────────────────────────────────────────────────────

def load_tokenizer(model_dir: Path):
    """Load tokenizer from model_dir/tokenizer.json using the tokenizers library."""
    try:
        from tokenizers import Tokenizer
    except ImportError:
        print("ERROR: tokenizers library is required.")
        print("Install: pip install tokenizers")
        sys.exit(1)

    tok_path = model_dir / "tokenizer.json"
    if not tok_path.exists():
        print(f"ERROR: tokenizer.json not found at {tok_path}")
        sys.exit(1)

    tokenizer = Tokenizer.from_file(str(tok_path))
    tokenizer.enable_padding(pad_id=0, pad_token="[PAD]", length=128)
    tokenizer.enable_truncation(max_length=128)
    return tokenizer


def tokenize_batch(tokenizer, texts: List[str]):
    """
    Tokenize a batch of texts.
    Returns dict with input_ids and attention_mask as int64 numpy arrays.
    """
    encodings = tokenizer.encode_batch(texts)
    input_ids = np.array([enc.ids for enc in encodings], dtype=np.int64)
    attention_mask = np.array([enc.attention_mask for enc in encodings], dtype=np.int64)
    return {"input_ids": input_ids, "attention_mask": attention_mask}


# ── Embedding ─────────────────────────────────────────────────────────────────

def embed_texts(
    texts: List[str],
    model_dir: Path,
    batch_size: int,
) -> np.ndarray:
    """
    Compute MiniLM embeddings for all texts.
    Returns float32 array of shape [N, 384].
    """
    try:
        import onnxruntime as ort
    except ImportError:
        print("ERROR: onnxruntime is required.")
        print("Install: pip install onnxruntime")
        sys.exit(1)

    tokenizer = load_tokenizer(model_dir)

    # Locate the ONNX model — try common filenames
    onnx_candidates = ["model.onnx", "onnx/model.onnx", "model_quantized.onnx"]
    onnx_path = None
    for candidate in onnx_candidates:
        candidate_path = model_dir / candidate
        if candidate_path.exists():
            onnx_path = candidate_path
            break

    if onnx_path is None:
        print(f"ERROR: no ONNX model found in {model_dir}")
        print(f"  Tried: {onnx_candidates}")
        sys.exit(1)

    print(f"  Loading ONNX model from {onnx_path} ...")
    sess_opts = ort.SessionOptions()
    sess_opts.log_severity_level = 3
    session = ort.InferenceSession(str(onnx_path), sess_options=sess_opts)

    input_names = {inp.name for inp in session.get_inputs()}
    output_names = [out.name for out in session.get_outputs()]

    all_embeddings = []
    total_batches = (len(texts) + batch_size - 1) // batch_size

    for batch_idx in range(total_batches):
        batch_texts = texts[batch_idx * batch_size:(batch_idx + 1) * batch_size]
        enc = tokenize_batch(tokenizer, batch_texts)

        feeds = {}
        feeds["input_ids"] = enc["input_ids"]
        feeds["attention_mask"] = enc["attention_mask"]
        if "token_type_ids" in input_names:
            feeds["token_type_ids"] = np.zeros_like(enc["input_ids"])

        outputs = session.run(output_names, feeds)

        # Prefer last_hidden_state for mean pooling
        if "last_hidden_state" in output_names:
            hidden = outputs[output_names.index("last_hidden_state")]
        else:
            hidden = outputs[0]
            # If output is already pooled (2-D), skip mean pooling
            if hidden.ndim == 2:
                all_embeddings.append(hidden.astype(np.float32))
                print(f"  Batch {batch_idx + 1}/{total_batches} done", end="\r")
                continue

        # Mean pool over non-padding positions [batch, seq_len, dim] → [batch, dim]
        mask = enc["attention_mask"][:, :, np.newaxis].astype(np.float32)
        # Trim mask to seq dimension of hidden state in case lengths differ
        seq_len = hidden.shape[1]
        mask = mask[:, :seq_len, :]
        summed = (hidden * mask).sum(axis=1)
        counts = mask.sum(axis=1).clip(min=1e-9)
        pooled = summed / counts  # [batch, 384]

        all_embeddings.append(pooled.astype(np.float32))
        print(f"  Batch {batch_idx + 1}/{total_batches} done", end="\r")

    print()  # newline after \r progress
    embeddings = np.concatenate(all_embeddings, axis=0)
    return embeddings  # [N, 384]


# ── Label encoding ────────────────────────────────────────────────────────────

def build_label_vocab(labels_list: List[List[str]]) -> List[str]:
    """Collect all unique labels in sorted order."""
    vocab = set()
    for labels in labels_list:
        vocab.update(labels)
    return sorted(vocab)


def encode_labels(labels_list: List[List[str]], label_vocab: List[str]) -> np.ndarray:
    """Return binary matrix [N, L]."""
    label_index = {lbl: i for i, lbl in enumerate(label_vocab)}
    N = len(labels_list)
    L = len(label_vocab)
    Y = np.zeros((N, L), dtype=np.float32)
    for i, labels in enumerate(labels_list):
        for lbl in labels:
            if lbl in label_index:
                Y[i, label_index[lbl]] = 1.0
    return Y


# ── Training ──────────────────────────────────────────────────────────────────

def train_mlp(
    X_train: np.ndarray,
    Y_train: np.ndarray,
    X_val: np.ndarray,
    Y_val: np.ndarray,
    hidden_dim: int,
    epochs: int,
    lr: float,
    batch_size: int,
) -> Tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
    """
    Train 2-layer MLP with pure NumPy + Adam.
    Returns (W1, b1, W2, b2).
    """
    input_dim = X_train.shape[1]
    num_labels = Y_train.shape[1]
    N = X_train.shape[0]

    rng = np.random.default_rng(42)

    # He initialization
    W1 = rng.standard_normal((hidden_dim, input_dim)).astype(np.float32) * np.sqrt(2.0 / input_dim)
    b1 = np.zeros(hidden_dim, dtype=np.float32)
    W2 = rng.standard_normal((num_labels, hidden_dim)).astype(np.float32) * np.sqrt(2.0 / hidden_dim)
    b2 = np.zeros(num_labels, dtype=np.float32)

    # Adam state
    mW1 = np.zeros_like(W1); vW1 = np.zeros_like(W1)
    mb1 = np.zeros_like(b1); vb1 = np.zeros_like(b1)
    mW2 = np.zeros_like(W2); vW2 = np.zeros_like(W2)
    mb2 = np.zeros_like(b2); vb2 = np.zeros_like(b2)

    t = 0  # global Adam step counter
    indices = np.arange(N)

    for epoch in range(1, epochs + 1):
        rng.shuffle(indices)
        epoch_loss = 0.0
        num_batches = 0

        for start in range(0, N, batch_size):
            batch_idx = indices[start:start + batch_size]
            X = X_train[batch_idx]
            Y = Y_train[batch_idx]
            bs = X.shape[0]
            t += 1

            # Forward
            h = np.maximum(0.0, X @ W1.T + b1)           # [bs, hidden]
            logits = h @ W2.T + b2                         # [bs, num_labels]
            probs = 1.0 / (1.0 + np.exp(-np.clip(logits, -30, 30)))  # sigmoid

            # Loss: binary cross-entropy
            loss = -np.mean(
                Y * np.log(probs + 1e-9) + (1 - Y) * np.log(1 - probs + 1e-9)
            )
            epoch_loss += loss
            num_batches += 1

            # Backprop
            dlogits = (probs - Y) / (bs * num_labels)
            dW2 = dlogits.T @ h
            db2 = dlogits.sum(axis=0)
            dh = dlogits @ W2
            dh_relu = dh * (h > 0)
            dW1 = dh_relu.T @ X
            db1 = dh_relu.sum(axis=0)

            # Adam updates
            W1, mW1, vW1 = adam_update(W1, dW1, mW1, vW1, t, lr=lr)
            b1, mb1, vb1 = adam_update(b1, db1, mb1, vb1, t, lr=lr)
            W2, mW2, vW2 = adam_update(W2, dW2, mW2, vW2, t, lr=lr)
            b2, mb2, vb2 = adam_update(b2, db2, mb2, vb2, t, lr=lr)

        if epoch % 10 == 0:
            avg_loss = epoch_loss / max(num_batches, 1)
            print(f"  Epoch {epoch:4d}/{epochs}  loss={avg_loss:.5f}")

    return W1, b1, W2, b2


# ── Threshold calibration ─────────────────────────────────────────────────────

def calibrate_thresholds(
    W1: np.ndarray,
    b1: np.ndarray,
    W2: np.ndarray,
    b2: np.ndarray,
    X_val: np.ndarray,
    Y_val: np.ndarray,
    label_vocab: List[str],
    output_dir: Path,
) -> None:
    """Sweep thresholds [0.1..0.8] per label, pick F1-maximising threshold."""
    h = np.maximum(0.0, X_val @ W1.T + b1)
    logits = h @ W2.T + b2
    probs = 1.0 / (1.0 + np.exp(-np.clip(logits, -30, 30)))

    thresholds_sweep = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8]
    result = {}
    label_counts = Y_val.sum(axis=0)

    for j, lbl in enumerate(label_vocab):
        if label_counts[j] < 3:
            result[lbl] = 0.5
            continue

        best_f1 = -1.0
        best_thresh = 0.5
        for thresh in thresholds_sweep:
            preds_j = (probs[:, j] >= thresh).astype(np.float32)
            tp = float(np.sum(preds_j * Y_val[:, j]))
            fp = float(np.sum(preds_j * (1 - Y_val[:, j])))
            fn = float(np.sum((1 - preds_j) * Y_val[:, j]))
            prec = tp / (tp + fp) if (tp + fp) > 0 else 0.0
            rec = tp / (tp + fn) if (tp + fn) > 0 else 0.0
            f1 = (2 * prec * rec / (prec + rec)) if (prec + rec) > 0 else 0.0
            if f1 > best_f1:
                best_f1 = f1
                best_thresh = thresh

        result[lbl] = best_thresh

    out_path = output_dir / "miniml_thresholds.json"
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2)
    print(f"\nThresholds saved to {out_path}")
    print(f"  {'Label':<30} {'Threshold':>10}")
    print(f"  {'-'*30} {'-'*10}")
    for lbl, thresh in result.items():
        print(f"  {lbl:<30} {thresh:>10.1f}")


def evaluate_val(
    W1, b1, W2, b2,
    X_val: np.ndarray,
    Y_val: np.ndarray,
    label_vocab: List[str],
) -> None:
    """Print per-label F1 and micro-averaged F1 on val set (threshold=0.5)."""
    h = np.maximum(0.0, X_val @ W1.T + b1)
    logits = h @ W2.T + b2
    probs = 1.0 / (1.0 + np.exp(-np.clip(logits, -30, 30)))
    y_pred = (probs >= 0.5).astype(np.float32)

    print("\nValidation evaluation (threshold=0.5):")
    print(f"  {'Label':<30} {'P':>6} {'R':>6} {'F1':>6} {'Support':>8}")
    print(f"  {'-'*30} {'-'*6} {'-'*6} {'-'*6} {'-'*8}")

    micro_tp = micro_fp = micro_fn = 0.0

    for j, lbl in enumerate(label_vocab):
        tp = float(np.sum(y_pred[:, j] * Y_val[:, j]))
        fp = float(np.sum(y_pred[:, j] * (1 - Y_val[:, j])))
        fn = float(np.sum((1 - y_pred[:, j]) * Y_val[:, j]))
        support = int(Y_val[:, j].sum())
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


# ── Export PMLP ───────────────────────────────────────────────────────────────

def export_pmlp(
    W1: np.ndarray,
    b1: np.ndarray,
    W2: np.ndarray,
    b2: np.ndarray,
    label_vocab: List[str],
    output_dir: Path,
) -> None:
    """Write mlp_head.pmlp in PMLP binary format."""
    input_dim = W1.shape[1]
    hidden_dim = W1.shape[0]
    num_labels = W2.shape[0]

    out_path = output_dir / "mlp_head.pmlp"
    with open(out_path, "wb") as f:
        # Header (16 bytes)
        f.write(b"PMLP")
        f.write(struct.pack("<I", 1))             # version
        f.write(struct.pack("<I", input_dim))
        f.write(struct.pack("<I", num_labels))

        # Label table
        for lbl in label_vocab:
            lbl_bytes = lbl.encode("utf-8")
            f.write(struct.pack("<H", len(lbl_bytes)))
            f.write(lbl_bytes)

        # Layer 1
        f.write(struct.pack("<I", hidden_dim))
        f.write(W1.astype("<f4").tobytes())       # [hidden_dim, input_dim]
        f.write(b1.astype("<f4").tobytes())       # [hidden_dim]

        # Layer 2
        f.write(W2.astype("<f4").tobytes())       # [num_labels, hidden_dim]
        f.write(b2.astype("<f4").tobytes())       # [num_labels]

    file_size = out_path.stat().st_size
    print(f"\nExported mlp_head.pmlp ({file_size // 1024} KB)")
    print(f"  input_dim:  {input_dim}")
    print(f"  hidden_dim: {hidden_dim}")
    print(f"  num_labels: {num_labels}")


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--input", required=True,
        help='JSONL file: {"text": "...", "labels": [...]} per line',
    )
    parser.add_argument(
        "--model-dir", required=True,
        help="Directory containing model.onnx and tokenizer.json",
    )
    parser.add_argument(
        "--output-dir", default=None,
        help="Where to write mlp_head.pmlp (defaults to --model-dir)",
    )
    parser.add_argument("--hidden-dim", type=int, default=128, help="MLP hidden layer size (default: 128)")
    parser.add_argument("--epochs", type=int, default=100, help="Training epochs (default: 100)")
    parser.add_argument("--lr", type=float, default=0.001, help="Adam learning rate (default: 0.001)")
    parser.add_argument("--val-split", type=float, default=0.1, help="Validation fraction (default: 0.1)")
    parser.add_argument("--batch-size", type=int, default=32, help="Training batch size (default: 32)")
    args = parser.parse_args()

    model_dir = Path(args.model_dir)
    output_dir = Path(args.output_dir) if args.output_dir else model_dir
    output_dir.mkdir(parents=True, exist_ok=True)

    if not model_dir.exists():
        print(f"ERROR: model directory does not exist: {model_dir}")
        sys.exit(1)

    # Load data
    print(f"Loading data from {args.input} ...")
    texts, labels_list = load_jsonl(args.input)
    if not texts:
        print("ERROR: no data found in input file.")
        sys.exit(1)
    print(f"  Loaded {len(texts)} examples")

    # Build label vocabulary
    label_vocab = build_label_vocab(labels_list)
    print(f"  Labels ({len(label_vocab)}): {label_vocab}")

    # Shuffle and split
    rng = np.random.default_rng(42)
    perm = rng.permutation(len(texts))
    texts = [texts[i] for i in perm]
    labels_list = [labels_list[i] for i in perm]

    val_count = max(1, int(len(texts) * args.val_split))
    train_texts = texts[:-val_count]
    val_texts = texts[-val_count:]
    train_labels = labels_list[:-val_count]
    val_labels = labels_list[-val_count:]
    print(f"  Train: {len(train_texts)}  Val: {len(val_texts)}")

    # Embed
    print(f"\nComputing training embeddings ...")
    X_train = embed_texts(train_texts, model_dir, args.batch_size)
    print(f"  X_train: {X_train.shape}")

    print(f"Computing validation embeddings ...")
    X_val = embed_texts(val_texts, model_dir, args.batch_size)
    print(f"  X_val:   {X_val.shape}")

    # Encode labels
    Y_train = encode_labels(train_labels, label_vocab)
    Y_val = encode_labels(val_labels, label_vocab)

    # Train
    print(f"\nTraining MLP (hidden_dim={args.hidden_dim}, epochs={args.epochs}, lr={args.lr}) ...")
    W1, b1, W2, b2 = train_mlp(
        X_train, Y_train,
        X_val, Y_val,
        hidden_dim=args.hidden_dim,
        epochs=args.epochs,
        lr=args.lr,
        batch_size=args.batch_size,
    )

    # Export
    export_pmlp(W1, b1, W2, b2, label_vocab, output_dir)

    # Calibrate thresholds
    calibrate_thresholds(W1, b1, W2, b2, X_val, Y_val, label_vocab, output_dir)

    # Evaluate
    evaluate_val(W1, b1, W2, b2, X_val, Y_val, label_vocab)

    print("\nDone.")


if __name__ == "__main__":
    main()
