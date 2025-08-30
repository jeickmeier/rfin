#![cfg(feature = "serde")]

use finstack_core::{Currency, Money};
use finstack_valuations::pricing::result::ValuationResult;

#[test]
fn valuation_result_roundtrip_serde() {
    // Build a simple result
    let as_of = time::macros::date!(2024 - 01 - 15);
    let mut vr = ValuationResult::stamped("BOND-1", as_of, Money::new(100.0, Currency::USD));
    vr.measures.insert("pv".to_string(), 100.0);
    vr.measures.insert("dv01".to_string(), 0.0123);

    // Serialize to JSON
    let json = serde_json::to_string(&vr).expect("serialize");

    // Deserialize back
    let de: ValuationResult = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(de.instrument_id, "BOND-1");
    assert_eq!(de.as_of, as_of);
    assert_eq!(de.value.currency(), Currency::USD);
    assert!((de.value.amount() - 100.0).abs() < 1e-9);
    assert!(de.measures.get("pv").is_some());
    assert!(de.measures.get("dv01").is_some());
    // Ensure meta carried through
    assert_eq!(
        de.meta.core.numeric_mode as u8,
        finstack_core::config::numeric_mode() as u8
    );
}
