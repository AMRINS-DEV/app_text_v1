#!/usr/bin/env python3
"""Generates the ONNX parity fixture `crates/strategy` tests against
(§17 Phase 3: "ONNX export with a parity test asserting Rust `ort`
inference matches Python within 1e-6"). Run this once whenever the
synthetic dataset or training config changes; the output is committed —
the Rust test has no Python at test time to regenerate it from.

Usage: uv run --package agents-models python scripts/generate_onnx_fixture.py
"""

from __future__ import annotations

import json
from pathlib import Path

import onnxruntime as ort
from agents_models.training import (
    TrainConfig,
    export_to_onnx,
    make_synthetic_dataset,
    raw_probability,
    train_gbdt,
)

FIXTURE_DIR = (
    Path(__file__).resolve().parents[5] / "crates" / "strategy" / "testdata" / "onnx_parity"
)
N_FEATURES = 6
N_SAMPLE_ROWS = 10


def main() -> None:
    FIXTURE_DIR.mkdir(parents=True, exist_ok=True)
    data = make_synthetic_dataset(n_samples=500, n_features=N_FEATURES, seed=2024)
    model = train_gbdt(data.x, data.y, TrainConfig(n_estimators=20, num_leaves=7, seed=2024))

    onnx_path = FIXTURE_DIR / "model.onnx"
    export_to_onnx(model, n_features=N_FEATURES, path=str(onnx_path))

    sample_inputs = data.x[:N_SAMPLE_ROWS].astype("float32")
    python_probabilities = raw_probability(model, sample_inputs).astype("float64")

    # Confirm the exported ONNX file itself reproduces the same probabilities
    # before writing the fixture — the fixture must record what ort will
    # actually see, not what Python's own in-memory model produces.
    session = ort.InferenceSession(str(onnx_path), providers=["CPUExecutionProvider"])
    input_name = session.get_inputs()[0].name
    outputs = session.run(None, {input_name: sample_inputs})
    onnx_probabilities = [row[1] for row in outputs[1]]
    for py_p, onnx_p in zip(python_probabilities, onnx_probabilities, strict=True):
        msg = f"onnxruntime disagrees with the Python model: {py_p} vs {onnx_p}"
        assert abs(py_p - onnx_p) < 1e-6, msg

    fixture = {
        "n_features": N_FEATURES,
        "rows": [
            {
                "input": sample_inputs[i].tolist(),
                "expected_probability": float(python_probabilities[i]),
            }
            for i in range(N_SAMPLE_ROWS)
        ],
    }
    fixture_path = FIXTURE_DIR / "expected.json"
    fixture_path.write_text(json.dumps(fixture, indent=2) + "\n")
    print(f"wrote {onnx_path} and {fixture_path}")


if __name__ == "__main__":
    main()
