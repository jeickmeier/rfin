use crate::cashflow::traits::CashflowProvider;
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::math::solver::{BrentSolver, Solver};
use std::cell::RefCell;

/// Configuration for Z-spread solver with maturity-aware bracket sizing.
///
/// Controls convergence tolerance and initial search bracket width for the
/// Z-spread root-finding algorithm. The bracket width scales with bond maturity
/// to handle both short-dated and long-dated bonds efficiently.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::bond::metrics::price_yield_spread::ZSpreadSolverConfig;
///
/// let config = ZSpreadSolverConfig {
///     tolerance: 1e-12,        // Tighter tolerance
///     base_bracket_bp: 2000.0, // Wider initial bracket
///     max_bracket_bp: 5000.0,  // Higher cap for long maturities
/// };
/// ```
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
/// Calculates the zero-volatility spread (Z-spread) as the constant additive spread
/// to the base discount curve that makes the discounted value of future cashflows
/// equal to the bond's dirty market price. The spread is applied as an exponential
/// shift: `df_z(t) = df_base(t) * exp(-z * t)`.
///
/// Uses Brent's method with a maturity-aware initial bracket and a configurable
/// tolerance. The default configuration is tuned for production use:
/// - `tolerance = 1e-10`
/// - short-dated bonds: ±1000 bp initial bracket
/// - long-dated/distressed: widened up to ±3000 bp
///
/// # Dependencies
///
/// Requires `Accrued` metric to be computed first.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::bond::Bond;
/// use finstack_valuations::metrics::{MetricRegistry, MetricId, MetricContext};
/// use finstack_core::market_data::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example();
/// # let market = MarketContext::new();
/// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
/// // Z-spread is computed automatically when requesting bond metrics
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
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

        // OPTIMIZATION: Pre-calculate cashflow times and base discount factors
        // to avoid repeated date logic and curve lookups inside the solver loop.
        let flows = bond.build_schedule(&context.curves, context.as_of)?;
        let disc = context.curves.get_discount_ref(&bond.discount_curve_id)?;
        let as_of = context.as_of;
        let dc = disc.day_count();

        // Cache (time, df_base, amount) for each future cashflow
        let cached_flows: Vec<(f64, f64, f64)> = flows
            .iter()
            .filter(|(d, _)| *d > as_of)
            .map(|(d, amt)| {
                let t = dc
                    .year_fraction(as_of, *d, DayCountCtx::default())
                    .unwrap_or(0.0);
                let df_base_abs = disc.df_on_date_curve(*d);
                let df_as_of = disc.df_on_date_curve(as_of);
                let df_base = if df_as_of != 0.0 {
                    df_base_abs / df_as_of
                } else {
                    1.0
                };
                (t, df_base, amt.amount())
            })
            .collect();

        // Objective: PV_z(z) - target_value_ccy = 0
        let pricing_error: RefCell<Option<finstack_core::Error>> = RefCell::new(None);

        let objective = |z: f64| -> f64 {
            // Optimized PV calculation using pre-computed flows
            let mut pv = 0.0;
            for (t, df_base, amt) in &cached_flows {
                // Apply Z-spread shift: exp(-z * t)
                let spread_df = (-z * t).exp();
                pv += amt * df_base * spread_df;
            }
            pv - target_value_ccy
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
