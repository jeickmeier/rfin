//! SIMM risk classification and sensitivity types.
//!
//! Types for ISDA SIMM categorization and sensitivity inputs,
//! used by the [`Marginable`](crate::traits::Marginable) trait
//! and SIMM calculator.

use finstack_core::currency::Currency;
use finstack_core::HashMap;

/// Risk classes for SIMM categorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
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

/// SIMM credit sector for bucket assignment.
///
/// Maps reference entities to ISDA SIMM credit qualifying buckets.
/// See ISDA SIMM v2.6 Table 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum SimmCreditSector {
    /// Bucket 1: Sovereigns including central banks
    Sovereign,
    /// Bucket 2: Financials (banks, insurance, broker-dealers)
    Financial,
    /// Bucket 3: Basic materials / energy / industrials
    BasicMaterials,
    /// Bucket 4: Consumer goods / services
    ConsumerGoods,
    /// Bucket 5: Technology / media / telecoms
    TechnologyMedia,
    /// Bucket 6: Health care / utilities
    HealthCare,
    /// Bucket 7: Indices (CDX.NA.IG, iTraxx, etc.)
    Index,
    /// Bucket 8: Covered bonds / securitized
    Securitized,
    /// Residual bucket
    Residual,
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
/// use finstack_margin::SimmSensitivities;
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
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
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
    fn test_simm_risk_class_display() {
        assert_eq!(SimmRiskClass::InterestRate.to_string(), "InterestRate");
        assert_eq!(
            SimmRiskClass::CreditQualifying.to_string(),
            "CreditQualifying"
        );
        assert_eq!(SimmRiskClass::Fx.to_string(), "FX");
    }
}
