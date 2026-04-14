// finstack/scenarios/src/horizon.rs

//! Horizon total return analysis.
//!
//! Composes [`ScenarioSpec`] application with P&L attribution to answer:
//! "If I hold this instrument under these market assumptions, what is my
//! decomposed total return?"
//!
//! The caller supplies a [`ScenarioSpec`] that may include a
//! [`OperationSpec::TimeRollForward`] (holding period) alongside any market
//! shocks (spread widening, rate shifts, vol changes, etc.).  The engine
//! applies the spec to construct a T₁ market state, then delegates to the
//! existing attribution framework to decompose the P&L.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use finstack_scenarios::horizon::HorizonAnalysis;
//! use finstack_scenarios::{OperationSpec, ScenarioSpec, TimeRollMode};
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_valuations::instruments::internal::InstrumentExt;
//! use std::sync::Arc;
//! use time::macros::date;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let instrument: Arc<dyn InstrumentExt> = todo!("your instrument");
//! let market = MarketContext::new();
//! let as_of = date!(2025-01-15);
//!
//! // Hold for 3 months, spreads widen 25bp
//! let scenario = ScenarioSpec {
//!     id: "hold_3m_spread_25".into(),
//!     name: None,
//!     description: None,
//!     operations: vec![
//!         OperationSpec::TimeRollForward {
//!             period: "3M".into(),
//!             apply_shocks: true,
//!             roll_mode: TimeRollMode::BusinessDays,
//!         },
//!         OperationSpec::CurveParallelBp {
//!             curve_kind: finstack_scenarios::CurveKind::Hazard,
//!             curve_id: "AAPL-CDS".into(),
//!             discount_curve_id: None,
//!             bp: 25.0,
//!         },
//!     ],
//!     priority: 0,
//!     resolution_mode: Default::default(),
//! };
//!
//! let analyzer = HorizonAnalysis::default();
//! let result = analyzer.compute(&instrument, &market, as_of, &scenario)?;
//!
//! println!("Total return: {:.2}%", result.total_return_pct() * 100.0);
//! println!("Carry: {}", result.attribution.carry);
//! println!("Credit P&L: {}", result.attribution.credit_curves_pnl);
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;

use finstack_core::config::FinstackConfig;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_valuations::attribution::{
    attribute_pnl_metrics_based, attribute_pnl_parallel, attribute_pnl_taylor_standard,
    attribute_pnl_waterfall, default_attribution_metrics, AttributionFactor, AttributionMethod,
    PnlAttribution,
};
use finstack_valuations::instruments::internal::InstrumentExt as Instrument;
use finstack_valuations::instruments::PricingOptions;

use crate::engine::ApplicationReport;
use crate::{ExecutionContext, ScenarioEngine, ScenarioSpec};

/// Horizon total return analyzer.
///
/// Composes scenario application with P&L attribution.  Construct with an
/// [`AttributionMethod`] and [`FinstackConfig`], then call [`compute`] to
/// project an instrument forward under a [`ScenarioSpec`] and decompose the
/// resulting P&L.
///
/// [`compute`]: HorizonAnalysis::compute
#[derive(Debug, Clone)]
pub struct HorizonAnalysis {
    /// Attribution methodology for decomposing the horizon P&L.
    pub attribution_method: AttributionMethod,
    /// Finstack configuration (rounding, tolerances).
    pub config: FinstackConfig,
    /// Scenario engine instance.
    pub engine: ScenarioEngine,
}

impl Default for HorizonAnalysis {
    fn default() -> Self {
        Self {
            attribution_method: AttributionMethod::Parallel,
            config: FinstackConfig::default(),
            engine: ScenarioEngine::new(),
        }
    }
}

impl HorizonAnalysis {
    /// Create a new analyzer with the given attribution method and config.
    pub fn new(attribution_method: AttributionMethod, config: FinstackConfig) -> Self {
        Self {
            attribution_method,
            config,
            engine: ScenarioEngine::new(),
        }
    }
}

/// Result of a horizon total return computation.
///
/// Wraps a [`PnlAttribution`] with scenario context and convenience
/// accessors for total return percentage and annualized return.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HorizonResult {
    /// Full factor-decomposed P&L from the attribution framework.
    pub attribution: PnlAttribution,
    /// Initial instrument value at (market_t0, as_of_t0).
    pub initial_value: Money,
    /// Final instrument value at (market_t1, as_of_t1).
    pub terminal_value: Money,
    /// Number of calendar days in the horizon (`None` if no time-roll in spec).
    pub horizon_days: Option<i64>,
    /// Report from scenario engine application.
    pub scenario_report: ApplicationReport,
}

impl HorizonAnalysis {
    /// Compute horizon total return under a scenario.
    ///
    /// Applies the [`ScenarioSpec`] to the provided market context (cloned
    /// internally) and runs P&L attribution between the original and
    /// scenario-modified states.
    ///
    /// The spec may include a [`OperationSpec::TimeRollForward`] to define
    /// the holding period.  If no time-roll is present, the analysis is a
    /// pure mark-to-scenario (carry will be zero, `horizon_days` will be
    /// `None`).
    ///
    /// # Errors
    ///
    /// Returns an error if scenario application or attribution fails (e.g.
    /// missing market data for a curve referenced in the spec).
    pub fn compute(
        &self,
        instrument: &Arc<dyn Instrument>,
        market_t0: &MarketContext,
        as_of_t0: Date,
        scenario: &ScenarioSpec,
    ) -> crate::Result<HorizonResult> {
        // 1. Price at t0
        let initial_value = instrument
            .value(market_t0, as_of_t0)
            .map_err(|e| crate::Error::Internal(format!("t0 pricing failed: {e}")))?;

        // 2. Clone market and build execution context
        let mut market_t1 = market_t0.clone();
        let mut model = finstack_statements::FinancialModelSpec::new("__horizon_temp__", vec![]);
        let mut ctx = ExecutionContext {
            market: &mut market_t1,
            model: &mut model,
            instruments: None,
            rate_bindings: None,
            calendar: None,
            as_of: as_of_t0,
        };

        // 3. Apply scenario
        let scenario_report = self.engine.apply(scenario, &mut ctx)?;
        let as_of_t1 = ctx.as_of;

        // 4. Derive horizon
        let diff_days = (as_of_t1 - as_of_t0).whole_days();
        let horizon_days = if diff_days > 0 {
            Some(diff_days)
        } else {
            None
        };

        // 5. Run attribution
        let attribution = self.run_attribution(
            instrument,
            market_t0,
            &market_t1,
            as_of_t0,
            as_of_t1,
        )?;

        // 6. Price at t1
        let terminal_value = instrument
            .value(&market_t1, as_of_t1)
            .map_err(|e| crate::Error::Internal(format!("t1 pricing failed: {e}")))?;

        Ok(HorizonResult {
            attribution,
            initial_value,
            terminal_value,
            horizon_days,
            scenario_report,
        })
    }

    /// Dispatch to the appropriate attribution function based on `self.attribution_method`.
    fn run_attribution(
        &self,
        instrument: &Arc<dyn Instrument>,
        market_t0: &MarketContext,
        market_t1: &MarketContext,
        as_of_t0: Date,
        as_of_t1: Date,
    ) -> crate::Result<PnlAttribution> {
        let result = match &self.attribution_method {
            AttributionMethod::Parallel => attribute_pnl_parallel(
                instrument,
                market_t0,
                market_t1,
                as_of_t0,
                as_of_t1,
                &self.config,
                None,
            ),
            AttributionMethod::Waterfall(order) => attribute_pnl_waterfall(
                instrument,
                market_t0,
                market_t1,
                as_of_t0,
                as_of_t1,
                &self.config,
                order.clone(),
                false,
                None,
            ),
            AttributionMethod::MetricsBased => {
                let metrics = default_attribution_metrics();
                let val_t0 = instrument
                    .price_with_metrics(market_t0, as_of_t0, &metrics, PricingOptions::default())
                    .map_err(|e| {
                        crate::Error::Internal(format!("t0 metrics pricing failed: {e}"))
                    })?;
                let val_t1 = instrument
                    .price_with_metrics(market_t1, as_of_t1, &metrics, PricingOptions::default())
                    .map_err(|e| {
                        crate::Error::Internal(format!("t1 metrics pricing failed: {e}"))
                    })?;
                attribute_pnl_metrics_based(
                    instrument, market_t0, market_t1, &val_t0, &val_t1, as_of_t0, as_of_t1,
                )
            }
            AttributionMethod::Taylor(config) => attribute_pnl_taylor_standard(
                instrument, market_t0, market_t1, as_of_t0, as_of_t1, config,
            ),
        };
        result.map_err(|e| crate::Error::Internal(format!("attribution failed: {e}")))
    }
}

impl HorizonResult {
    /// Total return as a decimal fraction (e.g. 0.05 = 5%).
    ///
    /// Computed as `total_pnl / initial_value`.  Returns 0.0 if the initial
    /// value is zero to avoid division by zero.
    pub fn total_return_pct(&self) -> f64 {
        let iv = self.initial_value.amount();
        if iv == 0.0 {
            return 0.0;
        }
        self.attribution.total_pnl.amount() / iv
    }

    /// Annualized total return.
    ///
    /// Uses `(1 + total_return_pct)^(365 / horizon_days) - 1`.
    /// Returns `None` when there is no time-roll in the scenario (i.e.
    /// `horizon_days` is `None`).
    pub fn annualized_return(&self) -> Option<f64> {
        let days = self.horizon_days? as f64;
        if days <= 0.0 {
            return None;
        }
        let tr = self.total_return_pct();
        Some((1.0 + tr).powf(365.0 / days) - 1.0)
    }

    /// A single factor's P&L as a fraction of initial value.
    ///
    /// Returns 0.0 if initial value is zero.
    pub fn factor_contribution(&self, factor: &AttributionFactor) -> f64 {
        let iv = self.initial_value.amount();
        if iv == 0.0 {
            return 0.0;
        }
        let pnl = match factor {
            AttributionFactor::Carry => self.attribution.carry.amount(),
            AttributionFactor::RatesCurves => self.attribution.rates_curves_pnl.amount(),
            AttributionFactor::CreditCurves => self.attribution.credit_curves_pnl.amount(),
            AttributionFactor::InflationCurves => self.attribution.inflation_curves_pnl.amount(),
            AttributionFactor::Correlations => self.attribution.correlations_pnl.amount(),
            AttributionFactor::Fx => self.attribution.fx_pnl.amount(),
            AttributionFactor::Volatility => self.attribution.vol_pnl.amount(),
            AttributionFactor::ModelParameters => self.attribution.model_params_pnl.amount(),
            AttributionFactor::MarketScalars => self.attribution.market_scalars_pnl.amount(),
        };
        pnl / iv
    }
}
