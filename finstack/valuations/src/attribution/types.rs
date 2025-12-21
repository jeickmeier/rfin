//! Core data structures for P&L attribution.
//!
//! This module provides types for decomposing multi-period P&L changes into
//! constituent factors: carry, curve shifts, credit spreads, FX, volatility,
//! model parameters, and market scalars.

use finstack_core::config::{FinstackConfig, RoundingContext};
use finstack_core::money::fx::FxPolicyMeta;
use finstack_core::prelude::*;
use finstack_core::types::CurveId;
use indexmap::IndexMap;
use std::sync::Arc;

use crate::instruments::common::traits::Instrument;
use crate::results::ValuationResult;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Attribution methodology for decomposing P&L.
///
/// Three methodologies are supported:
/// - **Parallel**: Independent factor isolation (may not sum due to cross-effects)
/// - **Waterfall**: Sequential application (guarantees sum = total, order matters)
/// - **MetricsBased**: Linear approximation using existing metrics (fast but approximate)
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AttributionMethod {
    /// Independent factor isolation (may not sum due to cross-effects).
    ///
    /// Each factor is isolated independently by restoring T₀ values for that
    /// factor while keeping T₁ values for all others. Residual captures
    /// cross-effects and non-linearities.
    #[default]
    Parallel,

    /// Sequential waterfall attribution (guarantees sum = total, order matters).
    ///
    /// Factors are applied one-by-one in the specified order. Each factor's
    /// P&L is computed with all previous factors at T₁ and remaining at T₀.
    /// Residual is minimal by construction.
    Waterfall(Vec<AttributionFactor>),

    /// Use existing metrics (Theta, DV01, CS01) for approximation.
    ///
    /// Linear approximation using pre-computed sensitivities. Fast but less
    /// accurate for large market moves due to convexity effects.
    MetricsBased,
}

/// Factor types for P&L attribution.
///
/// Maps cleanly to `MarketContext` structure:
/// - **RatesCurves**: discount_curves + forward_curves
/// - **CreditCurves**: hazard_curves
/// - **InflationCurves**: inflation_curves
/// - **Correlations**: base_correlation_curves
/// - **Fx**: FxMatrix
/// - **Volatility**: surfaces (VolSurface)
/// - **MarketScalars**: prices, series, inflation_indices, dividends
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AttributionFactor {
    /// Time decay and accruals (Theta).
    Carry,

    /// Interest rate curves (discount & forward).
    RatesCurves,

    /// Credit hazard curves (spread risk).
    CreditCurves,

    /// Inflation curves.
    InflationCurves,

    /// Base correlation curves (structured credit).
    Correlations,

    /// FX rate changes.
    Fx,

    /// Implied volatility changes.
    Volatility,

    /// Model-specific parameters (prepayment, default, recovery, conversion).
    ModelParameters,

    /// Market scalars (dividends, equity/commodity prices, inflation indices).
    MarketScalars,
}

/// Input parameters for P&L attribution.
///
/// Consolidates common parameters used across all attribution methods
/// (parallel, waterfall, metrics-based) to reduce function parameter counts
/// and improve API ergonomics.
///
/// # Method-Specific Parameters
///
/// Different attribution methods use different subsets of these parameters:
///
/// - **Parallel**: Uses `config` and `model_params_t0`
/// - **Waterfall**: Uses `config`, `model_params_t0`, and `strict_validation`
/// - **MetricsBased**: Uses `val_t0` and `val_t1`
///
/// # Examples
///
/// ```rust,ignore
/// use finstack_valuations::attribution::{AttributionInput, AttributionMethod};
/// use finstack_core::prelude::Date;
///
/// // Parallel attribution
/// let input = AttributionInput {
///     instrument: &my_bond,
///     market_t0: &market_t0,
///     market_t1: &market_t1,
///     as_of_t0: Date::new(2024, 1, 1),
///     as_of_t1: Date::new(2024, 1, 2),
///     config: Some(&config),
///     model_params_t0: None,
///     val_t0: None,
///     val_t1: None,
///     strict_validation: false,
/// };
///
/// let attribution = attribute_pnl(AttributionMethod::Parallel, &input)?;
///
/// // Metrics-based attribution
/// let input = AttributionInput {
///     instrument: &my_bond,
///     market_t0: &market_t0,
///     market_t1: &market_t1,
///     as_of_t0: Date::new(2024, 1, 1),
///     as_of_t1: Date::new(2024, 1, 2),
///     config: None,
///     model_params_t0: None,
///     val_t0: Some(&val_t0),
///     val_t1: Some(&val_t1),
///     strict_validation: false,
/// };
///
/// let attribution = attribute_pnl(AttributionMethod::MetricsBased, &input)?;
/// ```
#[derive(Clone)]
pub struct AttributionInput<'a> {
    /// Instrument to attribute P&L for.
    pub instrument: &'a Arc<dyn Instrument>,

    /// Market state at time T₀.
    pub market_t0: &'a MarketContext,

    /// Market state at time T₁.
    pub market_t1: &'a MarketContext,

    /// Valuation date T₀.
    pub as_of_t0: Date,

    /// Valuation date T₁.
    pub as_of_t1: Date,

    /// Configuration for rounding context (used by parallel and waterfall).
    ///
    /// Set to `None` for metrics-based attribution.
    pub config: Option<&'a FinstackConfig>,

    /// Model parameters snapshot at T₀ (used by parallel and waterfall).
    ///
    /// If provided, T₀ valuation will use these model parameters.
    /// Set to `None` for metrics-based attribution or when using current model parameters.
    pub model_params_t0: Option<&'a crate::attribution::model_params::ModelParamsSnapshot>,

    /// Pre-computed valuation at T₀ (used by metrics-based).
    ///
    /// Set to `None` for parallel and waterfall attribution.
    pub val_t0: Option<&'a ValuationResult>,

    /// Pre-computed valuation at T₁ (used by metrics-based).
    ///
    /// Set to `None` for parallel and waterfall attribution.
    pub val_t1: Option<&'a ValuationResult>,

    /// Strict validation for waterfall attribution (default: false).
    ///
    /// When true, waterfall attribution will error if factor order is incomplete.
    /// When false, missing factors are silently ignored.
    pub strict_validation: bool,
}

/// Complete P&L attribution result for a single instrument.
///
/// Decomposes total P&L into constituent factors with optional detailed
/// breakdowns by curve, tenor, FX pair, etc.
///
/// # Examples
///
/// ```rust,ignore
/// use finstack_valuations::attribution::{PnlAttribution, AttributionMethod};
///
/// let attribution = attribute_pnl_parallel(
///     &instrument,
///     &market_t0,
///     &market_t1,
///     as_of_t0,
///     as_of_t1,
///     &config,
/// )?;
///
/// println!("Total P&L: {}", attribution.total_pnl);
/// println!("Carry: {} ({:.1}%)",
///     attribution.carry,
///     attribution.carry.amount() / attribution.total_pnl.amount() * 100.0
/// );
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PnlAttribution {
    /// Total P&L (val_t1 - val_t0).
    pub total_pnl: Money,

    /// Carry P&L (theta + accruals).
    pub carry: Money,

    /// Interest rate curves P&L.
    pub rates_curves_pnl: Money,

    /// Credit hazard curves P&L.
    pub credit_curves_pnl: Money,

    /// Inflation curves P&L.
    pub inflation_curves_pnl: Money,

    /// Base correlation curves P&L.
    pub correlations_pnl: Money,

    /// FX rate changes P&L.
    pub fx_pnl: Money,

    /// Implied volatility changes P&L.
    pub vol_pnl: Money,

    /// Model parameters P&L.
    pub model_params_pnl: Money,

    /// Market scalars P&L.
    pub market_scalars_pnl: Money,

    /// Residual P&L (total - sum of attributed factors).
    pub residual: Money,

    // Detailed breakdowns
    /// Detailed rates curves attribution (by curve and tenor).
    pub rates_detail: Option<RatesCurvesAttribution>,

    /// Detailed credit curves attribution (by curve and tenor).
    pub credit_detail: Option<CreditCurvesAttribution>,

    /// Detailed inflation curves attribution (by curve, optional tenor).
    pub inflation_detail: Option<InflationCurvesAttribution>,

    /// Detailed correlations attribution (by curve).
    pub correlations_detail: Option<CorrelationsAttribution>,

    /// Detailed FX attribution (by currency pair).
    pub fx_detail: Option<FxAttribution>,

    /// Detailed volatility attribution (by surface).
    pub vol_detail: Option<VolAttribution>,

    /// Detailed model parameters attribution.
    pub model_params_detail: Option<ModelParamsAttribution>,

    /// Detailed market scalars attribution.
    pub scalars_detail: Option<ScalarsAttribution>,

    /// Attribution metadata.
    pub meta: AttributionMeta,
}

/// Detailed attribution for interest rate curves.
///
/// Provides aggregate and per-curve/per-tenor breakdown for discount
/// and forward curves.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RatesCurvesAttribution {
    /// P&L by curve ID.
    pub by_curve: IndexMap<CurveId, Money>,

    /// P&L by (curve_id, tenor).
    pub by_tenor: IndexMap<(CurveId, String), Money>,

    /// Total discount curves P&L.
    pub discount_total: Money,

    /// Total forward curves P&L.
    pub forward_total: Money,
}

/// Detailed attribution for credit hazard curves.
///
/// Provides per-curve and per-tenor breakdown for credit spread risk.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CreditCurvesAttribution {
    /// P&L by curve ID.
    pub by_curve: IndexMap<CurveId, Money>,

    /// P&L by (curve_id, tenor).
    pub by_tenor: IndexMap<(CurveId, String), Money>,
}

/// Detailed attribution for inflation curves.
///
/// Provides per-curve breakdown with optional tenor detail for
/// term-structured inflation curves.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InflationCurvesAttribution {
    /// P&L by curve ID.
    pub by_curve: IndexMap<CurveId, Money>,

    /// P&L by (curve_id, tenor) for term-structured inflation curves.
    pub by_tenor: Option<IndexMap<(CurveId, String), Money>>,
}

/// Detailed attribution for base correlation curves.
///
/// Used for structured credit products (CDO tranches, synthetic credit).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CorrelationsAttribution {
    /// P&L by correlation curve ID.
    pub by_curve: IndexMap<CurveId, Money>,
}

/// Detailed attribution for FX rate changes.
///
/// Provides per-currency-pair breakdown.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FxAttribution {
    /// P&L by (from_currency, to_currency) pair.
    pub by_pair: IndexMap<(Currency, Currency), Money>,
}

/// Detailed attribution for implied volatility changes.
///
/// Provides per-surface breakdown.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VolAttribution {
    /// P&L by volatility surface ID.
    pub by_surface: IndexMap<CurveId, Money>,
}

/// Detailed attribution for model-specific parameters.
///
/// Extensible structure for instrument-specific model parameters
/// (prepayment speeds, default rates, recovery rates, etc.).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ModelParamsAttribution {
    /// Prepayment speed changes (for MBS/ABS).
    pub prepayment: Option<Money>,

    /// Default rate changes (for structured credit).
    pub default_rate: Option<Money>,

    /// Recovery rate changes (for credit instruments).
    pub recovery_rate: Option<Money>,

    /// Conversion ratio changes (for convertible bonds).
    pub conversion_ratio: Option<Money>,

    /// Other model-specific parameters.
    #[cfg_attr(feature = "serde", serde(default))]
    pub other: IndexMap<String, Money>,
}

/// Detailed attribution for market scalars.
///
/// Includes dividends, equity/commodity prices, inflation indices, etc.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ScalarsAttribution {
    /// Dividend changes by equity ID.
    #[cfg_attr(feature = "serde", serde(default))]
    pub dividends: IndexMap<CurveId, Money>,

    /// Inflation index changes.
    #[cfg_attr(feature = "serde", serde(default))]
    pub inflation: IndexMap<CurveId, Money>,

    /// Equity price changes.
    #[cfg_attr(feature = "serde", serde(default))]
    pub equity_prices: IndexMap<CurveId, Money>,

    /// Commodity price changes.
    #[cfg_attr(feature = "serde", serde(default))]
    pub commodity_prices: IndexMap<CurveId, Money>,
}

/// Attribution metadata.
///
/// Records methodology, dates, repricing count, and residual statistics.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AttributionMeta {
    /// Attribution method used.
    pub method: AttributionMethod,

    /// Start date (T₀).
    pub t0: Date,

    /// End date (T₁).
    pub t1: Date,

    /// Instrument identifier.
    pub instrument_id: String,

    /// Number of repricings performed.
    pub num_repricings: usize,

    /// Absolute tolerance for residual validation.
    pub tolerance_abs: f64,

    /// Percentage tolerance for residual validation.
    pub tolerance_pct: f64,

    /// Residual as percentage of total P&L.
    pub residual_pct: f64,

    /// Rounding context used for calculations.
    pub rounding: RoundingContext,

    /// FX policy metadata (if FX conversions were applied).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub fx_policy: Option<FxPolicyMeta>,

    /// Diagnostic notes and warnings.
    #[cfg_attr(feature = "serde", serde(default))]
    pub notes: Vec<String>,
}

impl PnlAttribution {
    /// Create a new P&L attribution with required fields.
    ///
    /// # Arguments
    ///
    /// * `total_pnl` - Total P&L (val_t1 - val_t0)
    /// * `instrument_id` - Instrument identifier
    /// * `t0` - Start date
    /// * `t1` - End date
    /// * `method` - Attribution methodology
    ///
    /// # Returns
    ///
    /// New `PnlAttribution` with all factor P&Ls initialized to zero.
    pub fn new(
        total_pnl: Money,
        instrument_id: impl Into<String>,
        t0: Date,
        t1: Date,
        method: AttributionMethod,
    ) -> Self {
        let zero = Money::new(0.0, total_pnl.currency());

        Self {
            total_pnl,
            carry: zero,
            rates_curves_pnl: zero,
            credit_curves_pnl: zero,
            inflation_curves_pnl: zero,
            correlations_pnl: zero,
            fx_pnl: zero,
            vol_pnl: zero,
            model_params_pnl: zero,
            market_scalars_pnl: zero,
            residual: total_pnl, // Initially all P&L is residual
            rates_detail: None,
            credit_detail: None,
            inflation_detail: None,
            correlations_detail: None,
            fx_detail: None,
            vol_detail: None,
            model_params_detail: None,
            scalars_detail: None,
            meta: AttributionMeta {
                method,
                t0,
                t1,
                instrument_id: instrument_id.into(),
                num_repricings: 0,
                tolerance_abs: 1.0,
                tolerance_pct: 0.01,
                residual_pct: 100.0,
                rounding: RoundingContext::default(),
                fx_policy: None,
                notes: Vec::new(),
            },
        }
    }

    /// Create a new P&L attribution with explicit rounding context.
    ///
    /// # Arguments
    ///
    /// * `total_pnl` - Total P&L (val_t1 - val_t0)
    /// * `instrument_id` - Instrument identifier
    /// * `t0` - Start date
    /// * `t1` - End date
    /// * `method` - Attribution methodology
    /// * `rounding` - Rounding context to stamp
    ///
    /// # Returns
    ///
    /// New `PnlAttribution` with all factor P&Ls initialized to zero.
    pub fn new_with_rounding(
        total_pnl: Money,
        instrument_id: impl Into<String>,
        t0: Date,
        t1: Date,
        method: AttributionMethod,
        rounding: RoundingContext,
    ) -> Self {
        let mut attr = Self::new(total_pnl, instrument_id, t0, t1, method);
        attr.meta.rounding = rounding;
        attr
    }

    /// Scale all attribution values by a factor.
    ///
    /// Useful for scaling per-unit attribution to position quantity.
    pub fn scale(&mut self, factor: f64) {
        self.total_pnl *= factor;
        self.carry *= factor;
        self.rates_curves_pnl *= factor;
        self.credit_curves_pnl *= factor;
        self.inflation_curves_pnl *= factor;
        self.correlations_pnl *= factor;
        self.fx_pnl *= factor;
        self.vol_pnl *= factor;
        self.model_params_pnl *= factor;
        self.market_scalars_pnl *= factor;
        self.residual *= factor;

        // Scale details if present
        if let Some(d) = &mut self.rates_detail {
            for v in d.by_curve.values_mut() {
                *v *= factor;
            }
            for v in d.by_tenor.values_mut() {
                *v *= factor;
            }
            d.discount_total *= factor;
            d.forward_total *= factor;
        }

        if let Some(d) = &mut self.credit_detail {
            for v in d.by_curve.values_mut() {
                *v *= factor;
            }
            for v in d.by_tenor.values_mut() {
                *v *= factor;
            }
        }

        if let Some(d) = &mut self.inflation_detail {
            for v in d.by_curve.values_mut() {
                *v *= factor;
            }
            if let Some(bt) = &mut d.by_tenor {
                for v in bt.values_mut() {
                    *v *= factor;
                }
            }
        }

        if let Some(d) = &mut self.correlations_detail {
            for v in d.by_curve.values_mut() {
                *v *= factor;
            }
        }

        if let Some(d) = &mut self.fx_detail {
            for v in d.by_pair.values_mut() {
                *v *= factor;
            }
        }

        if let Some(d) = &mut self.vol_detail {
            for v in d.by_surface.values_mut() {
                *v *= factor;
            }
        }

        if let Some(d) = &mut self.model_params_detail {
            if let Some(v) = &mut d.prepayment {
                *v *= factor;
            }
            if let Some(v) = &mut d.default_rate {
                *v *= factor;
            }
            if let Some(v) = &mut d.recovery_rate {
                *v *= factor;
            }
            if let Some(v) = &mut d.conversion_ratio {
                *v *= factor;
            }
            for v in d.other.values_mut() {
                *v *= factor;
            }
        }

        if let Some(d) = &mut self.scalars_detail {
            for v in d.dividends.values_mut() {
                *v *= factor;
            }
            for v in d.inflation.values_mut() {
                *v *= factor;
            }
            for v in d.equity_prices.values_mut() {
                *v *= factor;
            }
            for v in d.commodity_prices.values_mut() {
                *v *= factor;
            }
        }
    }

    /// Validate that all factor currencies match total_pnl currency.
    ///
    /// # Returns
    ///
    /// Ok(()) if all currencies match, Err otherwise.
    pub fn validate_currencies(&self) -> Result<()> {
        let expected = self.total_pnl.currency();

        let factors = [
            ("carry", self.carry.currency()),
            ("rates_curves", self.rates_curves_pnl.currency()),
            ("credit_curves", self.credit_curves_pnl.currency()),
            ("inflation_curves", self.inflation_curves_pnl.currency()),
            ("correlations", self.correlations_pnl.currency()),
            ("fx", self.fx_pnl.currency()),
            ("vol", self.vol_pnl.currency()),
            ("model_params", self.model_params_pnl.currency()),
            ("market_scalars", self.market_scalars_pnl.currency()),
        ];

        for (_name, ccy) in &factors {
            if *ccy != expected {
                return Err(Error::CurrencyMismatch {
                    expected,
                    actual: *ccy,
                });
            }
        }

        Ok(())
    }

    /// Compute residual as total_pnl minus sum of all attributed factors.
    ///
    /// Updates both `residual` and `residual_pct` fields.
    ///
    /// # Returns
    ///
    /// Ok(()) on success, Err if currency mismatch detected.
    ///
    /// # Notes
    ///
    /// On error, sets residual to zero and adds a diagnostic note to metadata.
    pub fn compute_residual(&mut self) -> Result<()> {
        // Validate currencies first
        if let Err(e) = self.validate_currencies() {
            let note = format!(
                "Currency validation failed during residual computation: {}",
                e
            );
            self.meta.notes.push(note);
            self.residual = Money::new(0.0, self.total_pnl.currency());
            self.meta.residual_pct = 0.0;
            return Err(e);
        }

        // Sum all attributed factors (safe now that currencies are validated)
        let mut attributed_sum = self.carry;
        attributed_sum = add_factor(
            attributed_sum,
            self.rates_curves_pnl,
            "rates curves P&L",
            &mut self.meta.notes,
        )?;
        attributed_sum = add_factor(
            attributed_sum,
            self.credit_curves_pnl,
            "credit curves P&L",
            &mut self.meta.notes,
        )?;
        attributed_sum = add_factor(
            attributed_sum,
            self.inflation_curves_pnl,
            "inflation curves P&L",
            &mut self.meta.notes,
        )?;
        attributed_sum = add_factor(
            attributed_sum,
            self.correlations_pnl,
            "correlations P&L",
            &mut self.meta.notes,
        )?;
        attributed_sum = add_factor(attributed_sum, self.fx_pnl, "FX P&L", &mut self.meta.notes)?;
        attributed_sum = add_factor(
            attributed_sum,
            self.vol_pnl,
            "vol P&L",
            &mut self.meta.notes,
        )?;
        attributed_sum = add_factor(
            attributed_sum,
            self.model_params_pnl,
            "model params P&L",
            &mut self.meta.notes,
        )?;
        attributed_sum = add_factor(
            attributed_sum,
            self.market_scalars_pnl,
            "market scalars P&L",
            &mut self.meta.notes,
        )?;

        self.residual = self.total_pnl.checked_sub(attributed_sum).map_err(|e| {
            let note = format!("Failed to compute residual: {}", e);
            self.meta.notes.push(note.clone());
            e
        })?;

        // Compute residual percentage (handle zero total_pnl) via RoundingContext
        let rc = &self.meta.rounding;
        self.meta.residual_pct =
            if !rc.is_effectively_zero_money(self.total_pnl.amount(), self.total_pnl.currency()) {
                (self.residual.amount() / self.total_pnl.amount()) * 100.0
            } else {
                0.0
            };

        Ok(())
    }

    /// Check if residual is within tolerance.
    ///
    /// Tolerance is either percentage-based (relative to total P&L) or
    /// absolute, whichever is larger.
    ///
    /// # Arguments
    ///
    /// * `pct_tolerance` - Percentage tolerance (e.g., 0.1 for 0.1%)
    /// * `abs_tolerance` - Absolute tolerance (e.g., 100.0 for $100)
    ///
    /// # Returns
    ///
    /// `true` if residual is within tolerance.
    pub fn residual_within_tolerance(&self, pct_tolerance: f64, abs_tolerance: f64) -> bool {
        let abs_residual = self.residual.amount().abs();
        let abs_total = self.total_pnl.amount().abs();

        // Tolerance is the larger of percentage-based or absolute
        let tolerance = if abs_total > 1e-10 {
            (abs_total * pct_tolerance / 100.0).max(abs_tolerance)
        } else {
            abs_tolerance
        };

        abs_residual <= tolerance
    }

    /// Check if residual is within the stored tolerance thresholds.
    ///
    /// Uses the tolerance_abs and tolerance_pct from metadata.
    ///
    /// # Returns
    ///
    /// `true` if residual is within the stored tolerances.
    pub fn residual_within_meta_tolerance(&self) -> bool {
        self.residual_within_tolerance(self.meta.tolerance_pct, self.meta.tolerance_abs)
    }

    /// Generate a structured tree explanation of P&L attribution.
    ///
    /// Creates a human-readable tree showing the total P&L broken down by factor.
    /// Includes detailed breakdowns where available (per-curve, per-tenor, etc.).
    ///
    /// # Returns
    ///
    /// Multi-line string with tree structure.
    ///
    /// # Examples
    ///
    /// ```text
    /// Total P&L: $125,430
    ///   ├─ Carry: $45,000 (35.8%)
    ///   ├─ Rates Curves: $65,000 (51.7%)
    ///   │   ├─ USD-OIS: $50,000
    ///   │   └─ EUR-OIS: $15,000
    ///   ├─ Credit Curves: $5,000 (4.0%)
    ///   ├─ FX: $12,000 (9.5%)
    ///   ├─ Vol: $2,000 (1.6%)
    ///   └─ Residual: -$1,570 (-1.2%)
    /// ```
    pub fn explain(&self) -> String {
        let mut lines = Vec::new();

        // Helper to format money and percentage
        let fmt = |amount: &Money, total: &Money| -> String {
            let rc = RoundingContext::default();
            let pct = if !rc.is_effectively_zero_money(total.amount(), total.currency()) {
                (amount.amount() / total.amount()) * 100.0
            } else {
                0.0
            };
            format!("{} ({:.1}%)", amount, pct)
        };

        let rc = RoundingContext::default();

        // Total P&L
        lines.push(format!("Total P&L: {}", self.total_pnl));

        // Carry
        if !rc.is_effectively_zero_money(self.carry.amount(), self.carry.currency()) {
            lines.push(format!("  ├─ Carry: {}", fmt(&self.carry, &self.total_pnl)));
        }

        // Rates curves
        if !rc.is_effectively_zero_money(
            self.rates_curves_pnl.amount(),
            self.rates_curves_pnl.currency(),
        ) {
            lines.push(format!(
                "  ├─ Rates Curves: {}",
                fmt(&self.rates_curves_pnl, &self.total_pnl)
            ));
            if let Some(ref detail) = self.rates_detail {
                for (curve_id, pnl) in &detail.by_curve {
                    lines.push(format!("  │   ├─ {}: {}", curve_id, pnl));
                }
            }
        }

        // Credit curves
        if !rc.is_effectively_zero_money(
            self.credit_curves_pnl.amount(),
            self.credit_curves_pnl.currency(),
        ) {
            lines.push(format!(
                "  ├─ Credit Curves: {}",
                fmt(&self.credit_curves_pnl, &self.total_pnl)
            ));
            if let Some(ref detail) = self.credit_detail {
                for (curve_id, pnl) in &detail.by_curve {
                    lines.push(format!("  │   ├─ {}: {}", curve_id, pnl));
                }
            }
        }

        // Inflation curves
        if !rc.is_effectively_zero_money(
            self.inflation_curves_pnl.amount(),
            self.inflation_curves_pnl.currency(),
        ) {
            lines.push(format!(
                "  ├─ Inflation Curves: {}",
                fmt(&self.inflation_curves_pnl, &self.total_pnl)
            ));
        }

        // Correlations
        if !rc.is_effectively_zero_money(
            self.correlations_pnl.amount(),
            self.correlations_pnl.currency(),
        ) {
            lines.push(format!(
                "  ├─ Correlations: {}",
                fmt(&self.correlations_pnl, &self.total_pnl)
            ));
        }

        // FX
        if !rc.is_effectively_zero_money(self.fx_pnl.amount(), self.fx_pnl.currency()) {
            lines.push(format!("  ├─ FX: {}", fmt(&self.fx_pnl, &self.total_pnl)));
        }

        // Volatility
        if !rc.is_effectively_zero_money(self.vol_pnl.amount(), self.vol_pnl.currency()) {
            lines.push(format!("  ├─ Vol: {}", fmt(&self.vol_pnl, &self.total_pnl)));
        }

        // Model parameters
        if !rc.is_effectively_zero_money(
            self.model_params_pnl.amount(),
            self.model_params_pnl.currency(),
        ) {
            lines.push(format!(
                "  ├─ Model Params: {}",
                fmt(&self.model_params_pnl, &self.total_pnl)
            ));
        }

        // Market scalars
        if !rc.is_effectively_zero_money(
            self.market_scalars_pnl.amount(),
            self.market_scalars_pnl.currency(),
        ) {
            lines.push(format!(
                "  ├─ Market Scalars: {}",
                fmt(&self.market_scalars_pnl, &self.total_pnl)
            ));
        }

        // Residual (always show)
        lines.push(format!(
            "  └─ Residual: {}",
            fmt(&self.residual, &self.total_pnl)
        ));

        lines.join("\n")
    }
}

#[cfg(feature = "serde")]
impl JsonEnvelope for PnlAttribution {
    fn parse_error(e: serde_json::Error) -> finstack_core::Error {
        finstack_core::Error::Calibration {
            message: format!("Failed to parse P&L attribution JSON: {}", e),
            category: "json_parse".to_string(),
        }
    }

    fn serialize_error(e: serde_json::Error) -> finstack_core::Error {
        finstack_core::Error::Calibration {
            message: format!("Failed to serialize P&L attribution: {}", e),
            category: "json_serialize".to_string(),
        }
    }
}

impl std::fmt::Display for AttributionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttributionMethod::Parallel => write!(f, "Parallel"),
            AttributionMethod::Waterfall(_) => write!(f, "Waterfall"),
            AttributionMethod::MetricsBased => write!(f, "MetricsBased"),
        }
    }
}

impl std::fmt::Display for AttributionFactor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttributionFactor::Carry => write!(f, "Carry"),
            AttributionFactor::RatesCurves => write!(f, "RatesCurves"),
            AttributionFactor::CreditCurves => write!(f, "CreditCurves"),
            AttributionFactor::InflationCurves => write!(f, "InflationCurves"),
            AttributionFactor::Correlations => write!(f, "Correlations"),
            AttributionFactor::Fx => write!(f, "Fx"),
            AttributionFactor::Volatility => write!(f, "Volatility"),
            AttributionFactor::ModelParameters => write!(f, "ModelParameters"),
            AttributionFactor::MarketScalars => write!(f, "MarketScalars"),
        }
    }
}

fn add_factor(sum: Money, value: Money, label: &str, notes: &mut Vec<String>) -> Result<Money> {
    sum.checked_add(value).map_err(|e| {
        let note = format!("Failed to add {}: {}", label, e);
        notes.push(note.clone());
        e
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn test_pnl_attribution_new() {
        let total = Money::new(1000.0, Currency::USD);
        let attr = PnlAttribution::new(
            total,
            "BOND-001",
            date!(2025 - 01 - 15),
            date!(2025 - 01 - 16),
            AttributionMethod::Parallel,
        );

        assert_eq!(attr.total_pnl, total);
        assert_eq!(attr.carry.amount(), 0.0);
        assert_eq!(attr.residual, total);
        assert_eq!(attr.meta.residual_pct, 100.0);
    }

    #[test]
    fn test_compute_residual() {
        let total = Money::new(1000.0, Currency::USD);
        let mut attr = PnlAttribution::new(
            total,
            "BOND-001",
            date!(2025 - 01 - 15),
            date!(2025 - 01 - 16),
            AttributionMethod::Parallel,
        );

        attr.carry = Money::new(100.0, Currency::USD);
        attr.rates_curves_pnl = Money::new(500.0, Currency::USD);
        attr.fx_pnl = Money::new(390.0, Currency::USD);

        attr.compute_residual()
            .expect("Residual computation should succeed in test");

        assert_eq!(attr.residual.amount(), 10.0); // 1000 - 100 - 500 - 390
        assert!((attr.meta.residual_pct - 1.0).abs() < 1e-10); // 10/1000 * 100
    }

    #[test]
    fn test_residual_tolerance() {
        let total = Money::new(10000.0, Currency::USD);
        let mut attr = PnlAttribution::new(
            total,
            "BOND-001",
            date!(2025 - 01 - 15),
            date!(2025 - 01 - 16),
            AttributionMethod::Parallel,
        );

        attr.carry = Money::new(9990.0, Currency::USD);
        attr.compute_residual()
            .expect("Residual computation should succeed in test");

        // Residual is 10.0
        // 0.1% of 10000 = 10, so should pass
        assert!(attr.residual_within_tolerance(0.1, 100.0));

        // But not 0.05% tolerance (= 5.0)
        assert!(!attr.residual_within_tolerance(0.05, 5.0));

        // Absolute tolerance of 100 should pass
        assert!(attr.residual_within_tolerance(0.01, 100.0));
    }

    #[test]
    fn test_currency_validation() {
        let total = Money::new(1000.0, Currency::USD);
        let mut attr = PnlAttribution::new(
            total,
            "BOND-001",
            date!(2025 - 01 - 15),
            date!(2025 - 01 - 16),
            AttributionMethod::Parallel,
        );

        // Valid - all USD
        assert!(attr.validate_currencies().is_ok());

        // Invalid - inject EUR
        attr.fx_pnl = Money::new(100.0, Currency::EUR);
        assert!(attr.validate_currencies().is_err());

        // Compute residual should handle gracefully
        let result = attr.compute_residual();
        assert!(result.is_err());
        assert!(!attr.meta.notes.is_empty());
        assert_eq!(attr.residual.amount(), 0.0);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_pnl_attribution_json_envelope_trait() {
        let total = Money::new(1000.0, Currency::USD);
        let mut attr = PnlAttribution::new(
            total,
            "BOND-001",
            date!(2025 - 01 - 15),
            date!(2025 - 01 - 16),
            AttributionMethod::Parallel,
        );

        // Set some values
        attr.carry = Money::new(100.0, Currency::USD);
        attr.rates_curves_pnl = Money::new(500.0, Currency::USD);
        attr.fx_pnl = Money::new(390.0, Currency::USD);
        attr.compute_residual()
            .expect("Residual computation should succeed");

        // Test to_json from JsonEnvelope trait
        let json = attr.to_json().expect("to_json should succeed");
        assert!(json.contains("BOND-001"));
        assert!(json.contains("\"carry\""));

        // Test from_json from JsonEnvelope trait
        let parsed = PnlAttribution::from_json(&json).expect("from_json should succeed");
        assert_eq!(parsed.total_pnl, attr.total_pnl);
        assert_eq!(parsed.carry, attr.carry);
        assert_eq!(parsed.rates_curves_pnl, attr.rates_curves_pnl);
        assert_eq!(parsed.fx_pnl, attr.fx_pnl);
        assert_eq!(parsed.residual.amount(), attr.residual.amount());

        // Test from_reader from JsonEnvelope trait
        let reader = std::io::Cursor::new(json.as_bytes());
        let parsed_from_reader =
            PnlAttribution::from_reader(reader).expect("from_reader should succeed");
        assert_eq!(parsed_from_reader.total_pnl, attr.total_pnl);
    }
}

/// Trait for types that can be serialized to/from JSON envelopes.
///
/// Provides default implementations for common JSON I/O operations with
/// consistent error handling. Types implementing this trait must provide
/// error conversion methods to map `serde_json` errors to domain-specific
/// error types.
///
/// # Type Requirements
///
/// Implementors must:
/// - Implement `serde::Serialize` for JSON output
/// - Implement `serde::de::DeserializeOwned` for JSON input
/// - Provide `parse_error` to convert deserialization errors
/// - Provide `serialize_error` to convert serialization errors
///
/// # Examples
///
/// ```rust,ignore
/// use finstack_valuations::attribution::JsonEnvelope;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct MyEnvelope {
///     schema: String,
///     data: String,
/// }
///
/// impl JsonEnvelope for MyEnvelope {
///     fn parse_error(e: serde_json::Error) -> finstack_core::Error {
///         finstack_core::Error::Calibration {
///             message: format!("Failed to parse envelope: {}", e),
///             category: "json_parse".to_string(),
///         }
///     }
///
///     fn serialize_error(e: serde_json::Error) -> finstack_core::Error {
///         finstack_core::Error::Calibration {
///             message: format!("Failed to serialize envelope: {}", e),
///             category: "json_serialize".to_string(),
///         }
///     }
/// }
///
/// // Now you can use the trait methods:
/// let envelope = MyEnvelope {
///     schema: "v1".to_string(),
///     data: "test".to_string(),
/// };
///
/// // Serialize to JSON string
/// let json = envelope.to_json()?;
///
/// // Parse from JSON string
/// let parsed = MyEnvelope::from_json(&json)?;
///
/// // Parse from reader
/// let cursor = std::io::Cursor::new(json.as_bytes());
/// let from_reader = MyEnvelope::from_reader(cursor)?;
/// ```
///
/// # Design Rationale
///
/// This trait eliminates boilerplate code for envelope types while maintaining:
/// - **Type safety**: Uses associated error types rather than generic `serde_json::Error`
/// - **Consistency**: All envelopes use the same serialization format (pretty-printed JSON)
/// - **Flexibility**: Implementors control error messages and categories
/// - **Ergonomics**: Three-line trait impl replaces ~30 lines of duplicate code per type
///
/// # Performance
///
/// JSON serialization is not optimized for performance. For high-throughput scenarios,
/// consider binary formats (bincode, MessagePack) or zero-copy alternatives (flatbuffers).
#[cfg(feature = "serde")]
pub trait JsonEnvelope: Sized + Serialize + serde::de::DeserializeOwned {
    /// Convert a JSON parsing error to the domain error type.
    ///
    /// # Arguments
    ///
    /// * `e` - The serde_json deserialization error
    ///
    /// # Returns
    ///
    /// Domain-specific error with context about the failure.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// fn parse_error(e: serde_json::Error) -> finstack_core::Error {
    ///     finstack_core::Error::Calibration {
    ///         message: format!("Failed to parse attribution envelope: {}", e),
    ///         category: "json_parse".to_string(),
    ///     }
    /// }
    /// ```
    fn parse_error(e: serde_json::Error) -> finstack_core::Error;

    /// Convert a JSON serialization error to the domain error type.
    ///
    /// # Arguments
    ///
    /// * `e` - The serde_json serialization error
    ///
    /// # Returns
    ///
    /// Domain-specific error with context about the failure.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// fn serialize_error(e: serde_json::Error) -> finstack_core::Error {
    ///     finstack_core::Error::Calibration {
    ///         message: format!("Failed to serialize attribution envelope: {}", e),
    ///         category: "json_serialize".to_string(),
    ///     }
    /// }
    /// ```
    fn serialize_error(e: serde_json::Error) -> finstack_core::Error;

    /// Parse from a JSON string.
    ///
    /// Uses `serde_json::from_str` internally with custom error conversion.
    ///
    /// # Arguments
    ///
    /// * `json` - JSON string to parse
    ///
    /// # Returns
    ///
    /// Parsed instance or error if JSON is malformed or fields are missing.
    ///
    /// # Errors
    ///
    /// Returns error via `parse_error` if:
    /// - JSON syntax is invalid
    /// - Required fields are missing (for `#[serde(deny_unknown_fields)]` types)
    /// - Type conversions fail
    /// - Custom validation fails (for types with `#[serde(deserialize_with)]`)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let json = r#"{"schema": "v1", "data": "test"}"#;
    /// let envelope = MyEnvelope::from_json(json)?;
    /// ```
    fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(Self::parse_error)
    }

    /// Parse from a reader (file, socket, buffer, etc.).
    ///
    /// Uses `serde_json::from_reader` internally with custom error conversion.
    /// Efficient for large JSON payloads as it streams parsing.
    ///
    /// # Arguments
    ///
    /// * `reader` - Any type implementing `std::io::Read`
    ///
    /// # Returns
    ///
    /// Parsed instance or error if JSON is malformed or fields are missing.
    ///
    /// # Errors
    ///
    /// Same error conditions as `from_json`, plus I/O errors from the reader.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // From file
    /// let file = std::fs::File::open("envelope.json")?;
    /// let envelope = MyEnvelope::from_reader(file)?;
    ///
    /// // From in-memory buffer
    /// let cursor = std::io::Cursor::new(json_bytes);
    /// let envelope = MyEnvelope::from_reader(cursor)?;
    /// ```
    fn from_reader<R: std::io::Read>(reader: R) -> Result<Self> {
        serde_json::from_reader(reader).map_err(Self::parse_error)
    }

    /// Serialize to a pretty-printed JSON string.
    ///
    /// Uses `serde_json::to_string_pretty` for human-readable output with
    /// 2-space indentation. For compact JSON, use `serde_json::to_string(self)`
    /// directly.
    ///
    /// # Returns
    ///
    /// JSON string with proper formatting or error if serialization fails.
    ///
    /// # Errors
    ///
    /// Returns error via `serialize_error` if:
    /// - Circular references detected (should not happen with finite types)
    /// - Custom serialization logic fails
    /// - Very rare serde_json internal errors
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let envelope = MyEnvelope { schema: "v1".to_string(), data: "test".to_string() };
    /// let json = envelope.to_json()?;
    /// println!("{}", json);  // Pretty-printed with indentation
    /// ```
    fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(Self::serialize_error)
    }
}

#[cfg(all(test, feature = "serde"))]
mod json_envelope_tests {
    use super::*;

    /// Test envelope type to verify JsonEnvelope trait functionality.
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct TestEnvelope {
        schema: String,
        data: String,
        number: i32,
    }

    impl JsonEnvelope for TestEnvelope {
        fn parse_error(e: serde_json::Error) -> finstack_core::Error {
            finstack_core::Error::Calibration {
                message: format!("Failed to parse test envelope: {}", e),
                category: "test_parse".to_string(),
            }
        }

        fn serialize_error(e: serde_json::Error) -> finstack_core::Error {
            finstack_core::Error::Calibration {
                message: format!("Failed to serialize test envelope: {}", e),
                category: "test_serialize".to_string(),
            }
        }
    }

    #[test]
    fn test_json_envelope_roundtrip() {
        let envelope = TestEnvelope {
            schema: "test/v1".to_string(),
            data: "test data".to_string(),
            number: 42,
        };

        // Serialize to JSON
        let json = envelope.to_json().expect("Serialization should succeed");
        assert!(json.contains("\"schema\""));
        assert!(json.contains("\"test/v1\""));
        assert!(json.contains("\"data\""));
        assert!(json.contains("\"test data\""));
        assert!(json.contains("\"number\""));
        assert!(json.contains("42"));

        // Parse back from JSON
        let parsed = TestEnvelope::from_json(&json).expect("Deserialization should succeed");
        assert_eq!(parsed.schema, envelope.schema);
        assert_eq!(parsed.data, envelope.data);
        assert_eq!(parsed.number, envelope.number);
    }

    #[test]
    fn test_json_envelope_from_reader() {
        let envelope = TestEnvelope {
            schema: "test/v1".to_string(),
            data: "reader test".to_string(),
            number: 123,
        };

        // Serialize to JSON
        let json = envelope.to_json().expect("Serialization should succeed");

        // Create a reader from the JSON string
        let cursor = std::io::Cursor::new(json.as_bytes());

        // Parse from reader
        let parsed =
            TestEnvelope::from_reader(cursor).expect("Deserialization from reader should succeed");
        assert_eq!(parsed.schema, envelope.schema);
        assert_eq!(parsed.data, envelope.data);
        assert_eq!(parsed.number, envelope.number);
    }

    #[test]
    fn test_json_envelope_parse_error() {
        let invalid_json = r#"{"schema": "test/v1", "data": "test", "number": "not a number"}"#;

        let result = TestEnvelope::from_json(invalid_json);
        assert!(result.is_err());

        // Verify error message contains expected details
        let err = result.expect_err("Expected error from invalid JSON");
        if let finstack_core::Error::Calibration { message, category } = err {
            assert!(message.contains("Failed to parse test envelope"));
            assert_eq!(category, "test_parse");
        } else {
            panic!("Expected Calibration error, got: {:?}", err);
        }
    }

    #[test]
    fn test_json_envelope_missing_fields() {
        let incomplete_json = r#"{"schema": "test/v1"}"#;

        let result = TestEnvelope::from_json(incomplete_json);
        assert!(result.is_err());

        let err = result.expect_err("Expected error from incomplete JSON");
        if let finstack_core::Error::Calibration { message, category } = err {
            assert!(message.contains("Failed to parse test envelope"));
            assert_eq!(category, "test_parse");
        } else {
            panic!("Expected Calibration error, got: {:?}", err);
        }
    }

    #[test]
    fn test_json_envelope_malformed_json() {
        let malformed_json = r#"{"schema": "test/v1", "data": "test", "number": 42"#; // Missing closing brace

        let result = TestEnvelope::from_json(malformed_json);
        assert!(result.is_err());

        let err = result.expect_err("Expected error from malformed JSON");
        if let finstack_core::Error::Calibration { message, category } = err {
            assert!(message.contains("Failed to parse test envelope"));
            assert_eq!(category, "test_parse");
        } else {
            panic!("Expected Calibration error, got: {:?}", err);
        }
    }

    #[test]
    fn test_json_envelope_reader_io_error() {
        // Create a reader that will fail
        struct FailingReader;
        impl std::io::Read for FailingReader {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("Simulated I/O error"))
            }
        }

        let result = TestEnvelope::from_reader(FailingReader);
        assert!(result.is_err());

        let err = result.expect_err("Expected error from I/O failure");
        if let finstack_core::Error::Calibration { message, .. } = err {
            assert!(message.contains("Failed to parse test envelope"));
        } else {
            panic!("Expected Calibration error, got: {:?}", err);
        }
    }

    #[test]
    fn test_json_envelope_pretty_printing() {
        let envelope = TestEnvelope {
            schema: "test/v1".to_string(),
            data: "test".to_string(),
            number: 42,
        };

        let json = envelope.to_json().expect("Serialization should succeed");

        // Verify pretty-printing (should have newlines and indentation)
        assert!(json.contains('\n'));
        assert!(json.lines().count() > 1);

        // Should be parseable
        let parsed = TestEnvelope::from_json(&json).expect("Parsing pretty JSON should succeed");
        assert_eq!(parsed, envelope);
    }

    #[test]
    fn test_json_envelope_equivalence() {
        let envelope1 = TestEnvelope {
            schema: "test/v1".to_string(),
            data: "data1".to_string(),
            number: 100,
        };

        let envelope2 = TestEnvelope {
            schema: "test/v1".to_string(),
            data: "data1".to_string(),
            number: 100,
        };

        let json1 = envelope1.to_json().expect("Serialization should succeed");
        let json2 = envelope2.to_json().expect("Serialization should succeed");

        // JSON should be identical for identical structs
        assert_eq!(json1, json2);

        // Both should parse to equivalent structs
        let parsed1 = TestEnvelope::from_json(&json1).expect("Parse should succeed");
        let parsed2 = TestEnvelope::from_json(&json2).expect("Parse should succeed");
        assert_eq!(parsed1, parsed2);
        assert_eq!(parsed1, envelope1);
    }
}
