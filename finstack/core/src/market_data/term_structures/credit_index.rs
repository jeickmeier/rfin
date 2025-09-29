//! Credit index market data for CDS tranche pricing.
//!
//! Aggregates market data components required for pricing instruments on a
//! standardized credit index (e.g., CDX.NA.IG.42, iTraxx Europe).

use super::{hazard_curve::HazardCurve, BaseCorrelationCurve};
use crate::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

/// Aggregated market data for a specific credit index.
///
/// Contains all the curves and parameters needed to price credit derivatives
/// on a standardized credit index using models like the Gaussian Copula.
///
/// Note: This struct contains Arc-wrapped curves that cannot be directly serialized.
/// For persistence, extract and serialize the underlying curve data separately.
#[derive(Clone, Debug)]
pub struct CreditIndexData {
    /// Number of constituents in the credit index (e.g., 125 for CDX IG)
    pub num_constituents: u16,
    /// Default recovery rate for the index (typically 40% for senior unsecured)
    pub recovery_rate: f64,
    /// Hazard curve for the index as a whole
    pub index_credit_curve: Arc<HazardCurve>,
    /// Base correlation curve mapping detachment points to correlations
    pub base_correlation_curve: Arc<BaseCorrelationCurve>,
    /// Optional individual hazard curves for each constituent issuer
    /// Key is the issuer identifier (e.g., ticker or CUSIP)
    pub issuer_credit_curves: Option<HashMap<String, Arc<HazardCurve>>>,
}

impl CreditIndexData {
    /// Create a new credit index data builder.
    pub fn builder() -> CreditIndexDataBuilder {
        CreditIndexDataBuilder::default()
    }

    /// Get the credit curve for a specific issuer.
    ///
    /// Returns the issuer-specific curve if available, otherwise falls back
    /// to the index curve (homogeneous portfolio assumption).
    pub fn get_issuer_curve(&self, issuer_id: &str) -> &HazardCurve {
        match &self.issuer_credit_curves {
            Some(curves) => curves
                .get(issuer_id)
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
        self.issuer_credit_curves
            .as_ref()
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
///
/// The builder collects index-wide metadata (constituent count, recovery) and
/// attaches the market curves required for tranche pricing.
///
/// # Examples
/// ```rust
/// use finstack_core::market_data::term_structures::{
///     credit_index::CreditIndexData,
///     hazard_curve::HazardCurve,
///     base_correlation::BaseCorrelationCurve,
/// };
/// use finstack_core::dates::Date;
/// use std::sync::Arc;
/// use time::Month;
///
/// let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
/// let hazard = Arc::new(
///     HazardCurve::builder("CDX")
///         .base_date(base)
///         .knots([(0.0, 0.01), (5.0, 0.015)])
///         .build()
///         .unwrap(),
/// );
/// let base_corr = Arc::new(
///     BaseCorrelationCurve::builder("CDX")
///         .points([(3.0, 0.25), (10.0, 0.55)])
///         .build()
///         .unwrap(),
/// );
/// let index = CreditIndexData::builder()
///     .num_constituents(125)
///     .recovery_rate(0.4)
///     .index_credit_curve(hazard)
///     .base_correlation_curve(base_corr)
///     .build()
///     .unwrap();
/// assert_eq!(index.num_constituents, 125);
/// ```
#[derive(Default)]
pub struct CreditIndexDataBuilder {
    num_constituents: Option<u16>,
    recovery_rate: Option<f64>,
    index_credit_curve: Option<Arc<HazardCurve>>,
    base_correlation_curve: Option<Arc<BaseCorrelationCurve>>,
    issuer_credit_curves: Option<HashMap<String, Arc<HazardCurve>>>,
}

impl CreditIndexDataBuilder {
    /// Set the number of constituents in the index.
    pub fn num_constituents(mut self, count: u16) -> Self {
        self.num_constituents = Some(count);
        self
    }

    /// Set the recovery rate (fraction between 0.0 and 1.0).
    pub fn recovery_rate(mut self, rate: f64) -> Self {
        self.recovery_rate = Some(rate);
        self
    }

    /// Set the index-level credit curve.
    pub fn index_credit_curve(mut self, curve: Arc<HazardCurve>) -> Self {
        self.index_credit_curve = Some(curve);
        self
    }

    /// Set the base correlation curve.
    pub fn base_correlation_curve(mut self, curve: Arc<BaseCorrelationCurve>) -> Self {
        self.base_correlation_curve = Some(curve);
        self
    }

    /// Add issuer-specific credit curves for heterogeneous portfolio modeling.
    pub fn with_issuer_curves(mut self, curves: HashMap<String, Arc<HazardCurve>>) -> Self {
        self.issuer_credit_curves = Some(curves);
        self
    }

    /// Add a single issuer credit curve.
    pub fn add_issuer_curve(mut self, issuer_id: String, curve: Arc<HazardCurve>) -> Self {
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
        let num_constituents = self
            .num_constituents
            .ok_or_else(|| crate::Error::from(crate::error::InputError::Invalid))?;

        let recovery_rate = self.recovery_rate.unwrap_or(0.40);

        let index_credit_curve = self
            .index_credit_curve
            .ok_or_else(|| crate::Error::from(crate::error::InputError::Invalid))?;

        let base_correlation_curve = self
            .base_correlation_curve
            .ok_or_else(|| crate::Error::from(crate::error::InputError::Invalid))?;

        // Validate recovery rate
        if !(0.0..=1.0).contains(&recovery_rate) {
            return Err(crate::Error::from(crate::error::InputError::Invalid));
        }

        // Validate number of constituents
        if num_constituents == 0 {
            return Err(crate::Error::from(crate::error::InputError::Invalid));
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
