//! Cliquet option Monte Carlo pricer.

#[cfg(feature = "mc")]
use crate::instruments::common_impl::mc::paths::ProcessParams;
#[cfg(feature = "mc")]
use crate::instruments::common_impl::mc::process::metadata::ProcessMetadata;
#[cfg(feature = "mc")]
use crate::instruments::common_impl::mc::rng::philox::PhiloxRng;
#[cfg(feature = "mc")]
use crate::instruments::common_impl::mc::time_grid::TimeGrid;
#[cfg(feature = "mc")]
use crate::instruments::common_impl::mc::traits::Discretization;
#[cfg(feature = "mc")]
use crate::instruments::common_impl::mc::traits::StochasticProcess;
#[cfg(feature = "mc")]
use crate::instruments::common_impl::models::monte_carlo::engine::{McEngine, McEngineConfig};
#[cfg(feature = "mc")]
use crate::instruments::common_impl::models::monte_carlo::payoff::cliquet::{
    CliquetCallPayoff, CliquetPayoffType as McPayoffType,
};
#[cfg(feature = "mc")]
use crate::instruments::common_impl::models::monte_carlo::pricer::path_dependent::PathDependentPricerConfig;
#[cfg(feature = "mc")]
use crate::instruments::common_impl::traits::Instrument;
#[cfg(feature = "mc")]
use crate::instruments::equity::cliquet_option::types::{CliquetOption, CliquetPayoffType};
#[cfg(feature = "mc")]
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
#[cfg(feature = "mc")]
use crate::results::ValuationResult;
#[cfg(feature = "mc")]
use finstack_core::dates::{Date, DayCountCtx};
#[cfg(feature = "mc")]
use finstack_core::market_data::context::MarketContext;
#[cfg(feature = "mc")]
use finstack_core::money::Money;
#[cfg(feature = "mc")]
use finstack_core::Result;

/// Piecewise constant GBM process for cliquet options.
/// Handles term structure of volatility and rates between reset dates.
#[cfg(feature = "mc")]
#[derive(Clone, Debug)]
struct PiecewiseGbmProcess {
    /// End times of intervals (sorted)
    times: Vec<f64>,
    /// Risk-free rates per interval
    rs: Vec<f64>,
    /// Dividend yields per interval
    qs: Vec<f64>,
    /// Volatilities per interval
    sigmas: Vec<f64>,
}

#[cfg(feature = "mc")]
impl StochasticProcess for PiecewiseGbmProcess {
    fn dim(&self) -> usize {
        1
    }

    fn num_factors(&self) -> usize {
        1
    }

    fn drift(&self, t: f64, x: &[f64], out: &mut [f64]) {
        let idx = self.times.partition_point(|&time| time < t);
        let idx = idx.min(self.times.len() - 1);
        // μ(S) = (r - q) S
        out[0] = (self.rs[idx] - self.qs[idx]) * x[0];
    }

    fn diffusion(&self, t: f64, x: &[f64], out: &mut [f64]) {
        let idx = self.times.partition_point(|&time| time < t);
        let idx = idx.min(self.times.len() - 1);
        // σ(S) = σ S
        out[0] = self.sigmas[idx] * x[0];
    }

    fn is_diagonal(&self) -> bool {
        true
    }
}

#[cfg(feature = "mc")]
impl ProcessMetadata for PiecewiseGbmProcess {
    fn metadata(&self) -> ProcessParams {
        let mut params = ProcessParams::new("PiecewiseGBM");
        // Just report first interval params as representative for metadata
        if !self.rs.is_empty() {
            params.add_param("r_initial", self.rs[0]);
            params.add_param("q_initial", self.qs[0]);
            params.add_param("sigma_initial", self.sigmas[0]);
        }
        params.with_factors(vec!["spot".to_string()])
    }
}

/// Exact discretization for Piecewise GBM.
#[cfg(feature = "mc")]
#[derive(Clone, Debug, Default)]
struct PiecewiseExactGbm;

#[cfg(feature = "mc")]
impl PiecewiseExactGbm {
    fn new() -> Self {
        Self
    }
}

#[cfg(feature = "mc")]
impl Discretization<PiecewiseGbmProcess> for PiecewiseExactGbm {
    fn step(
        &self,
        process: &PiecewiseGbmProcess,
        t: f64,
        dt: f64,
        x: &mut [f64],
        z: &[f64],
        _work: &mut [f64],
    ) {
        let idx = process.times.partition_point(|&time| time < t);
        let idx = idx.min(process.times.len() - 1);

        let r = process.rs[idx];
        let q = process.qs[idx];
        let sigma = process.sigmas[idx];

        // S(t+dt) = S(t) * exp( (r - q - 0.5*sigma^2)*dt + sigma*sqrt(dt)*Z )
        let drift = (r - q - 0.5 * sigma * sigma) * dt;
        let diffusion = sigma * dt.sqrt() * z[0];
        x[0] *= (drift + diffusion).exp();
    }

    fn work_size(&self, _process: &PiecewiseGbmProcess) -> usize {
        0
    }
}

/// Cliquet option Monte Carlo pricer.
#[cfg(feature = "mc")]
pub struct CliquetOptionMcPricer {
    config: PathDependentPricerConfig,
}

#[cfg(feature = "mc")]
impl CliquetOptionMcPricer {
    /// Create a new cliquet option MC pricer with default config.
    pub fn new() -> Self {
        Self {
            config: PathDependentPricerConfig::default(),
        }
    }

    /// Price a cliquet option using Monte Carlo.
    fn price_internal(
        &self,
        inst: &CliquetOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<finstack_core::money::Money> {
        let spot_scalar = curves.price(&inst.spot_id)?;
        let initial_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let final_date = inst.reset_dates.last().copied().unwrap_or(as_of);
        let t = inst
            .day_count
            .year_fraction(as_of, final_date, DayCountCtx::default())?;
        if t <= 0.0 {
            return Ok(Money::new(0.0, inst.notional.currency()));
        }

        // Get curves
        let disc_curve = curves.get_discount(inst.discount_curve_id.as_str())?;
        let vol_surface = curves.surface(inst.vol_surface_id.as_str())?;

        // Dividend yield from scalar id if provided
        //
        // When a dividend yield ID is explicitly provided, we require the lookup to succeed
        // and return a unitless scalar. Silent fallback to 0.0 would mask market data
        // configuration errors.
        let div_yield = if let Some(div_id) = &inst.div_yield_id {
            let ms = curves.price(div_id.as_str()).map_err(|e| {
                finstack_core::Error::Validation(format!(
                    "Failed to fetch dividend yield '{}': {}",
                    div_id, e
                ))
            })?;
            match ms {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                finstack_core::market_data::scalars::MarketScalar::Price(m) => {
                    return Err(finstack_core::Error::Validation(format!(
                        "Dividend yield '{}' should be a unitless scalar, got Price({})",
                        div_id,
                        m.currency()
                    )));
                }
            }
        } else {
            0.0
        };

        // Build piecewise parameters
        // Interval boundaries: reset dates
        let mut times = Vec::new();
        let mut rs = Vec::new();
        let mut qs = Vec::new();
        let mut sigmas = Vec::new();

        let mut prev_t = 0.0;
        let mut prev_var = 0.0;

        // Combine reset dates into a sorted list of times including maturity
        let mut check_points: Vec<f64> = inst
            .reset_dates
            .iter()
            .map(|d| {
                inst.day_count
                    .year_fraction(as_of, *d, DayCountCtx::default())
                    .unwrap_or(0.0)
            })
            .filter(|&t| t > 0.0)
            .collect();
        check_points.sort_by(|a, b| a.total_cmp(b));
        check_points.dedup();

        // Ensure we cover up to t if not in reset dates
        if let Some(&last) = check_points.last() {
            if last < t - 1e-6 {
                check_points.push(t);
            }
        } else {
            check_points.push(t);
        }

        for &curr_t in &check_points {
            if curr_t <= prev_t {
                continue;
            }

            // Forward Rate
            // df(prev_t) / df(curr_t) = exp(r * (curr_t - prev_t))
            // r = ln(df_prev / df_curr) / dt
            let t_prev_curve = disc_curve.day_count().year_fraction(
                disc_curve.base_date(),
                as_of,
                DayCountCtx::default(),
            )? + prev_t;
            let t_curr_curve = disc_curve.day_count().year_fraction(
                disc_curve.base_date(),
                as_of,
                DayCountCtx::default(),
            )? + curr_t;

            let df_prev = disc_curve.df(t_prev_curve);
            let df_curr = disc_curve.df(t_curr_curve);
            let dt = curr_t - prev_t;

            let fwd_r = if dt > 1e-6 && df_curr > 0.0 {
                (df_prev / df_curr).ln() / dt
            } else {
                0.0 // Fallback
            };

            // Forward Volatility
            // Look up volatility at the Forward price, not initial spot.
            // F(0, t) = S_0 * exp(-q*t) / DF(t) (approximated if r is stochastic, but here deterministic)
            // F(0, t) = S_0 * exp(-q*t) / df_curr (where df_curr is P(0, t))
            // Actually, we need F(0, curr_t) for the volatility lookup at curr_t
            // This is the standard ATM Forward Volatility.
            let forward_price = if df_curr > 0.0 {
                initial_spot * (-div_yield * curr_t).exp() / df_curr
                    * disc_curve.df(disc_curve.day_count().year_fraction(
                        disc_curve.base_date(),
                        as_of,
                        DayCountCtx::default(),
                    )?)
            } else {
                initial_spot
            };

            let vol_curr = vol_surface.value_clamped(curr_t, forward_price);
            let var_curr = vol_curr * vol_curr * curr_t;

            let fwd_var = var_curr - prev_var;
            let fwd_sigma = if fwd_var > 0.0 && dt > 1e-6 {
                (fwd_var / dt).sqrt()
            } else {
                vol_curr
            };

            times.push(curr_t);
            rs.push(fwd_r);
            qs.push(div_yield);
            sigmas.push(fwd_sigma);

            prev_t = curr_t;
            prev_var = var_curr;
        }

        let process = PiecewiseGbmProcess {
            times,
            rs,
            qs,
            sigmas,
        };

        let steps_per_year = self.config.steps_per_year;
        let num_steps = ((t * steps_per_year).round() as usize).max(self.config.min_steps);

        // Payoff reset times
        let reset_times: Vec<f64> = inst
            .reset_dates
            .iter()
            .map(|&date| {
                inst.day_count
                    .year_fraction(as_of, date, DayCountCtx::default())
                    .unwrap_or(0.0)
            })
            .collect();

        let payoff_type = match inst.payoff_type {
            CliquetPayoffType::Additive => McPayoffType::Additive,
            CliquetPayoffType::Multiplicative => McPayoffType::Multiplicative,
        };

        let payoff = CliquetCallPayoff::new(
            reset_times,
            inst.local_cap,
            inst.local_floor,
            inst.global_cap,
            inst.global_floor,
            inst.notional.amount(),
            inst.notional.currency(),
            initial_spot,
            payoff_type,
        );

        // Derive deterministic seed from instrument ID and scenario
        #[cfg(feature = "mc")]
        use crate::instruments::common_impl::models::monte_carlo::seed;

        let seed = if let Some(ref scenario) = inst.pricing_overrides.mc_seed_scenario {
            #[cfg(feature = "mc")]
            {
                seed::derive_seed(&inst.id, scenario)
            }
            #[cfg(not(feature = "mc"))]
            42
        } else {
            #[cfg(feature = "mc")]
            {
                seed::derive_seed(&inst.id, "base")
            }
            #[cfg(not(feature = "mc"))]
            self.config.seed
        };

        // Setup McEngine directly
        let time_grid = TimeGrid::uniform(t, num_steps)?;
        let engine_config = McEngineConfig {
            num_paths: self.config.num_paths,
            seed,
            time_grid,
            target_ci_half_width: None,
            use_parallel: self.config.use_parallel,
            chunk_size: self.config.chunk_size,
            path_capture: self.config.path_capture.clone(),
            antithetic: self.config.antithetic,
        };
        let engine = McEngine::new(engine_config);

        let rng = PhiloxRng::new(seed);
        let disc = PiecewiseExactGbm::new();
        let initial_state = vec![initial_spot];

        let maturity_date = inst
            .reset_dates
            .last()
            .copied()
            .unwrap_or(inst.reset_dates[0]);
        let discount_factor = disc_curve.df_between_dates(as_of, maturity_date)?;

        let result = engine.price(
            &rng,
            &process,
            &disc,
            &initial_state,
            &payoff,
            inst.notional.currency(),
            discount_factor,
        )?;

        Ok(result.mean)
    }
}

#[cfg(feature = "mc")]
impl Default for CliquetOptionMcPricer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "mc")]
impl Pricer for CliquetOptionMcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::CliquetOption, ModelKey::MonteCarloGBM)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common_impl::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let cliquet = instrument
            .as_any()
            .downcast_ref::<CliquetOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::CliquetOption, instrument.key())
            })?;

        let pv = self.price_internal(cliquet, market, as_of).map_err(|e| {
            PricingError::model_failure_ctx(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(cliquet.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
#[cfg(feature = "mc")]
pub(crate) fn compute_pv(
    inst: &CliquetOption,
    curves: &MarketContext,
    as_of: Date,
) -> Result<Money> {
    let pricer = CliquetOptionMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}
