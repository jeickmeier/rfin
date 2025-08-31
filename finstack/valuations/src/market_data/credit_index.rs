//! Credit index market data for CDS tranche pricing.
//!
//! Aggregates all market data components required for pricing instruments
//! on a specific credit index (e.g., CDX.NA.IG.42, iTraxx Europe).

use finstack_core::market_data::term_structures::{BaseCorrelationCurve, credit_curve::CreditCurve};
use finstack_core::prelude::*;
use finstack_core::F;
use std::collections::HashMap;
use std::sync::Arc;

/// Aggregated market data for a specific credit index.
///
/// Contains all the curves and parameters needed to price credit derivatives
/// on a standardized credit index using models like the Gaussian Copula.
#[derive(Clone, Debug)]
pub struct CreditIndexData {
    /// Number of constituents in the credit index (e.g., 125 for CDX IG)
    pub num_constituents: u16,
    /// Default recovery rate for the index (typically 40% for senior unsecured)
    pub recovery_rate: F,
    /// Credit curve for the index as a whole
    pub index_credit_curve: Arc<CreditCurve>,
    /// Base correlation curve mapping detachment points to correlations
    pub base_correlation_curve: Arc<BaseCorrelationCurve>,
    /// Optional individual credit curves for each constituent issuer
    /// Key is the issuer identifier (e.g., ticker or CUSIP)
    pub issuer_credit_curves: Option<HashMap<String, Arc<CreditCurve>>>,
}

impl CreditIndexData {
    /// Create a new credit index data builder.
    pub fn builder() -> CreditIndexDataBuilder {
        CreditIndexDataBuilder::new()
    }

    /// Get the credit curve for a specific issuer.
    /// 
    /// Returns the issuer-specific curve if available, otherwise falls back
    /// to the index curve (homogeneous portfolio assumption).
    pub fn get_issuer_curve(&self, issuer_id: &str) -> &CreditCurve {
        match &self.issuer_credit_curves {
            Some(curves) => curves.get(issuer_id)
                .map(|arc| arc.as_ref())
                .unwrap_or(self.index_credit_curve.as_ref()),
            None => self.index_credit_curve.as_ref(),
        }
    }

    /// Check if heterogeneous pricing mode is available.
    /// 
    /// Returns true if individual issuer curves are provided, enabling
    /// more granular portfolio loss modeling.
    pub fn has_issuer_curves(&self) -> bool {
        self.issuer_credit_curves.as_ref()
            .map(|curves| !curves.is_empty())
            .unwrap_or(false)
    }

    /// Get all available issuer identifiers.
    pub fn issuer_ids(&self) -> Vec<String> {
        match &self.issuer_credit_curves {
            Some(curves) => curves.keys().cloned().collect(),
            None => Vec::new(),
        }
    }
}

/// Builder for creating credit index data.
#[derive(Default)]
pub struct CreditIndexDataBuilder {
    num_constituents: Option<u16>,
    recovery_rate: Option<F>,
    index_credit_curve: Option<Arc<CreditCurve>>,
    base_correlation_curve: Option<Arc<BaseCorrelationCurve>>,
    issuer_credit_curves: Option<HashMap<String, Arc<CreditCurve>>>,
}

impl CreditIndexDataBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the number of constituents in the index.
    pub fn num_constituents(mut self, count: u16) -> Self {
        self.num_constituents = Some(count);
        self
    }

    /// Set the recovery rate (fraction between 0.0 and 1.0).
    pub fn recovery_rate(mut self, rate: F) -> Self {
        self.recovery_rate = Some(rate);
        self
    }

    /// Set the index-level credit curve.
    pub fn index_credit_curve(mut self, curve: Arc<CreditCurve>) -> Self {
        self.index_credit_curve = Some(curve);
        self
    }

    /// Set the base correlation curve.
    pub fn base_correlation_curve(mut self, curve: Arc<BaseCorrelationCurve>) -> Self {
        self.base_correlation_curve = Some(curve);
        self
    }

    /// Add issuer-specific credit curves for heterogeneous portfolio modeling.
    pub fn with_issuer_curves(mut self, curves: HashMap<String, Arc<CreditCurve>>) -> Self {
        self.issuer_credit_curves = Some(curves);
        self
    }

    /// Add a single issuer credit curve.
    pub fn add_issuer_curve(mut self, issuer_id: String, curve: Arc<CreditCurve>) -> Self {
        match &mut self.issuer_credit_curves {
            Some(curves) => {
                curves.insert(issuer_id, curve);
            }
            None => {
                let mut curves = HashMap::new();
                curves.insert(issuer_id, curve);
                self.issuer_credit_curves = Some(curves);
            }
        }
        self
    }

    /// Build the credit index data.
    pub fn build(self) -> Result<CreditIndexData> {
        let num_constituents = self.num_constituents
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        
        let recovery_rate = self.recovery_rate.unwrap_or(0.40); // Industry standard 40%
        
        let index_credit_curve = self.index_credit_curve
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        
        let base_correlation_curve = self.base_correlation_curve
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;

        // Validate recovery rate
        if !(0.0..=1.0).contains(&recovery_rate) {
            return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid));
        }

        // Validate number of constituents
        if num_constituents == 0 {
            return Err(finstack_core::Error::from(finstack_core::error::InputError::Invalid));
        }

        Ok(CreditIndexData {
            num_constituents,
            recovery_rate,
            index_credit_curve,
            base_correlation_curve,
            issuer_credit_curves: self.issuer_credit_curves,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::market_data::term_structures::{BaseCorrelationCurve, credit_curve::CreditCurve};
    use finstack_core::market_data::term_structures::credit_curve::Seniority;
    use finstack_core::dates::Date;
    use time::Month;

    fn sample_index_data() -> CreditIndexData {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        
        // Create sample index credit curve
        let index_curve = CreditCurve::builder("CDX.NA.IG.42")
            .issuer("CDX.NA.IG.42")
            .seniority(Seniority::Senior)
            .recovery_rate(0.40)
            .base_date(base_date)
            .spreads(vec![(1.0, 60.0), (3.0, 80.0), (5.0, 100.0), (7.0, 120.0), (10.0, 140.0)])
            .build()
            .unwrap();

        // Create sample base correlation curve
        let base_corr_curve = BaseCorrelationCurve::builder("CDX.NA.IG.42_5Y")
            .points(vec![
                (3.0, 0.25),   // 0-3% equity tranche
                (7.0, 0.45),   // 0-7% junior mezzanine
                (10.0, 0.60),  // 0-10% senior mezzanine
                (15.0, 0.75),  // 0-15% senior
                (30.0, 0.85),  // 0-30% super senior
            ])
            .build()
            .unwrap();

        CreditIndexData::builder()
            .num_constituents(125) // Standard CDX IG has 125 constituents
            .recovery_rate(0.40)   // Industry standard 40%
            .index_credit_curve(Arc::new(index_curve))
            .base_correlation_curve(Arc::new(base_corr_curve))
            .build()
            .unwrap()
    }

    #[test]
    fn test_credit_index_data_creation() {
        let data = sample_index_data();
        assert_eq!(data.num_constituents, 125);
        assert_eq!(data.recovery_rate, 0.40);
        assert!(!data.has_issuer_curves());
    }

    #[test]
    fn test_get_issuer_curve_fallback() {
        let data = sample_index_data();
        
        // Should return index curve for any issuer ID when no issuer curves available
        let curve = data.get_issuer_curve("AAPL");
        assert_eq!(curve.id().as_str(), "CDX.NA.IG.42");
    }

    #[test]
    fn test_with_issuer_curves() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        
        let index_curve = CreditCurve::builder("CDX.NA.IG.42")
            .issuer("CDX.NA.IG.42")
            .base_date(base_date)
            .spreads(vec![(1.0, 80.0), (5.0, 100.0)])
            .build()
            .unwrap();

        let aapl_curve = CreditCurve::builder("AAPL_SENIOR")
            .issuer("Apple Inc.")
            .seniority(Seniority::Senior)
            .base_date(base_date)
            .spreads(vec![(1.0, 50.0), (5.0, 70.0)])
            .build()
            .unwrap();

        let base_corr_curve = BaseCorrelationCurve::builder("CDX.NA.IG.42_5Y")
            .points(vec![(3.0, 0.25), (10.0, 0.60)])
            .build()
            .unwrap();

        let data = CreditIndexData::builder()
            .num_constituents(125)
            .index_credit_curve(Arc::new(index_curve))
            .base_correlation_curve(Arc::new(base_corr_curve))
            .add_issuer_curve("AAPL".to_string(), Arc::new(aapl_curve))
            .build()
            .unwrap();

        assert!(data.has_issuer_curves());
        assert_eq!(data.issuer_ids(), vec!["AAPL"]);
        
        let aapl_curve = data.get_issuer_curve("AAPL");
        assert_eq!(aapl_curve.issuer, "Apple Inc.");
        
        // Non-existent issuer should fall back to index curve
        let unknown_curve = data.get_issuer_curve("UNKNOWN");
        assert_eq!(unknown_curve.id().as_str(), "CDX.NA.IG.42");
    }

    #[test]
    fn test_builder_validation() {
        // Test missing required fields
        let result = CreditIndexData::builder().build();
        assert!(result.is_err());

        // Test invalid recovery rate
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let curve = CreditCurve::builder("TEST")
            .base_date(base_date)
            .spreads(vec![(1.0, 100.0)])
            .build()
            .unwrap();
        let base_corr = BaseCorrelationCurve::builder("TEST")
            .points(vec![(3.0, 0.25), (10.0, 0.60)])
            .build()
            .unwrap();

        let result = CreditIndexData::builder()
            .num_constituents(125)
            .recovery_rate(1.5) // Invalid > 1.0
            .index_credit_curve(Arc::new(curve))
            .base_correlation_curve(Arc::new(base_corr))
            .build();
        assert!(result.is_err());
    }
}
