//! ISDA Standard Initial Margin Model (SIMM) calculator.
//!
//! Implements the ISDA SIMM methodology for calculating initial margin
//! on non-centrally cleared OTC derivatives.
//!
//! # ISDA SIMM Methodology
//!
//! SIMM calculates IM based on sensitivities across risk classes:
//! - Interest Rate (IR): DV01 by tenor bucket
//! - Credit Qualifying (CQ): CS01 for investment grade
//! - Credit Non-Qualifying (CNQ): CS01 for high yield
//! - Equity: Delta sensitivities
//! - Commodity: Delta sensitivities
//! - FX: Delta sensitivities
//!
//! # Formula
//!
//! ```text
//! IM = sqrt(sum_i sum_j ρ_ij × K_i × K_j)
//! ```
//!
//! Where K_i is the risk-weighted sensitivity for bucket i.
//!
//! > **Implementation note:** `calculate_from_sensitivities` applies the SIMM
//! > risk-class correlation matrix (delta-only) but does not implement the
//! > full SIMM bucket/tenor correlations, vega, or curvature aggregation.

use crate::instruments::common::traits::Instrument;
use crate::margin::calculators::traits::{ImCalculator, ImResult};
use crate::margin::traits::{SimmRiskClass, SimmSensitivities};
use crate::margin::types::ImMethodology;
use finstack_core::collections::HashMap;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// SIMM version identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SimmVersion {
    /// SIMM v2.5 (2022)
    V2_5,
    /// SIMM v2.6 (2023)
    #[default]
    V2_6,
}

impl std::fmt::Display for SimmVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimmVersion::V2_5 => write!(f, "SIMM v2.5"),
            SimmVersion::V2_6 => write!(f, "SIMM v2.6"),
        }
    }
}

/// SIMM risk weights by version.
///
/// Contains calibrated risk weights for delta, vega, and curvature
/// across all risk classes.
#[derive(Debug, Clone)]
pub struct SimmRiskWeights {
    /// Version of the risk weights
    pub version: SimmVersion,

    /// Interest rate delta risk weights by tenor bucket (years)
    /// Tenors: 2w, 1m, 3m, 6m, 1y, 2y, 3y, 5y, 10y, 15y, 20y, 30y
    pub ir_delta_weights: HashMap<String, f64>,

    /// Credit qualifying delta risk weights by rating bucket
    pub cq_delta_weights: HashMap<String, f64>,

    /// Credit non-qualifying delta risk weights
    pub cnq_delta_weight: f64,

    /// Equity delta risk weight
    pub equity_delta_weight: f64,

    /// FX delta risk weight
    pub fx_delta_weight: f64,
}

impl Default for SimmRiskWeights {
    fn default() -> Self {
        Self::v2_6()
    }
}

impl SimmRiskWeights {
    /// SIMM v2.6 (2023) risk weights.
    #[must_use]
    pub fn v2_6() -> Self {
        let mut ir_delta_weights = HashMap::default();
        // SIMM v2.6 IR risk weights (USD as reference)
        ir_delta_weights.insert("2w".to_string(), 109.0);
        ir_delta_weights.insert("1m".to_string(), 105.0);
        ir_delta_weights.insert("3m".to_string(), 80.0);
        ir_delta_weights.insert("6m".to_string(), 67.0);
        ir_delta_weights.insert("1y".to_string(), 61.0);
        ir_delta_weights.insert("2y".to_string(), 52.0);
        ir_delta_weights.insert("3y".to_string(), 49.0);
        ir_delta_weights.insert("5y".to_string(), 51.0);
        ir_delta_weights.insert("10y".to_string(), 51.0);
        ir_delta_weights.insert("15y".to_string(), 51.0);
        ir_delta_weights.insert("20y".to_string(), 54.0);
        ir_delta_weights.insert("30y".to_string(), 62.0);

        let mut cq_delta_weights = HashMap::default();
        // Credit qualifying risk weights by sector
        cq_delta_weights.insert("sovereigns".to_string(), 85.0);
        cq_delta_weights.insert("financials".to_string(), 85.0);
        cq_delta_weights.insert("corporates".to_string(), 73.0);

        Self {
            version: SimmVersion::V2_6,
            ir_delta_weights,
            cq_delta_weights,
            cnq_delta_weight: 500.0, // High yield / non-qualifying
            equity_delta_weight: 32.0,
            fx_delta_weight: 8.4,
        }
    }

    /// SIMM v2.5 (2022) risk weights.
    #[must_use]
    pub fn v2_5() -> Self {
        // Slightly different calibration
        let mut weights = Self::v2_6();
        weights.version = SimmVersion::V2_5;
        // V2.5 had slightly lower IR weights
        weights.ir_delta_weights.insert("1y".to_string(), 59.0);
        weights.ir_delta_weights.insert("5y".to_string(), 48.0);
        weights
    }
}

/// ISDA SIMM calculator.
///
/// Calculates initial margin using the ISDA Standard Initial Margin Model.
/// This is the industry standard methodology for bilateral OTC derivatives.
///
/// # Usage
///
/// The calculator uses instrument sensitivities (DV01, CS01, etc.) to compute
/// risk-weighted margin amounts across all SIMM risk classes.
///
/// # Example
///
/// ```rust,no_run
/// use finstack_valuations::instruments::Instrument;
/// use finstack_valuations::margin::{ImCalculator, SimmCalculator};
/// use finstack_core::dates::Date;
/// use finstack_core::market_data::context::MarketContext;
/// use time::macros::date;
///
/// # fn main() -> finstack_core::Result<()> {
/// let calc = SimmCalculator::default();
/// # let swap: &dyn Instrument = todo!("provide a swap instrument");
/// # let context = MarketContext::new();
/// # let as_of: Date = date!(2025-01-01);
/// let im = calc.calculate(swap, &context, as_of)?;
///
/// println!("SIMM IM: {}", im.amount);
/// for (risk_class, amount) in &im.breakdown {
///     println!("  {}: {}", risk_class, amount);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct SimmCalculator {
    /// SIMM version
    pub version: SimmVersion,
    /// Risk weights
    pub risk_weights: SimmRiskWeights,
    /// Margin period of risk (days)
    pub mpor_days: u32,
}

impl Default for SimmCalculator {
    fn default() -> Self {
        Self::new(SimmVersion::V2_6)
    }
}

impl SimmCalculator {
    /// Create a new SIMM calculator with the specified version.
    #[must_use]
    pub fn new(version: SimmVersion) -> Self {
        let risk_weights = match version {
            SimmVersion::V2_5 => SimmRiskWeights::v2_5(),
            SimmVersion::V2_6 => SimmRiskWeights::v2_6(),
        };

        Self {
            version,
            risk_weights,
            mpor_days: 10,
        }
    }

    /// Set margin period of risk.
    #[must_use]
    pub fn with_mpor(mut self, days: u32) -> Self {
        self.mpor_days = days;
        self
    }

    /// Calculate IR delta margin from DV01 sensitivities.
    ///
    /// # Arguments
    ///
    /// * `dv01_by_tenor` - Map of tenor bucket to DV01 sensitivity
    pub fn calculate_ir_delta(&self, dv01_by_tenor: &HashMap<String, f64>) -> f64 {
        let mut weighted_sum = 0.0;

        for (tenor, dv01) in dv01_by_tenor {
            if let Some(&weight) = self.risk_weights.ir_delta_weights.get(tenor) {
                // Risk weight is in bp, DV01 is in currency units
                weighted_sum += (dv01 * weight).powi(2);
            }
        }

        weighted_sum.sqrt()
    }

    /// Calculate credit delta margin from CS01 sensitivities.
    ///
    /// # Arguments
    ///
    /// * `cs01` - Credit spread sensitivity (par spread bump)
    /// * `qualifying` - Whether the credit is investment grade (qualifying)
    pub fn calculate_credit_delta(&self, cs01: f64, qualifying: bool) -> f64 {
        let weight = if qualifying {
            *self
                .risk_weights
                .cq_delta_weights
                .get("corporates")
                .unwrap_or(&73.0)
        } else {
            self.risk_weights.cnq_delta_weight
        };

        (cs01 * weight).abs()
    }

    /// Calculate equity delta margin.
    ///
    /// # Arguments
    ///
    /// * `equity_delta` - Equity delta sensitivity
    pub fn calculate_equity_delta(&self, equity_delta: f64) -> f64 {
        (equity_delta * self.risk_weights.equity_delta_weight).abs()
    }

    /// Calculate FX delta margin.
    ///
    /// # Arguments
    ///
    /// * `fx_delta` - FX delta sensitivity
    pub fn calculate_fx_delta(&self, fx_delta: f64) -> f64 {
        (fx_delta * self.risk_weights.fx_delta_weight).abs()
    }

    /// Calculate commodity delta margin using SIMM bucket risk weights.
    pub fn calculate_commodity_delta(&self, delta_by_bucket: &HashMap<String, f64>) -> f64 {
        let mut weighted_sum = 0.0;
        for (bucket, delta) in delta_by_bucket {
            let weight = commodity_bucket_weight(bucket);
            weighted_sum += (delta * weight).abs();
        }
        weighted_sum
    }

    /// Calculate SIMM margin from pre-computed sensitivities.
    ///
    /// This is the primary entry point for SIMM calculation when you have
    /// `SimmSensitivities` from a `Marginable` instrument.
    ///
    /// # Arguments
    ///
    /// * `sensitivities` - SIMM sensitivities by risk class
    /// * `currency` - Currency for the resulting margin amounts
    ///
    /// # Returns
    ///
    /// A tuple of (total_margin, breakdown_by_risk_class).
    pub fn calculate_from_sensitivities(
        &self,
        sensitivities: &SimmSensitivities,
        currency: Currency,
    ) -> (f64, HashMap<String, Money>) {
        let mut breakdown = HashMap::default();
        let mut risk_class_margins = HashMap::default();

        // IR Delta
        if !sensitivities.ir_delta.is_empty() {
            let ir_delta_map: HashMap<String, f64> = sensitivities
                .ir_delta
                .iter()
                .map(|((_, tenor), delta)| (tenor.clone(), *delta))
                .collect();
            let ir_margin = self.calculate_ir_delta(&ir_delta_map);
            if ir_margin > 0.0 {
                breakdown.insert("IR_Delta".to_string(), Money::new(ir_margin, currency));
                risk_class_margins.insert(SimmRiskClass::InterestRate, ir_margin);
            }
        }

        // Credit Delta (Qualifying)
        let qualifying_total = sensitivities.credit_qualifying_delta.values().sum::<f64>();
        if qualifying_total.abs() > 0.0 {
            let credit_margin = self.calculate_credit_delta(qualifying_total, true);
            if credit_margin > 0.0 {
                breakdown.insert(
                    "Credit_Qualifying_Delta".to_string(),
                    Money::new(credit_margin, currency),
                );
                risk_class_margins.insert(SimmRiskClass::CreditQualifying, credit_margin);
            }
        }

        // Credit Delta (Non-Qualifying)
        let non_qual_total = sensitivities
            .credit_non_qualifying_delta
            .values()
            .sum::<f64>();
        if non_qual_total.abs() > 0.0 {
            let credit_margin = self.calculate_credit_delta(non_qual_total, false);
            if credit_margin > 0.0 {
                breakdown.insert(
                    "Credit_NonQualifying_Delta".to_string(),
                    Money::new(credit_margin, currency),
                );
                risk_class_margins.insert(SimmRiskClass::CreditNonQualifying, credit_margin);
            }
        }

        // Equity Delta
        let total_equity = sensitivities.total_equity_delta();
        if total_equity.abs() > 0.0 {
            let equity_margin = self.calculate_equity_delta(total_equity);
            if equity_margin > 0.0 {
                breakdown.insert(
                    "Equity_Delta".to_string(),
                    Money::new(equity_margin, currency),
                );
                risk_class_margins.insert(SimmRiskClass::Equity, equity_margin);
            }
        }

        // FX Delta
        let total_fx = sensitivities.fx_delta.values().sum::<f64>();
        if total_fx.abs() > 0.0 {
            let fx_margin = self.calculate_fx_delta(total_fx);
            if fx_margin > 0.0 {
                breakdown.insert("FX_Delta".to_string(), Money::new(fx_margin, currency));
                risk_class_margins.insert(SimmRiskClass::Fx, fx_margin);
            }
        }

        // Commodity Delta
        if !sensitivities.commodity_delta.is_empty() {
            let commodity_margin = self.calculate_commodity_delta(&sensitivities.commodity_delta);
            if commodity_margin > 0.0 {
                breakdown.insert(
                    "Commodity_Delta".to_string(),
                    Money::new(commodity_margin, currency),
                );
                risk_class_margins.insert(SimmRiskClass::Commodity, commodity_margin);
            }
        }

        let total_im = if risk_class_margins.is_empty() {
            0.0
        } else {
            self.aggregate_risk_classes(&risk_class_margins)
        };

        (total_im, breakdown)
    }

    /// Aggregate risk class margins with correlation.
    ///
    /// SIMM uses a correlation matrix to aggregate across risk classes.
    /// This helper provides a sqrt-of-sum-of-squares approximation and is used
    /// only by the heuristic [`ImCalculator`] implementation. The primary
    /// `calculate_from_sensitivities` path keeps a simple sum to preserve
    /// backwards-compatible behavior.
    pub fn aggregate_risk_classes(&self, risk_class_margins: &HashMap<SimmRiskClass, f64>) -> f64 {
        let mut sum = 0.0;
        for (risk_i, margin_i) in risk_class_margins {
            for (risk_j, margin_j) in risk_class_margins {
                let rho = risk_class_correlation(*risk_i, *risk_j);
                sum += rho * margin_i * margin_j;
            }
        }
        sum.max(0.0).sqrt()
    }
}

// ISDA SIMM v2.8+2506 risk-class correlations (applies to v2.5/v2.6 here).
fn risk_class_correlation(a: SimmRiskClass, b: SimmRiskClass) -> f64 {
    if a == b {
        return 1.0;
    }
    match (a, b) {
        (SimmRiskClass::InterestRate, SimmRiskClass::CreditQualifying)
        | (SimmRiskClass::CreditQualifying, SimmRiskClass::InterestRate) => 0.10,
        (SimmRiskClass::InterestRate, SimmRiskClass::CreditNonQualifying)
        | (SimmRiskClass::CreditNonQualifying, SimmRiskClass::InterestRate) => 0.14,
        (SimmRiskClass::InterestRate, SimmRiskClass::Equity)
        | (SimmRiskClass::Equity, SimmRiskClass::InterestRate) => 0.12,
        (SimmRiskClass::InterestRate, SimmRiskClass::Commodity)
        | (SimmRiskClass::Commodity, SimmRiskClass::InterestRate) => 0.30,
        (SimmRiskClass::InterestRate, SimmRiskClass::Fx)
        | (SimmRiskClass::Fx, SimmRiskClass::InterestRate) => 0.10,
        (SimmRiskClass::CreditQualifying, SimmRiskClass::CreditNonQualifying)
        | (SimmRiskClass::CreditNonQualifying, SimmRiskClass::CreditQualifying) => 0.60,
        (SimmRiskClass::CreditQualifying, SimmRiskClass::Equity)
        | (SimmRiskClass::Equity, SimmRiskClass::CreditQualifying) => 0.66,
        (SimmRiskClass::CreditQualifying, SimmRiskClass::Commodity)
        | (SimmRiskClass::Commodity, SimmRiskClass::CreditQualifying) => 0.25,
        (SimmRiskClass::CreditQualifying, SimmRiskClass::Fx)
        | (SimmRiskClass::Fx, SimmRiskClass::CreditQualifying) => 0.22,
        (SimmRiskClass::CreditNonQualifying, SimmRiskClass::Equity)
        | (SimmRiskClass::Equity, SimmRiskClass::CreditNonQualifying) => 0.52,
        (SimmRiskClass::CreditNonQualifying, SimmRiskClass::Commodity)
        | (SimmRiskClass::Commodity, SimmRiskClass::CreditNonQualifying) => 0.27,
        (SimmRiskClass::CreditNonQualifying, SimmRiskClass::Fx)
        | (SimmRiskClass::Fx, SimmRiskClass::CreditNonQualifying) => 0.15,
        (SimmRiskClass::Equity, SimmRiskClass::Commodity)
        | (SimmRiskClass::Commodity, SimmRiskClass::Equity) => 0.33,
        (SimmRiskClass::Equity, SimmRiskClass::Fx) | (SimmRiskClass::Fx, SimmRiskClass::Equity) => {
            0.24
        }
        (SimmRiskClass::Commodity, SimmRiskClass::Fx)
        | (SimmRiskClass::Fx, SimmRiskClass::Commodity) => 0.23,
        // Same class case is handled above with a == b check
        _ => 1.0,
    }
}

// Commodity delta risk weights by bucket (ISDA SIMM v2.8+2506).
fn commodity_bucket_weight(bucket: &str) -> f64 {
    let bucket_id = bucket_id_from_label(bucket).unwrap_or(16);
    match bucket_id {
        1 => 25.0,
        2 => 21.0,
        3 => 23.0,
        4 => 19.0,
        5 => 24.0,
        6 => 27.0,
        7 => 33.0,
        8 => 37.0,
        9 => 64.0,
        10 => 43.0,
        11 => 21.0,
        12 => 19.0,
        13 => 14.0,
        14 => 17.0,
        15 => 11.0,
        16 => 64.0,
        17 => 16.0,
        _ => 64.0,
    }
}

fn bucket_id_from_label(bucket: &str) -> Option<u8> {
    let trimmed = bucket.trim();
    if let Ok(value) = trimmed.parse::<u8>() {
        return Some(value);
    }
    let normalized: String = trimmed
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "coal" => Some(1),
        "crude" => Some(2),
        "lightends" => Some(3),
        "middledistillates" => Some(4),
        "heavydistillates" => Some(5),
        "northamericannaturalgas" => Some(6),
        "europeannaturalgas" => Some(7),
        "northamericanpowerandcarbon" => Some(8),
        "europeanpowerandcarbon" => Some(9),
        "freight" => Some(10),
        "basemetals" => Some(11),
        "preciousmetals" => Some(12),
        "grainsandoilseed" => Some(13),
        "softsandotheragriculturals" => Some(14),
        "livestockanddairy" => Some(15),
        "other" => Some(16),
        "indexes" | "indices" => Some(17),
        _ => None,
    }
}

impl ImCalculator for SimmCalculator {
    fn calculate(
        &self,
        instrument: &dyn Instrument,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<ImResult> {
        // Get instrument value for scaling
        let pv = instrument.value(context, as_of)?;
        let currency = pv.currency();

        // Simplified: estimate sensitivities from PV
        // In production, this would use actual DV01/CS01/Greek calculations
        let notional = pv.amount().abs();

        // Heuristic: estimate DV01 as ~0.01% of notional per year of duration
        // This is a rough approximation - real implementation would compute actual Greeks
        let estimated_dv01 = notional * 0.0001 * 5.0; // Assume ~5y duration

        let mut breakdown = HashMap::default();
        let mut risk_class_margins = HashMap::default();

        // IR risk (primary for IRS, bonds)
        let ir_margin =
            self.calculate_ir_delta(&[("5y".to_string(), estimated_dv01)].into_iter().collect());
        risk_class_margins.insert(SimmRiskClass::InterestRate, ir_margin);
        breakdown.insert("interest_rate".to_string(), Money::new(ir_margin, currency));
        // Aggregate across risk classes
        let total_im = self.aggregate_risk_classes(&risk_class_margins);

        Ok(ImResult::with_breakdown(
            Money::new(total_im, currency),
            ImMethodology::Simm,
            as_of,
            self.mpor_days,
            breakdown,
        ))
    }

    fn methodology(&self) -> ImMethodology {
        ImMethodology::Simm
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simm_version_display() {
        assert_eq!(SimmVersion::V2_6.to_string(), "SIMM v2.6");
    }

    #[test]
    fn ir_delta_calculation() {
        let calc = SimmCalculator::new(SimmVersion::V2_6);

        let dv01_by_tenor: HashMap<String, f64> = [
            ("5y".to_string(), 100_000.0), // $100K DV01 at 5y
        ]
        .into_iter()
        .collect();

        let ir_margin = calc.calculate_ir_delta(&dv01_by_tenor);

        // Risk weight for 5y is 51, so margin = 100K * 51 = 5.1M
        assert!((ir_margin - 5_100_000.0).abs() < 1.0);
    }

    #[test]
    fn credit_delta_calculation() {
        let calc = SimmCalculator::new(SimmVersion::V2_6);

        let cs01 = 50_000.0; // $50K CS01

        let cq_margin = calc.calculate_credit_delta(cs01, true);
        let cnq_margin = calc.calculate_credit_delta(cs01, false);

        // Qualifying uses lower weight (~73), non-qualifying uses 500
        assert!(cq_margin < cnq_margin);
        assert!((cq_margin - 3_650_000.0).abs() < 1.0); // 50K * 73
        assert!((cnq_margin - 25_000_000.0).abs() < 1.0); // 50K * 500
    }

    #[test]
    fn risk_weights_loaded() {
        let weights = SimmRiskWeights::v2_6();
        assert_eq!(weights.version, SimmVersion::V2_6);
        assert!(weights.ir_delta_weights.contains_key("5y"));
        assert!(weights.cq_delta_weights.contains_key("corporates"));
    }

    #[test]
    fn aggregation() {
        let calc = SimmCalculator::default();

        let risk_class_margins: HashMap<SimmRiskClass, f64> = [
            (SimmRiskClass::InterestRate, 1_000_000.0),
            (SimmRiskClass::CreditQualifying, 500_000.0),
        ]
        .into_iter()
        .collect();

        let total = calc.aggregate_risk_classes(&risk_class_margins);

        // sqrt(1M^2 + 0.5M^2 + 2*0.10*1M*0.5M) ≈ 1.162M
        assert!((total - 1_161_895.0).abs() < 1.0);
    }

    #[test]
    fn calculate_from_sensitivities_uses_risk_class_correlation() {
        let calc = SimmCalculator::new(SimmVersion::V2_6);

        let mut sens = SimmSensitivities::new(Currency::USD);
        sens.add_ir_delta(Currency::USD, "5y", 100_000.0);
        sens.add_equity_delta("AAPL", 100_000.0);

        let (total_im, breakdown) = calc.calculate_from_sensitivities(&sens, Currency::USD);

        let ir_margin = breakdown
            .get("IR_Delta")
            .expect("IR margin present")
            .amount();
        let eq_margin = breakdown
            .get("Equity_Delta")
            .expect("Equity margin present")
            .amount();

        let expected =
            (ir_margin * ir_margin + eq_margin * eq_margin + 2.0 * 0.12 * ir_margin * eq_margin)
                .sqrt();
        assert!((total_im - expected).abs() < 1.0);
    }
}
