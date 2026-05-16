#!/usr/bin/env python3
"""
Generate label_embeddings.bin for the Pulse CLIP vision tagger.

Uses the CLIP ViT-B/32 ONNX text encoder (downloaded temporarily) to compute
L2-normalized embeddings for each label in vision_labels.rs, then writes
label_embeddings.bin to the target model directory.

Requirements (much lighter than PyTorch):
    pip install onnxruntime transformers huggingface_hub

Or with uv (recommended):
    uv venv scripts/.venv
    uv pip install onnxruntime transformers huggingface_hub --python scripts/.venv/bin/python
    scripts/.venv/bin/python scripts/compute_clip_labels.py --model-dir .pulse-data/models/clip-vit-b32

The CLIP text encoder (~72 MB) is downloaded into a temp directory and deleted
after use unless --keep-text-encoder is passed.
"""

import argparse
import struct
import sys
import tempfile
import shutil
import numpy as np
from pathlib import Path


# ── Label definitions — must match vision_labels.rs exactly ──────────────────
# (tag, description text fed to CLIP text encoder)
LABELS = [
    ("meme",        "a meme or humorous image with text overlay"),
    ("screenshot",  "a screenshot of a website, application, or software user interface"),
    ("photo-share", "a photograph of a real-world scene, landscape, person, or place"),
]

CLIP_HF_REPO    = "Xenova/clip-vit-base-patch32"
TEXT_ONNX_PATH  = "onnx/text_model.onnx"   # ~72 MB
EMB_DIM         = 512
MAGIC           = b"VLAB"


def download_text_encoder(dest_dir: Path) -> Path:
    """Download CLIP text ONNX to dest_dir, return the onnx file path."""
    try:
        from huggingface_hub import hf_hub_download
    except ImportError:
        print("ERROR: huggingface_hub is required.")
        print("Install: uv pip install huggingface_hub")
        sys.exit(1)

    print(f"Downloading CLIP text encoder from {CLIP_HF_REPO} ...")
    path = hf_hub_download(
        repo_id=CLIP_HF_REPO,
        filename=TEXT_ONNX_PATH,
        local_dir=str(dest_dir),
    )
    return Path(path)


def load_tokenizer():
    """Load the CLIP tokenizer (no torch needed — just vocab files)."""
    try:
        from transformers import CLIPTokenizerFast
    except ImportError:
        print("ERROR: transformers is required.")
        print("Install: uv pip install transformers")
        sys.exit(1)

    return CLIPTokenizerFast.from_pretrained(CLIP_HF_REPO)


def compute_embeddings(text_onnx_path: Path, labels):
    """Run CLIP text encoder via onnxruntime, return list of (tag, embedding)."""
    try:
        import onnxruntime as ort
    except ImportError:
        print("ERROR: onnxruntime is required.")
        print("Install: uv pip install onnxruntime")
        sys.exit(1)

    tokenizer = load_tokenizer()

    sess_opts = ort.SessionOptions()
    sess_opts.log_severity_level = 3  # suppress ort logs
    session = ort.InferenceSession(str(text_onnx_path), sess_options=sess_opts)

    input_names = [inp.name for inp in session.get_inputs()]
    output_names = [out.name for out in session.get_outputs()]
    print(f"  text model inputs:  {input_names}")
    print(f"  text model outputs: {output_names}")

    embeddings = []
    for tag, description in labels:
        enc = tokenizer(description, return_tensors="np", padding="max_length",
                        max_length=77, truncation=True)

        feeds = {}
        for name in input_names:
            if name == "input_ids":
                feeds[name] = enc["input_ids"].astype(np.int64)
            elif name == "attention_mask":
                feeds[name] = enc["attention_mask"].astype(np.int64)

        outputs = session.run(output_names, feeds)

        # Xenova CLIP text model outputs: text_embeds (projected 512-dim) or pooler_output
        if "text_embeds" in output_names:
            emb = outputs[output_names.index("text_embeds")][0]
        else:
            emb = outputs[0][0]

        # L2-normalize
        norm = np.linalg.norm(emb)
        if norm > 1e-8:
            emb = emb / norm

        print(f"  [{tag}] dim={len(emb)}, norm(before)={norm:.4f}")
        embeddings.append((tag, emb.astype(np.float32)))

    return embeddings


def write_embeddings_bin(path: Path, embeddings):
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
    print(f"\nWrote {path}")
    print(f"  {num_labels} labels × {emb_dim} dims = {total_bytes} bytes")


def main():
    parser = argparse.ArgumentParser(description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument(
        "--model-dir", required=True, type=Path,
        help="clip-vit-b32 model directory (label_embeddings.bin written here)",
    )
    parser.add_argument(
        "--keep-text-encoder", action="store_true",
        help="keep the downloaded text encoder ONNX after generating embeddings",
    )
    args = parser.parse_args()

    model_dir: Path = args.model_dir
    if not model_dir.exists():
        print(f"ERROR: model directory does not exist: {model_dir}")
        sys.exit(1)

    output_path = model_dir / "label_embeddings.bin"
    print(f"Generating embeddings for {len(LABELS)} labels ...")
    print()

    tmp_dir = None
    try:
        tmp_dir = tempfile.mkdtemp(prefix="pulse_clip_")
        text_onnx = download_text_encoder(Path(tmp_dir))
        print()
        embeddings = compute_embeddings(text_onnx, LABELS)
        write_embeddings_bin(output_path, embeddings)
    finally:
        if tmp_dir and not args.keep_text_encoder:
            shutil.rmtree(tmp_dir, ignore_errors=True)
            print("(temporary text encoder deleted)")

    print()
    print("Now activate the vision model in pulse:")
    print(f"  pulse --data-dir <data_dir> ai vision-download --no-activate")
    print(f"  echo clip-vit-b32 > <data_dir>/active_vision_model")
    print(f"  # or: run 'pulse ai vision-download' again (will auto-activate if embeddings exist)")


if __name__ == "__main__":
    main()
