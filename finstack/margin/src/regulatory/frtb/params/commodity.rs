//! Commodity risk prescribed parameters per BCBS d457.

/// Commodity delta risk weights by bucket (percentage).
///
/// Buckets 1-11 per FRTB specification:
/// 1: Energy - Crude oil
/// 2: Energy - Natural gas
/// 3: Energy - Coal/Electricity
/// 4: Freight
/// 5: Base metals
/// 6: Precious metals
/// 7: Grains and oilseed
/// 8: Softs and other agriculturals
/// 9: Livestock and dairy
/// 10: Other commodity
/// 11: Carbon trading
pub const COMMODITY_RISK_WEIGHTS: &[(u8, f64)] = &[
    (1, 19.0),
    (2, 20.0),
    (3, 17.0),
    (4, 16.0),
    (5, 15.0),
    (6, 11.0),
    (7, 22.0),
    (8, 27.0),
    (9, 24.0),
    (10, 52.0),
    (11, 16.0),
];

/// Commodity intra-bucket correlation.
pub const COMMODITY_INTRA_BUCKET_CORRELATION: f64 = 0.55;

/// Commodity inter-bucket correlation.
pub const COMMODITY_INTER_BUCKET_CORRELATION: f64 = 0.20;

/// Commodity vega risk weight.
pub const COMMODITY_VEGA_RISK_WEIGHT: f64 = 0.55;

/// Commodity curvature risk weight scale.
pub const COMMODITY_CURVATURE_RISK_WEIGHT: f64 = 0.5;

/// Look up a commodity risk weight by bucket.
#[must_use]
pub fn commodity_risk_weight(bucket: u8) -> f64 {
    COMMODITY_RISK_WEIGHTS
        .iter()
        .find(|(b, _)| *b == bucket)
        .map(|(_, w)| *w)
        .unwrap_or(20.0) // Default for unmapped buckets
}
