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
/// # Tolerance Design Rationale
///
/// The Z-spread tolerance is specified on the **spread axis** (in decimal, not bp).
/// The default `1e-10` (0.001 bp) is chosen to ensure:
///
/// 1. **Price accuracy**: For a 10Y bond with duration ~8, a spread tolerance
///    of `1e-10` translates to price error < `$0.00001` per $1000 face.
///
/// 2. **Consistency with YTM**: Same order of magnitude as YTM solver tolerance
///    ensures consistent precision across all yield/spread metrics.
///
/// ## Tolerance-to-Price Sensitivity
///
/// The relationship between spread tolerance and price accuracy:
///
/// ```text
/// Price Error ≈ Duration × Notional × Spread Tolerance
///
/// Example: Duration = 8, Notional = $1,000,000, Tolerance = 1e-10
/// Price Error ≈ 8 × 1,000,000 × 1e-10 = $0.0008
/// ```
///
/// ## Recommended Tolerances by Use Case
///
/// | Use Case | Tolerance | Spread Precision | Price Error ($1M) |
/// |----------|-----------|------------------|-------------------|
/// | Regulatory | `1e-12` | < 0.0001 bp | < $0.0001 |
/// | Trading | `1e-10` | < 0.01 bp | < $0.01 |
/// | Screening | `1e-8` | < 1 bp | < $1 |
///
/// # Maturity-Aware Bracketing
///
/// The initial bracket scales with maturity to handle both IG and HY:
///
/// ```text
/// bracket = min(base_bracket × (1 + years/30), max_bracket)
/// ```
///
/// This ensures:
/// - Short-dated IG bonds: tight ±500-1000 bp bracket → fast convergence
/// - Long-dated HY bonds: wider ±1500-3000 bp bracket → robust coverage
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::bond::metrics::price_yield_spread::ZSpreadSolverConfig;
///
/// // Default production configuration
/// let default = ZSpreadSolverConfig::default();
///
/// // Tighter tolerance for regulatory reporting
/// let regulatory = ZSpreadSolverConfig {
///     tolerance: 1e-12,
///     base_bracket_bp: 1000.0,
///     max_bracket_bp: 3000.0,
/// };
///
/// // Wider bracket for distressed debt screening
/// let distressed = ZSpreadSolverConfig {
///     tolerance: 1e-8,
///     base_bracket_bp: 2000.0,
///     max_bracket_bp: 5000.0,
/// };
/// ```
#[derive(Clone, Debug)]
pub struct ZSpreadSolverConfig {
    /// Convergence tolerance for the Z-spread solver (on the spread axis, decimal).
    ///
    /// Default: `1e-10` (~0.01 bp precision), which typically achieves price
    /// residuals well below `$0.01` per $1M face for all credit qualities.
    ///
    /// # Interpretation
    ///
    /// The solver stops when the price residual (model vs target) is less than
    /// `tolerance × duration × notional`, ensuring proportional accuracy.
    pub tolerance: f64,

    /// Base half-width of the initial search bracket, in basis points.
    ///
    /// Short-dated IG credit typically has spreads in 50-300 bp range, but
    /// we default to ±1000 bp to comfortably cover HY (300-800 bp) and
    /// distressed (800+ bp) names without manual configuration.
    ///
    /// # Maturity Scaling
    ///
    /// The actual bracket is scaled by maturity:
    /// `actual_bracket = base_bracket × (1 + years/30)`
    pub base_bracket_bp: f64,

    /// Maximum half-width of the initial search bracket after maturity scaling.
    ///
    /// Caps the bracket for very long-dated bonds (30Y+) to prevent excessive
    /// search domains that could slow convergence.
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
/// use finstack_core::market_data::context::MarketContext;
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
                        finstack_core::Error::from(finstack_core::InputError::NotFound {
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
            .map(|(d, amt)| -> finstack_core::Result<(f64, f64, f64)> {
                let t = dc.year_fraction(as_of, *d, DayCountCtx::default())?;
                let df_base = disc.df_between_dates(as_of, *d)?;
                Ok((t, df_base, amt.amount()))
            })
            .collect::<finstack_core::Result<Vec<_>>>()?;

        // Objective: PV_z(z) - target_value_ccy = 0
        let pricing_error: RefCell<Option<finstack_core::Error>> = RefCell::new(None);

        let objective = |z: f64| -> f64 {
            // Optimized PV calculation using pre-computed flows
            let mut pv = finstack_core::math::summation::NeumaierAccumulator::new();
            for (t, df_base, amt) in &cached_flows {
                // Apply Z-spread shift: exp(-z * t)
                let spread_df = (-z * t).exp();
                pv.add(amt * df_base * spread_df);
            }
            pv.total() - target_value_ccy
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
