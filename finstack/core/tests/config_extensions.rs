#![cfg(feature = "serde")]

use finstack_core::config::{FinstackConfig, RoundingMode};
use serde_json::json;

#[test]
fn config_extensions_roundtrip() {
    let mut cfg = FinstackConfig::default();
    cfg.rounding.mode = RoundingMode::AwayFromZero;
    cfg.extensions
        .insert("custom.section.v1", json!({ "alpha": 1, "beta": true }));

    let encoded = serde_json::to_string(&cfg).expect("serialize");
    let decoded: FinstackConfig = serde_json::from_str(&encoded).expect("deserialize");

    assert_eq!(decoded.rounding.mode, RoundingMode::AwayFromZero);
    let section = decoded
        .extensions
        .get("custom.section.v1")
        .expect("section exists");
    assert_eq!(section["alpha"], 1);
    assert_eq!(section["beta"], true);
}

#[test]
fn config_extensions_backward_compat() {
    let json = r#"{
        "rounding": {
            "mode": "Bankers",
            "ingest_scale": { "overrides": {} },
            "output_scale": { "overrides": {} }
        }
    }"#;

    let cfg: FinstackConfig = serde_json::from_str(json).expect("deserialize");
    assert_eq!(cfg.rounding.mode, RoundingMode::Bankers);
    assert!(cfg.extensions.is_empty());
}

