//! Traits for marginable instruments.
//!
//! Defines the common interface for instruments that support margin calculations,
//! enabling uniform margin metric calculation and portfolio aggregation.

use crate::instruments::common::traits::Instrument;
use crate::margin::types::OtcMarginSpec;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use finstack_core::collections::HashMap;

/// Risk classes for SIMM categorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SimmRiskClass {
    /// Interest rate risk
    InterestRate,
    /// Credit qualifying (investment grade)
    CreditQualifying,
    /// Credit non-qualifying (high yield, emerging markets)
    CreditNonQualifying,
    /// Equity risk
    Equity,
    /// Commodity risk
    Commodity,
    /// Foreign exchange risk
    Fx,
}

impl std::fmt::Display for SimmRiskClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimmRiskClass::InterestRate => write!(f, "InterestRate"),
            SimmRiskClass::CreditQualifying => write!(f, "CreditQualifying"),
            SimmRiskClass::CreditNonQualifying => write!(f, "CreditNonQualifying"),
            SimmRiskClass::Equity => write!(f, "Equity"),
            SimmRiskClass::Commodity => write!(f, "Commodity"),
            SimmRiskClass::Fx => write!(f, "FX"),
        }
    }
}

/// SIMM sensitivity inputs organized by risk class.
///
/// Contains the risk sensitivities needed for ISDA SIMM calculation.
/// Sensitivities are organized by risk class and further bucketed
/// according to SIMM specifications.
///
/// # Example
///
/// ```rust,no_run
/// use finstack_valuations::margin::SimmSensitivities;
/// use finstack_core::currency::Currency;
///
/// let mut sensitivities = SimmSensitivities::new(Currency::USD);
///
/// // Add IR delta sensitivities by tenor
/// sensitivities.add_ir_delta(Currency::USD, "2Y", 15_000.0);
/// sensitivities.add_ir_delta(Currency::USD, "5Y", 45_000.0);
/// sensitivities.add_ir_delta(Currency::USD, "10Y", 25_000.0);
///
/// // Add credit delta
/// sensitivities.add_credit_delta("CDX.NA.IG", true, "5Y", 50_000.0);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SimmSensitivities {
    /// Base currency for the sensitivities
    pub base_currency: Currency,

    /// Interest rate delta by (currency, tenor bucket).
    ///
    /// Tenor buckets follow SIMM specification: 2W, 1M, 3M, 6M, 1Y, 2Y, 3Y, 5Y, 10Y, 15Y, 20Y, 30Y
    pub ir_delta: HashMap<(Currency, String), f64>,

    /// Interest rate vega by (currency, tenor bucket).
    pub ir_vega: HashMap<(Currency, String), f64>,

    /// Credit qualifying delta by (issuer/index, tenor bucket).
    ///
    /// For single-name CDS and investment-grade indices.
    pub credit_qualifying_delta: HashMap<(String, String), f64>,

    /// Credit non-qualifying delta by (issuer/index, tenor bucket).
    ///
    /// For high-yield, distressed, and emerging market credit.
    pub credit_non_qualifying_delta: HashMap<(String, String), f64>,

    /// Equity delta by underlier.
    pub equity_delta: HashMap<String, f64>,

    /// Equity vega by underlier.
    pub equity_vega: HashMap<String, f64>,

    /// FX delta by currency (sensitivity to USD exchange rate).
    pub fx_delta: HashMap<Currency, f64>,

    /// FX vega by currency pair.
    pub fx_vega: HashMap<(Currency, Currency), f64>,

    /// Commodity delta by bucket.
    pub commodity_delta: HashMap<String, f64>,

    /// Curvature risk by risk class.
    pub curvature: HashMap<SimmRiskClass, f64>,
}

impl SimmSensitivities {
    /// Create new empty sensitivities for a base currency.
    #[must_use]
    pub fn new(base_currency: Currency) -> Self {
        Self {
            base_currency,
            ir_delta: HashMap::default(),
            ir_vega: HashMap::default(),
            credit_qualifying_delta: HashMap::default(),
            credit_non_qualifying_delta: HashMap::default(),
            equity_delta: HashMap::default(),
            equity_vega: HashMap::default(),
            fx_delta: HashMap::default(),
            fx_vega: HashMap::default(),
            commodity_delta: HashMap::default(),
            curvature: HashMap::default(),
        }
    }

    /// Add interest rate delta sensitivity.
    pub fn add_ir_delta(&mut self, currency: Currency, tenor: impl Into<String>, delta: f64) {
        let key = (currency, tenor.into());
        *self.ir_delta.entry(key).or_insert(0.0) += delta;
    }

    /// Add interest rate vega sensitivity.
    pub fn add_ir_vega(&mut self, currency: Currency, tenor: impl Into<String>, vega: f64) {
        let key = (currency, tenor.into());
        *self.ir_vega.entry(key).or_insert(0.0) += vega;
    }

    /// Add credit delta sensitivity.
    ///
    /// # Arguments
    /// * `name` - Issuer or index name
    /// * `qualifying` - True for investment grade, false for high yield/EM
    /// * `tenor` - Tenor bucket (e.g., "5Y")
    /// * `delta` - Sensitivity amount
    pub fn add_credit_delta(
        &mut self,
        name: impl Into<String>,
        qualifying: bool,
        tenor: impl Into<String>,
        delta: f64,
    ) {
        let key = (name.into(), tenor.into());
        if qualifying {
            *self.credit_qualifying_delta.entry(key).or_insert(0.0) += delta;
        } else {
            *self.credit_non_qualifying_delta.entry(key).or_insert(0.0) += delta;
        }
    }

    /// Add equity delta sensitivity.
    pub fn add_equity_delta(&mut self, underlier: impl Into<String>, delta: f64) {
        let key = underlier.into();
        *self.equity_delta.entry(key).or_insert(0.0) += delta;
    }

    /// Add equity vega sensitivity.
    pub fn add_equity_vega(&mut self, underlier: impl Into<String>, vega: f64) {
        let key = underlier.into();
        *self.equity_vega.entry(key).or_insert(0.0) += vega;
    }

    /// Add FX delta sensitivity.
    pub fn add_fx_delta(&mut self, currency: Currency, delta: f64) {
        *self.fx_delta.entry(currency).or_insert(0.0) += delta;
    }

    /// Check if sensitivities are empty.
    ///
    /// Returns true if no sensitivity buckets exist across any risk class.
    /// Note: This checks bucket existence, not whether net sensitivities are zero.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ir_delta.is_empty()
            && self.ir_vega.is_empty()
            && self.credit_qualifying_delta.is_empty()
            && self.credit_non_qualifying_delta.is_empty()
            && self.equity_delta.is_empty()
            && self.equity_vega.is_empty()
            && self.fx_delta.is_empty()
            && self.fx_vega.is_empty()
            && self.commodity_delta.is_empty()
            && self.curvature.is_empty()
    }

    /// Merge another set of sensitivities into this one.
    ///
    /// Sensitivities are added together, enabling risk offsetting within a netting set.
    pub fn merge(&mut self, other: &SimmSensitivities) {
        for (key, &value) in &other.ir_delta {
            *self.ir_delta.entry(key.clone()).or_insert(0.0) += value;
        }
        for (key, &value) in &other.ir_vega {
            *self.ir_vega.entry(key.clone()).or_insert(0.0) += value;
        }
        for (key, &value) in &other.credit_qualifying_delta {
            *self
                .credit_qualifying_delta
                .entry(key.clone())
                .or_insert(0.0) += value;
        }
        for (key, &value) in &other.credit_non_qualifying_delta {
            *self
                .credit_non_qualifying_delta
                .entry(key.clone())
                .or_insert(0.0) += value;
        }
        for (key, &value) in &other.equity_delta {
            *self.equity_delta.entry(key.clone()).or_insert(0.0) += value;
        }
        for (key, &value) in &other.equity_vega {
            *self.equity_vega.entry(key.clone()).or_insert(0.0) += value;
        }
        for (&key, &value) in &other.fx_delta {
            *self.fx_delta.entry(key).or_insert(0.0) += value;
        }
        for (&key, &value) in &other.fx_vega {
            *self.fx_vega.entry(key).or_insert(0.0) += value;
        }
        for (key, &value) in &other.commodity_delta {
            *self.commodity_delta.entry(key.clone()).or_insert(0.0) += value;
        }
        for (&key, &value) in &other.curvature {
            *self.curvature.entry(key).or_insert(0.0) += value;
        }
    }

    /// Get total IR delta across all currencies and tenors.
    #[must_use]
    pub fn total_ir_delta(&self) -> f64 {
        self.ir_delta.values().sum()
    }

    /// Get total credit delta (qualifying + non-qualifying).
    #[must_use]
    pub fn total_credit_delta(&self) -> f64 {
        self.credit_qualifying_delta.values().sum::<f64>()
            + self.credit_non_qualifying_delta.values().sum::<f64>()
    }

    /// Get total equity delta.
    #[must_use]
    pub fn total_equity_delta(&self) -> f64 {
        self.equity_delta.values().sum()
    }
}

/// Identifies a margin netting set.
///
/// Instruments in the same netting set can offset each other for margin
/// calculation purposes. The netting set is typically defined by the
/// CSA agreement or CCP membership.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NettingSetId {
    /// Counterparty identifier
    pub counterparty_id: String,
    /// CSA identifier (for bilateral trades)
    pub csa_id: Option<String>,
    /// CCP identifier (for cleared trades)
    pub ccp_id: Option<String>,
}

impl NettingSetId {
    /// Create a bilateral netting set ID.
    #[must_use]
    pub fn bilateral(counterparty_id: impl Into<String>, csa_id: impl Into<String>) -> Self {
        Self {
            counterparty_id: counterparty_id.into(),
            csa_id: Some(csa_id.into()),
            ccp_id: None,
        }
    }

    /// Create a cleared netting set ID.
    #[must_use]
    pub fn cleared(ccp_id: impl Into<String>) -> Self {
        let ccp_string = ccp_id.into();
        Self {
            counterparty_id: ccp_string.clone(),
            csa_id: None,
            ccp_id: Some(ccp_string),
        }
    }

    /// Check if this is a cleared netting set.
    #[must_use]
    pub fn is_cleared(&self) -> bool {
        self.ccp_id.is_some()
    }
}

impl std::fmt::Display for NettingSetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ccp) = &self.ccp_id {
            write!(f, "CCP:{}", ccp)
        } else if let Some(csa) = &self.csa_id {
            write!(f, "{}:{}", self.counterparty_id, csa)
        } else {
            write!(f, "{}", self.counterparty_id)
        }
    }
}

/// Trait for instruments that support margin calculations.
///
/// Implements this trait for instruments that can have margin requirements,
/// enabling uniform calculation of IM and VM across different instrument types.
///
/// # Implementors
///
/// - [`InterestRateSwap`] - OTC interest rate derivatives
/// - [`CreditDefaultSwap`] - OTC credit derivatives
/// - [`CDSIndex`] - Credit index derivatives
/// - [`EquityTotalReturnSwap`] - Equity TRS
/// - [`FIIndexTotalReturnSwap`] - Fixed income TRS
/// - [`Repo`] - Repurchase agreements
pub trait Marginable: Instrument {
    /// Get the margin specification for this instrument.
    ///
    /// Returns `None` if the instrument has no margin requirements configured.
    fn margin_spec(&self) -> Option<&OtcMarginSpec>;

    /// Get the repo margin specification (for repos only).
    ///
    /// Default implementation returns `None`. Override for repo instruments.
    fn repo_margin_spec(&self) -> Option<&crate::instruments::repo::RepoMarginSpec> {
        None
    }

    /// Get the netting set identifier for margin aggregation.
    ///
    /// Instruments in the same netting set can offset each other.
    /// Returns `None` if the instrument is not part of a netting set.
    fn netting_set_id(&self) -> Option<NettingSetId>;

    /// Calculate SIMM sensitivities for this instrument.
    ///
    /// Returns the risk sensitivities needed for ISDA SIMM calculation.
    /// The sensitivities are used to calculate initial margin.
    ///
    /// # Arguments
    /// * `market` - Market data context
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    /// SIMM sensitivities or error if calculation fails
    fn simm_sensitivities(&self, market: &MarketContext, as_of: Date) -> Result<SimmSensitivities>;

    /// Get the current mark-to-market value for VM calculation.
    ///
    /// This is typically the NPV of the instrument.
    fn mtm_for_vm(&self, market: &MarketContext, as_of: Date) -> Result<Money>;

    /// Check if margin is applicable for this instrument.
    ///
    /// Returns true if the instrument has margin requirements.
    fn has_margin(&self) -> bool {
        self.margin_spec().is_some() || self.repo_margin_spec().is_some()
    }
}

/// Result of calculating margin for an instrument.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InstrumentMarginResult {
    /// Instrument identifier
    pub instrument_id: String,
    /// Calculation date
    pub as_of: Date,
    /// Initial margin requirement
    pub initial_margin: Money,
    /// Variation margin requirement (can be negative = return)
    pub variation_margin: Money,
    /// Total margin requirement (IM + VM if positive)
    pub total_margin: Money,
    /// IM calculation methodology used
    pub im_methodology: crate::margin::types::ImMethodology,
    /// Whether instrument is cleared or bilateral
    pub is_cleared: bool,
    /// Netting set identifier
    pub netting_set: Option<NettingSetId>,
    /// SIMM sensitivities (if SIMM was used)
    pub sensitivities: Option<SimmSensitivities>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simm_sensitivities_creation() {
        let mut sens = SimmSensitivities::new(Currency::USD);
        assert!(sens.is_empty());

        sens.add_ir_delta(Currency::USD, "5Y", 100_000.0);
        sens.add_ir_delta(Currency::USD, "10Y", 50_000.0);
        sens.add_credit_delta("ACME_CORP", true, "5Y", 25_000.0);

        assert!(!sens.is_empty());
        assert_eq!(sens.total_ir_delta(), 150_000.0);
        assert_eq!(sens.total_credit_delta(), 25_000.0);
    }

    #[test]
    fn test_simm_sensitivities_merge() {
        let mut sens1 = SimmSensitivities::new(Currency::USD);
        sens1.add_ir_delta(Currency::USD, "5Y", 100_000.0);

        let mut sens2 = SimmSensitivities::new(Currency::USD);
        sens2.add_ir_delta(Currency::USD, "5Y", 50_000.0);
        sens2.add_ir_delta(Currency::USD, "10Y", 25_000.0);

        sens1.merge(&sens2);

        assert_eq!(
            sens1.ir_delta.get(&(Currency::USD, "5Y".to_string())),
            Some(&150_000.0)
        );
        assert_eq!(
            sens1.ir_delta.get(&(Currency::USD, "10Y".to_string())),
            Some(&25_000.0)
        );
    }

    #[test]
    fn test_netting_set_id() {
        let bilateral = NettingSetId::bilateral("COUNTERPARTY_A", "CSA_001");
        assert!(!bilateral.is_cleared());
        assert_eq!(bilateral.to_string(), "COUNTERPARTY_A:CSA_001");

        let cleared = NettingSetId::cleared("LCH");
        assert!(cleared.is_cleared());
        assert_eq!(cleared.to_string(), "CCP:LCH");
    }

    #[test]
    fn test_simm_risk_class_display() {
        assert_eq!(SimmRiskClass::InterestRate.to_string(), "InterestRate");
        assert_eq!(
            SimmRiskClass::CreditQualifying.to_string(),
            "CreditQualifying"
        );
        assert_eq!(SimmRiskClass::Fx.to_string(), "FX");
    }
}
