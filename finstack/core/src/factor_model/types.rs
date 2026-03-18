use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a risk factor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FactorId(String);

impl FactorId {
    /// Create a factor identifier from any string-like value.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the underlying factor identifier string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FactorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Broad classification of a risk factor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FactorType {
    /// Interest-rate factor.
    Rates,
    /// Credit-spread or hazard factor.
    Credit,
    /// Equity price factor.
    Equity,
    /// Foreign-exchange factor.
    FX,
    /// Volatility factor.
    Volatility,
    /// Commodity factor.
    Commodity,
    /// Inflation factor.
    Inflation,
    /// User-defined factor bucket.
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factor_id_from_string() {
        let id = FactorId::new("USD-Rates");
        assert_eq!(id.as_str(), "USD-Rates");
    }

    #[test]
    fn test_factor_id_equality() {
        let a = FactorId::new("USD-Rates");
        let b = FactorId::new("USD-Rates");
        assert_eq!(a, b);
    }

    #[test]
    fn test_factor_id_display() {
        let id = FactorId::new("NA-Energy-CCC");
        assert_eq!(format!("{id}"), "NA-Energy-CCC");
    }

    #[test]
    fn test_factor_id_serde_roundtrip() {
        let id = FactorId::new("USD-Rates");
        let json_result = serde_json::to_string(&id);
        assert!(json_result.is_ok());
        let Ok(json) = json_result else {
            return;
        };
        assert_eq!(json, "\"USD-Rates\"");

        let back_result: Result<FactorId, _> = serde_json::from_str(&json);
        assert!(back_result.is_ok());
        let Ok(back) = back_result else {
            return;
        };
        assert_eq!(id, back);
    }

    #[test]
    fn test_factor_type_serde() {
        let ft = FactorType::Credit;
        let json_result = serde_json::to_string(&ft);
        assert!(json_result.is_ok());
        let Ok(json) = json_result else {
            return;
        };

        let back_result: Result<FactorType, _> = serde_json::from_str(&json);
        assert!(back_result.is_ok());
        let Ok(back) = back_result else {
            return;
        };
        assert_eq!(ft, back);
    }

    #[test]
    fn test_factor_type_custom() {
        let ft = FactorType::Custom("Weather".into());
        let json_result = serde_json::to_string(&ft);
        assert!(json_result.is_ok());
        let Ok(json) = json_result else {
            return;
        };

        let back_result: Result<FactorType, _> = serde_json::from_str(&json);
        assert!(back_result.is_ok());
        let Ok(back) = back_result else {
            return;
        };
        assert_eq!(ft, back);
    }
}
