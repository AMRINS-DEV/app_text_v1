//! §17 Phase 3 exit criterion: "ONNX export with a parity test asserting
//! Rust `ort` inference matches Python within 1e-6." The fixture this test
//! reads (`testdata/onnx_parity/{model.onnx,expected.json}`) was generated
//! by `services/agents/packages/models/scripts/generate_onnx_fixture.py`
//! and is committed — this test has no Python interpreter to regenerate it
//! from, only to check against it.

use serde::Deserialize;
use strategy::OnnxClassifier;

#[derive(Deserialize)]
struct FixtureRow {
    input: Vec<f32>,
    expected_probability: f64,
}

#[derive(Deserialize)]
struct Fixture {
    n_features: usize,
    rows: Vec<FixtureRow>,
}

#[test]
fn rust_ort_inference_matches_the_python_trained_model_within_1e_minus_6() {
    let fixture_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/onnx_parity");
    let fixture_json = std::fs::read_to_string(format!("{fixture_dir}/expected.json"))
        .expect("fixture missing — run generate_onnx_fixture.py (see that script's own doc comment)");
    let fixture: Fixture = serde_json::from_str(&fixture_json).unwrap();

    let mut classifier = OnnxClassifier::load(format!("{fixture_dir}/model.onnx"), fixture.n_features).unwrap();

    assert!(!fixture.rows.is_empty(), "fixture must contain at least one row");
    for (i, row) in fixture.rows.iter().enumerate() {
        let predicted = classifier.predict_positive_class_probability(&row.input).unwrap();
        let diff = (predicted as f64 - row.expected_probability).abs();
        assert!(
            diff < 1e-6,
            "row {i}: Rust ort ({predicted}) diverges from Python ({}) by {diff}, exceeding 1e-6",
            row.expected_probability
        );
    }
}
