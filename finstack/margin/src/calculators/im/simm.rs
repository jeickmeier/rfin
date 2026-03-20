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
//! > **Implementation note:** `calculate_from_sensitivities` applies intra-bucket
//! > tenor correlations for IR delta, vega margin (IR, equity, FX), curvature
//! > risk, concentration add-ons, and the SIMM risk-class correlation matrix.

use crate::calculators::traits::{ImCalculator, ImResult};
use crate::config::margin_registry_from_config;
use crate::registry::{embedded_registry, MarginRegistry, SimmParams};
use crate::traits::Marginable;
use crate::types::ImMethodology;
use crate::types::{SimmRiskClass, SimmSensitivities};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::HashMap;
use finstack_core::Result;
use tracing::debug;

/// SIMM version identifier.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
#[non_exhaustive]
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

// Lookup helpers for SimmParams fields.
impl SimmParams {
    fn correlation(&self, a: SimmRiskClass, b: SimmRiskClass) -> f64 {
        if a == b {
            return 1.0;
        }
        let key = ordered_pair(a, b);
        self.risk_class_correlations
            .get(&key)
            .copied()
            .unwrap_or(1.0)
    }

    fn commodity_bucket_weight(&self, bucket: &str) -> f64 {
        let key = bucket_id_from_label(bucket)
            .map(|id| id.to_string())
            .unwrap_or_else(|| "other".to_string());
        self.commodity_bucket_weights
            .get(&key)
            .or_else(|| self.commodity_bucket_weights.get("other"))
            .copied()
            .unwrap_or(64.0)
    }
}

fn commodity_inter_bucket_correlation(a: u8, b: u8) -> f64 {
    const CORR: [[f64; 17]; 17] = [
        [
            1.00, 0.22, 0.18, 0.21, 0.20, 0.24, 0.49, 0.16, 0.38, 0.14, 0.10, 0.02, 0.12, 0.11,
            0.02, 0.00, 0.17,
        ],
        [
            0.22, 1.00, 0.92, 0.90, 0.88, 0.25, 0.08, 0.19, 0.17, 0.17, 0.42, 0.28, 0.36, 0.27,
            0.20, 0.00, 0.64,
        ],
        [
            0.18, 0.92, 1.00, 0.87, 0.84, 0.16, 0.07, 0.15, 0.10, 0.18, 0.33, 0.22, 0.27, 0.23,
            0.16, 0.00, 0.54,
        ],
        [
            0.21, 0.90, 0.87, 1.00, 0.77, 0.19, 0.11, 0.18, 0.16, 0.14, 0.32, 0.22, 0.28, 0.22,
            0.11, 0.00, 0.58,
        ],
        [
            0.20, 0.88, 0.84, 0.77, 1.00, 0.19, 0.09, 0.12, 0.13, 0.18, 0.42, 0.34, 0.32, 0.29,
            0.13, 0.00, 0.59,
        ],
        [
            0.24, 0.25, 0.16, 0.19, 0.19, 1.00, 0.31, 0.62, 0.23, 0.10, 0.21, 0.05, 0.18, 0.10,
            0.08, 0.00, 0.28,
        ],
        [
            0.49, 0.08, 0.07, 0.11, 0.09, 0.31, 1.00, 0.21, 0.79, 0.17, 0.10, -0.08, 0.10, 0.07,
            -0.02, 0.00, 0.13,
        ],
        [
            0.16, 0.19, 0.15, 0.18, 0.12, 0.62, 0.21, 1.00, 0.16, 0.08, 0.13, -0.07, 0.07, 0.05,
            0.02, 0.00, 0.19,
        ],
        [
            0.38, 0.17, 0.10, 0.16, 0.13, 0.23, 0.79, 0.16, 1.00, 0.15, 0.09, -0.06, 0.06, 0.06,
            0.01, 0.00, 0.16,
        ],
        [
            0.14, 0.17, 0.18, 0.14, 0.18, 0.10, 0.17, 0.08, 0.15, 1.00, 0.16, 0.09, 0.14, 0.09,
            0.03, 0.00, 0.11,
        ],
        [
            0.10, 0.42, 0.33, 0.32, 0.42, 0.21, 0.10, 0.13, 0.09, 0.16, 1.00, 0.36, 0.30, 0.25,
            0.18, 0.00, 0.37,
        ],
        [
            0.02, 0.28, 0.22, 0.22, 0.34, 0.05, -0.08, -0.07, -0.06, 0.09, 0.36, 1.00, 0.20, 0.18,
            0.11, 0.00, 0.26,
        ],
        [
            0.12, 0.36, 0.27, 0.28, 0.32, 0.18, 0.10, 0.07, 0.06, 0.14, 0.30, 0.20, 1.00, 0.28,
            0.19, 0.00, 0.39,
        ],
        [
            0.11, 0.27, 0.23, 0.22, 0.29, 0.10, 0.07, 0.05, 0.06, 0.09, 0.25, 0.18, 0.28, 1.00,
            0.13, 0.00, 0.26,
        ],
        [
            0.02, 0.20, 0.16, 0.11, 0.13, 0.08, -0.02, 0.02, 0.01, 0.03, 0.18, 0.11, 0.19, 0.13,
            1.00, 0.00, 0.21,
        ],
        [
            0.00, 0.00, 0.00, 0.00, 0.00, 0.00, 0.00, 0.00, 0.00, 0.00, 0.00, 0.00, 0.00, 0.00,
            0.00, 1.00, 0.00,
        ],
        [
            0.17, 0.64, 0.54, 0.58, 0.59, 0.28, 0.13, 0.19, 0.16, 0.11, 0.37, 0.26, 0.39, 0.26,
            0.21, 0.00, 1.00,
        ],
    ];

    match (a, b) {
        (1..=17, 1..=17) => CORR[(a - 1) as usize][(b - 1) as usize],
        _ => 0.0,
    }
}

fn resolve_simm_params(
    version: SimmVersion,
    registry: &MarginRegistry,
) -> finstack_core::Result<&SimmParams> {
    if let Some(found) = registry.simm.values().find(|p| p.version == version) {
        return Ok(found);
    }
    if let Some(default_id) = &registry.simm_default {
        if let Some(p) = registry.simm.get(default_id) {
            return Ok(p);
        }
    }
    registry
        .simm
        .values()
        .next()
        .ok_or_else(|| finstack_core::Error::Validation("SIMM registry is empty".to_string()))
}

fn ordered_pair(a: SimmRiskClass, b: SimmRiskClass) -> (SimmRiskClass, SimmRiskClass) {
    if (a as u8) <= (b as u8) {
        (a, b)
    } else {
        (b, a)
    }
}

fn ordered_tenor_pair(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

/// Pre-computed flat correlation matrix for IR tenor lookups.
/// Avoids per-lookup String allocations in the O(n^2) delta/vega loops.
#[derive(Debug, Clone)]
struct IrTenorCorrelationMatrix {
    tenor_to_idx: HashMap<String, usize>,
    matrix: Vec<f64>,
    n: usize,
}

impl IrTenorCorrelationMatrix {
    fn build(params: &SimmParams) -> Self {
        let tenors: Vec<String> = params.ir_delta_weights.keys().cloned().collect();
        let n = tenors.len();
        let mut tenor_to_idx = HashMap::default();
        for (i, t) in tenors.iter().enumerate() {
            tenor_to_idx.insert(t.clone(), i);
        }

        let mut matrix = vec![1.0; n * n];
        for i in 0..n {
            for j in 0..n {
                if i != j {
                    let key = ordered_tenor_pair(&tenors[i], &tenors[j]);
                    let rho = params
                        .ir_tenor_correlations
                        .get(&key)
                        .copied()
                        .unwrap_or(0.5);
                    matrix[i * n + j] = rho;
                }
            }
        }

        Self {
            tenor_to_idx,
            matrix,
            n,
        }
    }

    fn correlation(&self, idx_a: usize, idx_b: usize) -> f64 {
        if idx_a == idx_b {
            return 1.0;
        }
        self.matrix[idx_a * self.n + idx_b]
    }
}

/// ISDA SIMM calculator.
///
/// Calculates initial margin using the ISDA Standard Initial Margin Model.
/// This is the industry standard methodology for bilateral OTC derivatives.
#[derive(Debug, Clone)]
pub struct SimmCalculator {
    /// SIMM parameters (risk weights, correlations, thresholds)
    pub params: SimmParams,
    ir_corr_matrix: IrTenorCorrelationMatrix,
}

impl Default for SimmCalculator {
    #[allow(clippy::expect_used)]
    fn default() -> Self {
        Self::new(SimmVersion::V2_6).expect("embedded margin registry is a compile-time asset")
    }
}

impl SimmCalculator {
    /// Create a new SIMM calculator with the specified version.
    ///
    /// # Errors
    ///
    /// Returns an error if the embedded margin registry cannot be loaded.
    pub fn new(version: SimmVersion) -> Result<Self> {
        let registry = embedded_registry()?;
        let params = resolve_simm_params(version, registry)?.clone();
        let ir_corr_matrix = IrTenorCorrelationMatrix::build(&params);
        Ok(Self {
            params,
            ir_corr_matrix,
        })
    }

    /// Create a new SIMM calculator resolved from a `FinstackConfig`.
    pub fn from_finstack_config(
        version: SimmVersion,
        cfg: &finstack_core::config::FinstackConfig,
    ) -> finstack_core::Result<Self> {
        let registry = margin_registry_from_config(cfg)?;
        let params = resolve_simm_params(version, &registry)?.clone();
        let ir_corr_matrix = IrTenorCorrelationMatrix::build(&params);
        Ok(Self {
            params,
            ir_corr_matrix,
        })
    }

    /// SIMM version.
    #[must_use]
    pub fn version(&self) -> SimmVersion {
        self.params.version
    }

    /// Margin period of risk (days).
    #[must_use]
    pub fn mpor_days(&self) -> u32 {
        self.params.mpor_days
    }

    /// Set margin period of risk.
    #[must_use]
    pub fn with_mpor(mut self, days: u32) -> Self {
        self.params.mpor_days = days;
        self
    }

    /// Calculate IR delta margin with multi-currency aggregation.
    ///
    /// Per ISDA SIMM methodology:
    /// 1. For each currency, compute K_ccy using intra-currency tenor correlations
    /// 2. Aggregate across currencies: `sqrt(sum_c sum_d gamma(c,d) * K_c * K_d)`
    ///    where gamma = 1.0 when c == d, and `ir_inter_currency_correlation` otherwise
    ///
    /// # Arguments
    ///
    /// * `ir_delta` - Map of (currency, tenor) to DV01 sensitivity
    pub fn calculate_ir_delta_multi_currency(
        &self,
        ir_delta: &HashMap<(Currency, String), f64>,
    ) -> f64 {
        // Group sensitivities by currency
        let mut by_currency: HashMap<Currency, HashMap<String, f64>> = HashMap::default();
        for ((ccy, tenor), delta) in ir_delta {
            *by_currency
                .entry(*ccy)
                .or_default()
                .entry(tenor.clone())
                .or_insert(0.0) += delta;
        }

        // Compute K_ccy for each currency using intra-currency tenor correlations
        let k_values: Vec<f64> = by_currency
            .values()
            .map(|tenor_map| self.calculate_ir_delta(tenor_map))
            .collect();

        // Single currency: no cross-currency aggregation needed
        if k_values.len() <= 1 {
            return k_values.first().copied().unwrap_or(0.0);
        }

        // Aggregate across currencies with inter-currency correlation gamma
        let gamma = self.params.ir_inter_currency_correlation;
        let mut total = 0.0;
        for (i, k_i) in k_values.iter().enumerate() {
            for (j, k_j) in k_values.iter().enumerate() {
                let corr = if i == j { 1.0 } else { gamma };
                total += corr * k_i * k_j;
            }
        }
        total.max(0.0).sqrt()
    }

    /// Calculate IR delta margin for a single currency from DV01 sensitivities.
    ///
    /// Uses intra-bucket tenor correlations per ISDA SIMM methodology:
    /// `K = sqrt(sum_i sum_j rho(i,j) * WS_i * WS_j)`
    ///
    /// # Arguments
    ///
    /// * `dv01_by_tenor` - Map of tenor bucket to DV01 sensitivity
    pub fn calculate_ir_delta(&self, dv01_by_tenor: &HashMap<String, f64>) -> f64 {
        let weighted: Vec<(usize, f64)> = dv01_by_tenor
            .iter()
            .filter_map(|(tenor, dv01)| {
                let weight = self.params.ir_delta_weights.get(tenor)?;
                let idx = self.ir_corr_matrix.tenor_to_idx.get(tenor)?;
                Some((*idx, dv01 * weight))
            })
            .collect();

        let mut sum = 0.0;
        for &(idx_i, ws_i) in &weighted {
            for &(idx_j, ws_j) in &weighted {
                let rho = self.ir_corr_matrix.correlation(idx_i, idx_j);
                sum += rho * ws_i * ws_j;
            }
        }
        sum.max(0.0).sqrt()
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
                .params
                .cq_delta_weights
                .get("corporates")
                .unwrap_or(&73.0)
        } else {
            self.params.cnq_delta_weight
        };

        (cs01 * weight).abs()
    }

    /// Calculate equity delta margin.
    ///
    /// # Arguments
    ///
    /// * `equity_delta` - Equity delta sensitivity
    pub fn calculate_equity_delta(&self, equity_delta: f64) -> f64 {
        (equity_delta * self.params.equity_delta_weight).abs()
    }

    /// Calculate FX delta margin.
    ///
    /// # Arguments
    ///
    /// * `fx_delta` - FX delta sensitivity
    pub fn calculate_fx_delta(&self, fx_delta: f64) -> f64 {
        (fx_delta * self.params.fx_delta_weight).abs()
    }

    /// Calculate commodity delta margin using SIMM bucket risk weights.
    pub fn calculate_commodity_delta(&self, delta_by_bucket: &HashMap<String, f64>) -> f64 {
        let weighted_buckets: Vec<(u8, f64)> = delta_by_bucket
            .iter()
            .filter_map(|(bucket, delta)| {
                let bucket_id = bucket_id_from_label(bucket)?;
                let weight = self.params.commodity_bucket_weight(bucket);
                Some((bucket_id, delta * weight))
            })
            .collect();

        let mut sum = 0.0;
        for &(bucket_i, weighted_i) in &weighted_buckets {
            for &(bucket_j, weighted_j) in &weighted_buckets {
                let rho = if bucket_i == bucket_j {
                    1.0
                } else {
                    commodity_inter_bucket_correlation(bucket_i, bucket_j)
                };
                sum += rho * weighted_i * weighted_j;
            }
        }
        sum.max(0.0).sqrt()
    }

    /// Calculate IR vega margin from vega sensitivities.
    pub fn calculate_ir_vega(&self, vega_by_tenor: &HashMap<String, f64>) -> f64 {
        let weight = self.params.ir_vega_weight;
        let indexed: Vec<(usize, f64)> = vega_by_tenor
            .iter()
            .filter_map(|(tenor, vega)| {
                let idx = self.ir_corr_matrix.tenor_to_idx.get(tenor.as_str())?;
                Some((*idx, *vega * weight))
            })
            .collect();

        let mut sum = 0.0;
        for &(idx_i, wv_i) in &indexed {
            for &(idx_j, wv_j) in &indexed {
                let rho = self.ir_corr_matrix.correlation(idx_i, idx_j);
                sum += rho * wv_i * wv_j;
            }
        }
        sum.max(0.0).sqrt()
    }

    /// Calculate credit vega margin.
    pub fn calculate_credit_vega(&self, total_vega: f64, qualifying: bool) -> f64 {
        let weight = if qualifying {
            self.params.cq_vega_weight
        } else {
            self.params.cnq_vega_weight
        };
        (total_vega * weight).abs()
    }

    /// Calculate equity vega margin.
    pub fn calculate_equity_vega(&self, total_vega: f64) -> f64 {
        (total_vega * self.params.equity_vega_weight).abs()
    }

    /// Calculate FX vega margin.
    pub fn calculate_fx_vega(&self, total_vega: f64) -> f64 {
        (total_vega * self.params.fx_vega_weight).abs()
    }

    /// Calculate commodity vega margin.
    pub fn calculate_commodity_vega(&self, total_vega: f64) -> f64 {
        (total_vega * self.params.commodity_vega_weight).abs()
    }

    /// Calculate curvature margin for a risk class.
    ///
    /// SIMM curvature risk = scale_factor x max(0, sum of curvature CVR)
    pub fn calculate_curvature(
        &self,
        curvature_by_risk_class: &HashMap<SimmRiskClass, f64>,
    ) -> f64 {
        let scale = self.params.curvature_scale_factor;
        let mut sum = 0.0;
        for (risk_i, cvr_i) in curvature_by_risk_class {
            let weighted_i = cvr_i * scale;
            for (risk_j, cvr_j) in curvature_by_risk_class {
                let weighted_j = cvr_j * scale;
                let rho = self.params.correlation(*risk_i, *risk_j);
                sum += rho * weighted_i * weighted_j;
            }
        }
        sum.max(0.0).sqrt()
    }

    /// Calculate concentration add-on for a risk class.
    ///
    /// If the net sensitivity exceeds the concentration threshold,
    /// apply a sqrt(|sensitivity| / threshold) multiplier.
    pub fn concentration_factor(&self, risk_class: SimmRiskClass, net_sensitivity: f64) -> f64 {
        if let Some(&threshold) = self.params.concentration_thresholds.get(&risk_class) {
            if threshold > 0.0 && net_sensitivity.abs() > threshold {
                (net_sensitivity.abs() / threshold).sqrt()
            } else {
                1.0
            }
        } else {
            1.0
        }
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

        // IR Delta — per-currency calculation with inter-currency aggregation
        if !sensitivities.ir_delta.is_empty() {
            let ir_margin = self.calculate_ir_delta_multi_currency(&sensitivities.ir_delta);
            if ir_margin > 0.0 {
                breakdown.insert("IR_Delta".to_string(), Money::new(ir_margin, currency));
                risk_class_margins.insert(SimmRiskClass::InterestRate, ir_margin);
            }
        }

        // IR Vega
        if !sensitivities.ir_vega.is_empty() {
            let ir_vega_map: HashMap<String, f64> = sensitivities
                .ir_vega
                .iter()
                .map(|((_, tenor), vega)| (tenor.clone(), *vega))
                .collect();
            let ir_vega_margin = self.calculate_ir_vega(&ir_vega_map);
            if ir_vega_margin > 0.0 {
                breakdown.insert("IR_Vega".to_string(), Money::new(ir_vega_margin, currency));
                *risk_class_margins
                    .entry(SimmRiskClass::InterestRate)
                    .or_insert(0.0) += ir_vega_margin;
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

        // Equity Vega
        let total_equity_vega: f64 = sensitivities.equity_vega.values().sum();
        if total_equity_vega.abs() > 0.0 {
            let equity_vega_margin = self.calculate_equity_vega(total_equity_vega);
            if equity_vega_margin > 0.0 {
                breakdown.insert(
                    "Equity_Vega".to_string(),
                    Money::new(equity_vega_margin, currency),
                );
                *risk_class_margins
                    .entry(SimmRiskClass::Equity)
                    .or_insert(0.0) += equity_vega_margin;
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

        // FX Vega
        let total_fx_vega: f64 = sensitivities.fx_vega.values().sum();
        if total_fx_vega.abs() > 0.0 {
            let fx_vega_margin = self.calculate_fx_vega(total_fx_vega);
            if fx_vega_margin > 0.0 {
                breakdown.insert("FX_Vega".to_string(), Money::new(fx_vega_margin, currency));
                *risk_class_margins.entry(SimmRiskClass::Fx).or_insert(0.0) += fx_vega_margin;
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

        // Apply concentration factors per risk class.
        // SIMM scales the risk-class margin by sqrt(|net_sensitivity| / threshold)
        // when the net sensitivity exceeds the concentration threshold.
        let net_sensitivities: HashMap<SimmRiskClass, f64> = [
            (SimmRiskClass::InterestRate, sensitivities.total_ir_delta()),
            (
                SimmRiskClass::CreditQualifying,
                sensitivities.credit_qualifying_delta.values().sum::<f64>(),
            ),
            (
                SimmRiskClass::CreditNonQualifying,
                sensitivities
                    .credit_non_qualifying_delta
                    .values()
                    .sum::<f64>(),
            ),
            (SimmRiskClass::Equity, sensitivities.total_equity_delta()),
            (
                SimmRiskClass::Fx,
                sensitivities.fx_delta.values().sum::<f64>(),
            ),
            (
                SimmRiskClass::Commodity,
                sensitivities.commodity_delta.values().sum::<f64>(),
            ),
        ]
        .into_iter()
        .collect();

        for (rc, margin) in risk_class_margins.iter_mut() {
            let net = net_sensitivities.get(rc).copied().unwrap_or(0.0);
            let cf = self.concentration_factor(*rc, net);
            if cf > 1.0 {
                *margin *= cf;
            }
        }

        // Curvature -- added on top of the correlated risk-class total
        let curvature_addon = if !sensitivities.curvature.is_empty() {
            let cm = self.calculate_curvature(&sensitivities.curvature);
            if cm > 0.0 {
                breakdown.insert("Curvature".to_string(), Money::new(cm, currency));
            }
            cm
        } else {
            0.0
        };

        let correlated_total = if risk_class_margins.is_empty() {
            0.0
        } else {
            self.aggregate_risk_classes(&risk_class_margins)
        };
        let total_im = correlated_total + curvature_addon;

        (total_im, breakdown)
    }

    /// Aggregate risk class margins with the SIMM inter-risk-class correlation matrix.
    ///
    /// `Total = sqrt(sum_i sum_j rho(i,j) * K_i * K_j)`
    pub fn aggregate_risk_classes(&self, risk_class_margins: &HashMap<SimmRiskClass, f64>) -> f64 {
        let mut sum = 0.0;
        for (risk_i, margin_i) in risk_class_margins {
            for (risk_j, margin_j) in risk_class_margins {
                let rho = self.params.correlation(*risk_i, *risk_j);
                sum += rho * margin_i * margin_j;
            }
        }
        sum.max(0.0).sqrt()
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
        instrument: &dyn Marginable,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<ImResult> {
        let mtm = instrument.mtm_for_vm(context, as_of)?;
        let currency = mtm.currency();
        let sensitivities = instrument.simm_sensitivities(context, as_of)?;
        let (total_im, breakdown) = self.calculate_from_sensitivities(&sensitivities, currency);

        debug!(
            instrument = instrument.id(),
            total_im,
            risk_classes = breakdown.len(),
            "SIMM IM calculated"
        );

        Ok(ImResult::with_breakdown(
            Money::new(total_im, currency),
            ImMethodology::Simm,
            as_of,
            self.mpor_days(),
            breakdown,
        ))
    }

    fn methodology(&self) -> ImMethodology {
        ImMethodology::Simm
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::traits::Marginable;

    #[test]
    fn simm_version_display() {
        assert_eq!(SimmVersion::V2_6.to_string(), "SIMM v2.6");
    }

    #[test]
    fn ir_delta_calculation() {
        let calc = SimmCalculator::new(SimmVersion::V2_6).expect("registry should load");

        // Single-tenor: correlation matrix is 1.0 on diagonal so
        // result = sqrt((dv01 * weight)^2) = |dv01 * weight|
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
        let calc = SimmCalculator::new(SimmVersion::V2_6).expect("registry should load");

        let cs01 = 50_000.0; // $50K CS01

        let cq_margin = calc.calculate_credit_delta(cs01, true);
        let cnq_margin = calc.calculate_credit_delta(cs01, false);

        // Qualifying uses lower weight (~73), non-qualifying uses 500
        assert!(cq_margin < cnq_margin);
        assert!((cq_margin - 3_650_000.0).abs() < 1.0); // 50K * 73
        assert!((cnq_margin - 25_000_000.0).abs() < 1.0); // 50K * 500
    }

    #[test]
    fn params_loaded() {
        let calc = SimmCalculator::new(SimmVersion::V2_6).expect("registry should load");
        assert_eq!(calc.version(), SimmVersion::V2_6);
        assert!(calc.params.ir_delta_weights.contains_key("5y"));
        assert!(calc.params.cq_delta_weights.contains_key("corporates"));
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
        let calc = SimmCalculator::new(SimmVersion::V2_6).expect("registry should load");

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

    #[test]
    fn ir_delta_multi_tenor_with_correlations() {
        let calc = SimmCalculator::new(SimmVersion::V2_6).expect("registry should load");

        let dv01_by_tenor: HashMap<String, f64> = [
            ("5y".to_string(), 100_000.0),
            ("10y".to_string(), -80_000.0), // Partially hedged
        ]
        .into_iter()
        .collect();

        let ir_margin = calc.calculate_ir_delta(&dv01_by_tenor);

        // ws_5y = 100K*51 = 5.1M, ws_10y = -80K*51 = -4.08M
        // With high tenor correlation (~0.96), the hedge offsets most of the risk
        // so margin should be much less than the uncorrelated sqrt(5.1^2 + 4.08^2) ≈ 6.53M
        assert!(ir_margin > 1_000_000.0);
        assert!(ir_margin < 3_000_000.0);
    }

    #[test]
    fn ir_vega_calculation() {
        let calc = SimmCalculator::new(SimmVersion::V2_6).expect("registry should load");

        let vega_by_tenor: HashMap<String, f64> =
            [("5y".to_string(), 500_000.0)].into_iter().collect();

        let ir_vega_margin = calc.calculate_ir_vega(&vega_by_tenor);
        // Single tenor: sqrt((500K * 0.21)^2) = 500K * 0.21 = 105K
        assert!((ir_vega_margin - 105_000.0).abs() < 1.0);
    }

    #[test]
    fn curvature_uses_correlated_aggregation_across_risk_classes() {
        let calc = SimmCalculator::new(SimmVersion::V2_6).expect("registry should load");
        let curvature_by_risk_class: HashMap<SimmRiskClass, f64> = [
            (SimmRiskClass::InterestRate, 1_000_000.0),
            (SimmRiskClass::Equity, -600_000.0),
        ]
        .into_iter()
        .collect();

        let actual = calc.calculate_curvature(&curvature_by_risk_class);
        let scale = calc.params.curvature_scale_factor;
        let rho = calc
            .params
            .correlation(SimmRiskClass::InterestRate, SimmRiskClass::Equity);
        let ir = 1_000_000.0 * scale;
        let eq = -600_000.0 * scale;
        let expected = (ir * ir + eq * eq + 2.0 * rho * ir * eq).sqrt();

        assert!(
            (actual - expected).abs() < 1.0,
            "expected correlated curvature {}, got {}",
            expected,
            actual
        );
    }

    #[test]
    fn commodity_delta_uses_bucket_correlations() {
        let calc = SimmCalculator::new(SimmVersion::V2_6).expect("registry should load");
        let delta_by_bucket: HashMap<String, f64> =
            [("2".to_string(), 100_000.0), ("3".to_string(), -100_000.0)]
                .into_iter()
                .collect();

        let actual = calc.calculate_commodity_delta(&delta_by_bucket);
        let bucket_2 = 100_000.0 * calc.params.commodity_bucket_weight("2");
        let bucket_3 = -100_000.0 * calc.params.commodity_bucket_weight("3");
        let rho_23 = 0.92_f64;
        let expected =
            (bucket_2 * bucket_2 + bucket_3 * bucket_3 + 2.0 * rho_23 * bucket_2 * bucket_3).sqrt();

        assert!(
            (actual - expected).abs() < 1.0,
            "expected correlated commodity margin {}, got {}",
            expected,
            actual
        );
    }

    #[derive(Clone)]
    struct MarginableTestInstrument {
        id: String,
        value: Money,
        sensitivities: SimmSensitivities,
    }

    impl MarginableTestInstrument {
        fn new(value: Money, sensitivities: SimmSensitivities) -> Self {
            Self {
                id: "SIMM-TEST".to_string(),
                value,
                sensitivities,
            }
        }
    }

    impl Marginable for MarginableTestInstrument {
        fn id(&self) -> &str {
            &self.id
        }

        fn margin_spec(&self) -> Option<&crate::types::OtcMarginSpec> {
            None
        }

        fn netting_set_id(&self) -> Option<crate::NettingSetId> {
            None
        }

        fn simm_sensitivities(
            &self,
            _market: &MarketContext,
            _as_of: Date,
        ) -> Result<SimmSensitivities> {
            Ok(self.sensitivities.clone())
        }

        fn mtm_for_vm(&self, _market: &MarketContext, _as_of: Date) -> Result<Money> {
            Ok(self.value)
        }
    }

    #[test]
    fn public_calculate_matches_full_simm_sensitivities() {
        let calc = SimmCalculator::new(SimmVersion::V2_6).expect("registry should load");
        let as_of = Date::from_calendar_date(2024, time::Month::January, 1).expect("valid date");

        let mut sensitivities = SimmSensitivities::new(Currency::USD);
        sensitivities.add_ir_delta(Currency::USD, "5y", 50_000.0);
        sensitivities.add_equity_delta("AAPL", 100_000.0);
        sensitivities.add_fx_delta(Currency::EUR, 80_000.0);

        let instrument = MarginableTestInstrument::new(
            Money::new(1_000_000.0, Currency::USD),
            sensitivities.clone(),
        );
        let market = MarketContext::new();

        let expected = calc.calculate_from_sensitivities(&sensitivities, Currency::USD);
        let actual = calc
            .calculate(&instrument, &market, as_of)
            .expect("SIMM calculation should succeed");

        assert!(
            (actual.amount.amount() - expected.0).abs() < 1e-2,
            "expected total {}, got {} with breakdown {:?}",
            expected.0,
            actual.amount.amount(),
            actual.breakdown
        );
        for (key, expected_amount) in &expected.1 {
            let actual_amount = actual
                .breakdown
                .get(key)
                .expect("expected breakdown entry should be present");
            assert!(
                (actual_amount.amount() - expected_amount.amount()).abs() < 1e-2,
                "breakdown mismatch for {key}: expected {}, got {}",
                expected_amount.amount(),
                actual_amount.amount()
            );
        }
        assert!(actual.breakdown.contains_key("Equity_Delta"));
        assert!(actual.breakdown.contains_key("FX_Delta"));
    }
}
