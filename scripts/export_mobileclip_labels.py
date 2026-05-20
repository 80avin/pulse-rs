#!/usr/bin/env python3
"""
Generate label_embeddings.bin for the Pulse MobileCLIP vision tagger.

Analogous to compute_clip_labels.py but targets MobileCLIP ONNX models.
The output format (VLAB binary) is identical so the same Rust reader applies.

MobileCLIP uses the same CLIP text-encoder interface.  This script:
  1. Looks for a text encoder ONNX in --model-dir (tries several common names).
  2. If no ONNX text encoder is found, falls back to the OpenCLIP Python library.
  3. Writes label_embeddings.bin to --model-dir.

Requirements (ONNX path):
    pip install onnxruntime transformers

Requirements (OpenCLIP fallback path):
    pip install open_clip_torch torch

Or with uv (recommended):
    uv venv scripts/.venv
    uv pip install onnxruntime transformers --python scripts/.venv/bin/python
    scripts/.venv/bin/python scripts/export_mobileclip_labels.py \\
        --model-dir .pulse-data/models/mobileclip

The VLAB binary format:
  [0..4]   magic: b"VLAB"
  [4..8]   num_labels: u32 (LE)
  [8..12]  emb_dim: u32 (LE)
  [12..]   num_labels × emb_dim × f32, row-major, LE
"""

import argparse
import struct
import sys
from pathlib import Path
from typing import List, Tuple

import numpy as np


# ── Label definitions — must match vision_labels.rs exactly ──────────────────
# (tag, description text fed to text encoder)
LABELS = [
    ("meme",        "a meme or humorous image with text overlay"),
    ("screenshot",  "a screenshot of a website, application, or software user interface"),
    ("photo-share", "a photograph of a real-world scene, landscape, person, or place"),
]

MAGIC = b"VLAB"

# Common ONNX text encoder filenames to try inside --model-dir
TEXT_ONNX_CANDIDATES = [
    "text_model.onnx",
    "onnx/text_model.onnx",
    "text_encoder.onnx",
    "onnx/text_encoder.onnx",
    "text.onnx",
]

# Fallback: OpenCLIP model name for MobileCLIP-S2 (adjust if needed)
OPENCLIP_MODEL_NAME = "MobileCLIP-S2"
OPENCLIP_PRETRAINED = "datacompdr"

# CLIP tokenizer HF repo (same tokenizer as ViT-B/32; MobileCLIP shares the vocab)
CLIP_TOKENIZER_REPO = "Xenova/clip-vit-base-patch32"


# ── ONNX path ─────────────────────────────────────────────────────────────────

def find_text_onnx(model_dir: Path) -> Path | None:
    """Return the first existing text-encoder ONNX in model_dir, or None."""
    for candidate in TEXT_ONNX_CANDIDATES:
        path = model_dir / candidate
        if path.exists():
            return path
    return None


def load_clip_tokenizer():
    """Load the CLIP tokenizer (shared vocab between CLIP and MobileCLIP)."""
    try:
        from transformers import CLIPTokenizerFast
    except ImportError:
        print("ERROR: transformers is required for ONNX path.")
        print("Install: pip install transformers")
        sys.exit(1)

    return CLIPTokenizerFast.from_pretrained(CLIP_TOKENIZER_REPO)


def compute_embeddings_onnx(
    text_onnx_path: Path,
    labels: List[Tuple[str, str]],
) -> List[Tuple[str, np.ndarray]]:
    """Run MobileCLIP text encoder via onnxruntime → L2-normalised embeddings."""
    try:
        import onnxruntime as ort
    except ImportError:
        print("ERROR: onnxruntime is required.")
        print("Install: pip install onnxruntime")
        sys.exit(1)

    tokenizer = load_clip_tokenizer()

    sess_opts = ort.SessionOptions()
    sess_opts.log_severity_level = 3
    session = ort.InferenceSession(str(text_onnx_path), sess_options=sess_opts)

    input_names = [inp.name for inp in session.get_inputs()]
    output_names = [out.name for out in session.get_outputs()]
    print(f"  text model inputs:  {input_names}")
    print(f"  text model outputs: {output_names}")

    embeddings = []
    for tag, description in labels:
        enc = tokenizer(
            description,
            return_tensors="np",
            padding="max_length",
            max_length=77,
            truncation=True,
        )

        feeds = {}
        for name in input_names:
            if name == "input_ids":
                feeds[name] = enc["input_ids"].astype(np.int64)
            elif name == "attention_mask":
                feeds[name] = enc["attention_mask"].astype(np.int64)

        outputs = session.run(output_names, feeds)

        # Prefer named text_embeds; fall back to first output
        if "text_embeds" in output_names:
            emb = outputs[output_names.index("text_embeds")][0]
        elif "pooler_output" in output_names:
            emb = outputs[output_names.index("pooler_output")][0]
        else:
            raw = outputs[0]
            # If shape is [1, seq, dim] take pooler (first token CLS)
            if raw.ndim == 3:
                emb = raw[0, 0, :]
            else:
                emb = raw[0]

        norm = np.linalg.norm(emb)
        if norm > 1e-8:
            emb = emb / norm

        print(f"  [{tag}] dim={len(emb)}, norm(before)={norm:.4f}")
        embeddings.append((tag, emb.astype(np.float32)))

    return embeddings


# ── OpenCLIP fallback path ────────────────────────────────────────────────────

def compute_embeddings_openclip(
    labels: List[Tuple[str, str]],
) -> List[Tuple[str, np.ndarray]]:
    """Use open_clip Python library to encode label descriptions."""
    try:
        import open_clip
    except ImportError:
        print("ERROR: open_clip_torch is required for the fallback path.")
        print("Install: pip install open_clip_torch torch")
        sys.exit(1)

    try:
        import torch
    except ImportError:
        print("ERROR: torch is required for the OpenCLIP fallback path.")
        print("Install: pip install torch")
        sys.exit(1)

    print(f"Loading OpenCLIP model '{OPENCLIP_MODEL_NAME}' (pretrained='{OPENCLIP_PRETRAINED}') ...")
    try:
        model, _, _ = open_clip.create_model_and_transforms(
            OPENCLIP_MODEL_NAME,
            pretrained=OPENCLIP_PRETRAINED,
        )
    except Exception as e:
        print(f"ERROR: could not load OpenCLIP model '{OPENCLIP_MODEL_NAME}': {e}")
        print("Available MobileCLIP models:")
        for name in open_clip.list_models():
            if "mobile" in name.lower() or "Mobile" in name:
                print(f"  {name}")
        sys.exit(1)

    model.eval()
    tokenize = open_clip.get_tokenizer(OPENCLIP_MODEL_NAME)

    embeddings = []
    with torch.no_grad():
        for tag, description in labels:
            tokens = tokenize([description])
            emb = model.encode_text(tokens)
            emb = emb[0].float().numpy()

            norm = np.linalg.norm(emb)
            if norm > 1e-8:
                emb = emb / norm

            print(f"  [{tag}] dim={len(emb)}, norm(before)={norm:.4f}")
            embeddings.append((tag, emb.astype(np.float32)))

    return embeddings


# ── Binary output ─────────────────────────────────────────────────────────────

def write_embeddings_bin(path: Path, embeddings: List[Tuple[str, np.ndarray]]) -> None:
    """Write label_embeddings.bin in Pulse VLAB binary format."""
    num_labels = len(embeddings)
    emb_dim = len(embeddings[0][1])

    with open(path, "wb") as f:
        f.write(MAGIC)
        f.write(struct.pack("<I", num_labels))
        f.write(struct.pack("<I", emb_dim))
        for _, emb in embeddings:
            f.write(emb.astype("<f4").tobytes())

    total_bytes = 12 + num_labels * emb_dim * 4
    print(f"\nWrote {num_labels} label embeddings to {path}")
    print(f"  {num_labels} labels × {emb_dim} dims = {total_bytes} bytes")


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--model-dir", required=True, type=Path,
        help="MobileCLIP model directory (label_embeddings.bin written here)",
    )
    args = parser.parse_args()

    model_dir: Path = args.model_dir
    if not model_dir.exists():
        print(f"ERROR: model directory does not exist: {model_dir}")
        sys.exit(1)

    output_path = model_dir / "label_embeddings.bin"
    print(f"Generating embeddings for {len(LABELS)} labels ...")
    print()

    text_onnx = find_text_onnx(model_dir)

    if text_onnx is not None:
        print(f"Found text encoder ONNX: {text_onnx}")
        embeddings = compute_embeddings_onnx(text_onnx, LABELS)
    else:
        print(
            f"No text encoder ONNX found in {model_dir} "
            f"(tried: {TEXT_ONNX_CANDIDATES})"
        )
        print("Falling back to OpenCLIP Python library ...")
        print()
        embeddings = compute_embeddings_openclip(LABELS)

    write_embeddings_bin(output_path, embeddings)

    print()
    print("Now activate the vision model in pulse:")
    print(f"  pulse --data-dir <data_dir> ai vision-download --no-activate")
    print(f"  echo mobileclip > <data_dir>/active_vision_model")


if __name__ == "__main__":
    main()
