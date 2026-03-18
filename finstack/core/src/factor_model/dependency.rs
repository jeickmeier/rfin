use crate::currency::Currency;
use crate::types::CurveId;
use serde::{Deserialize, Serialize};

/// Classification of a curve dependency's role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CurveType {
    /// Discounting curve.
    Discount,
    /// Forward projection curve.
    Forward,
    /// Credit or hazard curve.
    Hazard,
    /// Inflation curve.
    Inflation,
    /// Base-correlation surface-backed curve.
    BaseCorrelation,
}

/// A single market dependency extracted from an instrument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketDependency {
    /// Discount, forward, or other rate curve.
    Curve {
        /// Curve identifier.
        id: CurveId,
        /// Role played by the curve.
        curve_type: CurveType,
    },
    /// Credit or hazard curve.
    CreditCurve {
        /// Curve identifier.
        id: CurveId,
    },
    /// Equity or commodity spot.
    Spot {
        /// Spot identifier or ticker.
        id: String,
    },
    /// Volatility surface identifier.
    VolSurface {
        /// Surface identifier.
        id: String,
    },
    /// FX pair dependency.
    FxPair {
        /// Base currency.
        base: Currency,
        /// Quote currency.
        quote: Currency,
    },
    /// Time-series dependency.
    Series {
        /// Series identifier.
        id: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_dependency_curve() {
        let dep = MarketDependency::Curve {
            id: CurveId::new("USD-OIS"),
            curve_type: CurveType::Discount,
        };

        match &dep {
            MarketDependency::Curve { id, curve_type } => {
                assert_eq!(id.as_ref(), "USD-OIS");
                assert_eq!(*curve_type, CurveType::Discount);
            }
            _ => unreachable!("expected curve dependency"),
        }
    }

    #[test]
    fn test_market_dependency_credit_curve() {
        let dep = MarketDependency::CreditCurve {
            id: CurveId::new("ACME-HAZARD"),
        };

        match &dep {
            MarketDependency::CreditCurve { id } => {
                assert_eq!(id.as_ref(), "ACME-HAZARD");
            }
            _ => unreachable!("expected credit curve dependency"),
        }
    }

    #[test]
    fn test_market_dependency_serde_roundtrip() {
        let dep = MarketDependency::Spot {
            id: "AAPL".to_string(),
        };
        let json_result = serde_json::to_string(&dep);
        assert!(json_result.is_ok());
        let Ok(json) = json_result else {
            return;
        };

        let back_result: Result<MarketDependency, _> = serde_json::from_str(&json);
        assert!(back_result.is_ok());
        let Ok(back) = back_result else {
            return;
        };
        assert_eq!(dep, back);
    }

    #[test]
    fn test_curve_type_all_variants_serde() {
        for curve_type in [
            CurveType::Discount,
            CurveType::Forward,
            CurveType::Hazard,
            CurveType::Inflation,
            CurveType::BaseCorrelation,
        ] {
            let json_result = serde_json::to_string(&curve_type);
            assert!(json_result.is_ok());
            let Ok(json) = json_result else {
                return;
            };

            let back_result: Result<CurveType, _> = serde_json::from_str(&json);
            assert!(back_result.is_ok());
            let Ok(back) = back_result else {
                return;
            };
            assert_eq!(curve_type, back);
        }
    }
}
