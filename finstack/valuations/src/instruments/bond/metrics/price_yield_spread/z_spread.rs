use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::math::solver::{BrentSolver, Solver};
use std::cell::RefCell;

/// Calculates Z-Spread (zero-volatility spread) for fixed-rate bonds.
///
/// Market-standard definition: constant additive spread `z` to the base
/// discount curve such that the discounted value of future cashflows equals
/// the bond's dirty market price. We apply the spread as an exponential
/// shift on discount factors: `df_z(t) = df_base(t) * exp(-z * t)`.
///
/// Returns `z` in decimal units (e.g., 0.01 = 100 bps).
///
/// Solver configuration is provided via [`ZSpreadSolverConfig`] and is
/// maturity-aware by default.
#[derive(Clone, Debug)]
pub struct ZSpreadSolverConfig {
    /// Convergence tolerance for the Z-spread solver (on the spread axis).
    ///
    /// Default: `1e-10`, which typically achieves price residuals well below
    /// `1e-6 * notional` for investment-grade and high-yield bonds.
    pub tolerance: f64,

    /// Base half-width of the initial search bracket, in basis points.
    ///
    /// Short-dated IG credit is usually well inside ±100–300 bp, but we
    /// default to a **wide** ±1000 bp range to comfortably cover HY.
    pub base_bracket_bp: f64,

    /// Maximum half-width of the initial search bracket after maturity scaling.
    ///
    /// Provides safety for distressed/long-dated names without exploding the
    /// initial search domain.
    pub max_bracket_bp: f64,
}

impl Default for ZSpreadSolverConfig {
    fn default() -> Self {
        Self {
            tolerance: 1e-10,
            // Short-dated bonds: ±1000 bp is generous and covers HY/distressed
            base_bracket_bp: 1000.0,
            // Allow widening for long maturities, but cap to a realistic range
            max_bracket_bp: 3000.0,
        }
    }
}

/// Z-spread metric calculator for vanilla bonds.
///
/// Uses Brent's method with a maturity-aware initial bracket and a configurable
/// tolerance. The default configuration is tuned for production use:
/// - `tolerance = 1e-10`
/// - short-dated bonds: ±1000 bp initial bracket
/// - long-dated/distressed: widened up to ±3000 bp
#[derive(Clone, Debug, Default)]
pub struct ZSpreadCalculator {
    config: ZSpreadSolverConfig,
}

impl ZSpreadCalculator {
    /// Create a Z-spread calculator with default production-grade solver
    /// settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a Z-spread calculator with a custom solver configuration.
    pub fn with_config(config: ZSpreadSolverConfig) -> Self {
        Self { config }
    }

    /// Compute a maturity-aware initial bracket in decimal units.
    ///
    /// Short-dated bonds use the base bracket (e.g., ±1000 bp). Longer
    /// maturities widen the bracket smoothly up to `max_bracket_bp`.
    fn initial_bracket_decimal(&self, bond: &Bond, as_of: Date) -> finstack_core::Result<f64> {
        if as_of >= bond.maturity {
            return Ok(self.config.base_bracket_bp / 10_000.0);
        }
        let dc = bond.cashflow_spec.day_count();
        let years = dc
            .year_fraction(as_of, bond.maturity, DayCountCtx::default())?
            .max(0.0);

        // Scale between 1x and 2x base over 0–30y, then clamp.
        let maturity_scale = 1.0 + (years / 30.0).min(1.0);
        let bracket_bp =
            (self.config.base_bracket_bp * maturity_scale).min(self.config.max_bracket_bp);

        Ok(bracket_bp / 10_000.0)
    }
}

impl MetricCalculator for ZSpreadCalculator {
    fn dependencies(&self) -> &[MetricId] {
        // Need accrued to form dirty market price when using quoted clean price
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        // Determine dirty market value in currency
        let bond: &Bond = context.instrument_as()?;
        let target_value_ccy: f64 =
            if let Some(clean_px) = bond.pricing_overrides.quoted_clean_price {
                // Accrued from computed metrics (currency amount)
                let accrued_ccy = context
                    .computed
                    .get(&MetricId::Accrued)
                    .copied()
                    .ok_or_else(|| {
                        finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                            id: "metric:Accrued".to_string(),
                        })
                    })?;
                // Convert clean price (quote, pct of par) to currency and add accrued currency
                clean_px * bond.notional.amount() / 100.0 + accrued_ccy
            } else {
                // Fallback to base PV if no market quote
                context.base_value.amount()
            };

        // Objective: PV_z(z) - target_value_ccy = 0
        let curves = std::sync::Arc::clone(&context.curves);
        let as_of = context.as_of;
        let pricing_error: RefCell<Option<finstack_core::Error>> = RefCell::new(None);

        let objective = |z: f64| -> f64 {
            match crate::instruments::bond::pricing::quote_engine::price_from_z_spread(
                bond, &curves, as_of, z,
            ) {
                Ok(pv) => pv - target_value_ccy,
                Err(e) => {
                    // Capture the first pricing error and map to a large non-zero residual
                    let mut slot = pricing_error.borrow_mut();
                    if slot.is_none() {
                        *slot = Some(e);
                    }
                    drop(slot);
                    // Use a large residual with deterministic sign so the solver never sees a
                    // spurious "perfect fit" at the initial guess (0.0 spread).
                    1e12 * if z >= 0.0 { 1.0 } else { -1.0 }
                }
            }
        };

        // Solve using Brent with a maturity-aware bracket and production-grade
        // tolerance. Initial guess is 0.0 (0 bp).
        let bracket = self.initial_bracket_decimal(bond, as_of)?;
        let solver = BrentSolver::new()
            .with_tolerance(self.config.tolerance)
            .with_initial_bracket_size(Some(bracket));
        let z = solver.solve(objective, 0.0)?;

        // If any pricing error occurred during objective evaluation, surface it instead of
        // returning a potentially meaningless Z-spread.
        if let Some(err) = pricing_error.into_inner() {
            return Err(err);
        }

        Ok(z)
    }
}
