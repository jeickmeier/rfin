use finstack_valuations::calibration::quotes::{InstrumentConventions, RatesQuote};

fn expect_quote_err(payload: &str) {
    let err = serde_json::from_str::<RatesQuote>(payload)
        .expect_err("payload should fail deserialization");
    assert!(
        err.is_data(),
        "expected data error for payload {payload}, got {err}"
    );
}

#[test]
fn deposit_unknown_field_is_rejected() {
    expect_quote_err(
        r#"{
            "deposit": {
                "maturity": "2025-01-01",
                "rate": 0.01,
                "unexpected": true
            }
        }"#,
    );
}

#[test]
fn fra_unknown_field_is_rejected() {
    expect_quote_err(
        r#"{
            "fra": {
                "start": "2025-01-01",
                "end": "2025-04-01",
                "rate": 0.012,
                "oops": "value"
            }
        }"#,
    );
}

#[test]
fn future_unknown_field_is_rejected() {
    expect_quote_err(
        r#"{
            "future": {
                "expiry": "2025-01-01",
                "period_start": "2025-03-01",
                "period_end": "2025-06-01",
                "price": 99.5,
                "specs": {
                    "multiplier": 1.0,
                    "face_value": 1000000.0,
                    "delivery_months": 3,
                    "day_count": "Act360"
                },
                "conventions": {},
                "oops": 1
            }
        }"#,
    );
}

#[test]
fn swap_unknown_field_is_rejected() {
    expect_quote_err(
        r#"{
            "swap": {
                "maturity": "2026-01-01",
                "rate": 0.015,
                "is_ois": true,
                "mystery": "xyz"
            }
        }"#,
    );
}

#[test]
fn basis_swap_unknown_field_is_rejected() {
    expect_quote_err(
        r#"{
            "basis_swap": {
                "maturity": "2027-01-01",
                "spread_bp": 0.2,
                "mystery": "xyz"
            }
        }"#,
    );
}

#[test]
fn instrument_conventions_unknown_field_is_rejected() {
    let payload = r#"{"settlement_dayz": 2}"#;
    serde_json::from_str::<InstrumentConventions>(payload)
        .expect_err("unknown InstrumentConventions field should error");
}
