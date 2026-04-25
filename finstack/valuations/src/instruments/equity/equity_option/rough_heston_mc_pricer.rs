//! Monte Carlo pricer for the rough Heston stochastic volatility model.
//!
//! Uses [`RoughHestonProcess`] with the [`RoughHestonHybrid`] Volterra discretization
//! to simulate spot and variance paths. The rough Heston discretization handles the
//! fractional kernel internally using standard normal increments — no fBM generator
//! is required.

use super::pricer::{collect_inputs_extended, option_currency};
use super::types::EquityOption;
use crate::instruments::common_impl::parameters::OptionType;
use crate::instruments::common_impl::traits::Instrument;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

/// Monte Carlo pricer for the rough Heston model.
///
/// Simulates spot and variance jointly under the rough Heston dynamics using the
/// hybrid Euler-Maruyama scheme with Volterra integral evaluation at every step.
/// The discretization is O(n^2) per path due to the non-Markovian variance kernel.
///
/// Rough Heston parameters are read from market scalars with the `ROUGH_HESTON_*`
/// prefix — see [`super::rough_heston_fourier_pricer`] for the full key listing.
pub(crate) struct EquityOptionRoughHestonMcPricer {
    /// Number of Monte Carlo paths.
    num_paths: usize,
    /// Number of time steps per path.
    num_steps: usize,
}

impl EquityOptionRoughHestonMcPricer {
    /// Create a new rough Heston MC pricer with explicit configuration.
    pub(crate) fn new(num_paths: usize, num_steps: usize) -> Self {
        Self {
            num_paths,
            num_steps,
        }
    }
}

impl Default for EquityOptionRoughHestonMcPricer {
    fn default() -> Self {
        use crate::instruments::common_impl::helpers::mc_defaults;
        Self::new(
            mc_defaults::DEFAULT_MC_PATHS,
            mc_defaults::DEFAULT_ROUGH_VOL_STEPS,
        )
    }
}

impl crate::pricer::Pricer for EquityOptionRoughHestonMcPricer {
    fn key(&self) -> crate::pricer::PricerKey {
        crate::pricer::PricerKey::new(
            crate::pricer::InstrumentType::EquityOption,
            crate::pricer::ModelKey::MonteCarloRoughHeston,
        )
    }

    #[tracing::instrument(
        name = "equity_option.rough_heston_mc.price_dyn",
        level = "debug",
        skip(self, instrument, market),
        fields(
            inst_id = %instrument.id(),
            as_of = %as_of,
            num_paths = self.num_paths,
            num_steps = self.num_steps,
        ),
        err,
    )]
    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common_impl::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> crate::pricer::PricingResult<crate::results::ValuationResult> {
        let equity_option = instrument
            .as_any()
            .downcast_ref::<EquityOption>()
            .ok_or_else(|| {
                crate::pricer::PricingError::type_mismatch(
                    crate::pricer::InstrumentType::EquityOption,
                    instrument.key(),
                )
            })?;

        let inputs = collect_inputs_extended(equity_option, market, as_of).map_err(|e| {
            crate::pricer::PricingError::model_failure_with_context(
                e.to_string(),
                crate::pricer::PricingErrorContext::from_instrument(equity_option)
                    .model(crate::pricer::ModelKey::MonteCarloRoughHeston),
            )
        })?;
        let (spot, r, q, _sigma, t) = (inputs.spot, inputs.r, inputs.q, inputs.sigma, inputs.t_vol);

        if t <= 0.0 {
            let intrinsic = match equity_option.option_type {
                OptionType::Call => (spot - equity_option.strike).max(0.0),
                OptionType::Put => (equity_option.strike - spot).max(0.0),
            };
            return Ok(crate::results::ValuationResult::stamped(
                equity_option.id(),
                as_of,
                Money::new(
                    intrinsic * equity_option.notional.amount(),
                    option_currency(equity_option),
                ),
            ));
        }

        // Source rough-Heston scalars from a single shared lookup.
        let s = crate::instruments::equity::equity_option::rough_heston_market::RoughHestonScalars::from_market(market);

        let err_ctx = crate::pricer::PricingErrorContext::from_instrument(equity_option)
            .model(crate::pricer::ModelKey::MonteCarloRoughHeston);

        // Build process
        let hurst_exp = finstack_core::math::fractional::HurstExponent::new(s.hurst)
            .map_err(|e| crate::pricer::PricingError::from_core(e, err_ctx.clone()))?;
        let params = finstack_monte_carlo::process::rough_heston::RoughHestonParams::new(
            r, q, hurst_exp, s.kappa, s.theta, s.sigma_v, s.rho, s.v0,
        )
        .map_err(|e| crate::pricer::PricingError::from_core(e, err_ctx.clone()))?;
        let process = finstack_monte_carlo::process::rough_heston::RoughHestonProcess::new(params);

        // Build time grid and discretization
        let time_grid = finstack_monte_carlo::time_grid::TimeGrid::uniform(t, self.num_steps)
            .map_err(|e| crate::pricer::PricingError::from_core(e, err_ctx.clone()))?;
        let times: Vec<f64> = (0..=self.num_steps)
            .map(|i| t * i as f64 / self.num_steps as f64)
            .collect();
        let disc = finstack_monte_carlo::discretization::rough_heston::RoughHestonHybrid::new(
            &times, s.hurst,
        )
        .map_err(|e| crate::pricer::PricingError::from_core(e, err_ctx.clone()))?;

        // Derive deterministic seed from instrument id
        let seed_val =
            if let Some(ref scenario) = equity_option.pricing_overrides.metrics.mc_seed_scenario {
                finstack_monte_carlo::seed::derive_seed(&equity_option.id, scenario)
            } else {
                finstack_monte_carlo::seed::derive_seed(&equity_option.id, "base")
            };

        // Resolve and cap the path count via the workspace helper before
        // allocating, so a malicious or typo'd `mc_paths` override can't OOM
        // the host.
        let num_paths = crate::instruments::common_impl::helpers::resolve_mc_paths(
            equity_option.pricing_overrides.model_config.mc_paths,
            self.num_paths,
        )
        .map_err(|e| crate::pricer::PricingError::from_core(e, err_ctx.clone()))?;

        // Build engine and payoff
        let engine = finstack_monte_carlo::engine::McEngine::builder()
            .num_paths(num_paths)
            .seed(seed_val)
            .time_grid(time_grid)
            .parallel(false)
            .build()
            .map_err(|e| crate::pricer::PricingError::from_core(e, err_ctx.clone()))?;

        let ccy = option_currency(equity_option);
        let discount_factor = (-r * t).exp();
        let initial_state = [spot, s.v0];
        let rng = finstack_monte_carlo::rng::philox::PhiloxRng::new(seed_val);

        let result = match equity_option.option_type {
            OptionType::Call => {
                let payoff = finstack_monte_carlo::payoff::vanilla::EuropeanCall::new(
                    equity_option.strike,
                    equity_option.notional.amount(),
                    self.num_steps,
                );
                engine
                    .price(
                        &rng,
                        &process,
                        &disc,
                        &initial_state,
                        &payoff,
                        ccy,
                        discount_factor,
                    )
                    .map_err(|e| crate::pricer::PricingError::from_core(e, err_ctx.clone()))?
            }
            OptionType::Put => {
                let payoff = finstack_monte_carlo::payoff::vanilla::EuropeanPut::new(
                    equity_option.strike,
                    equity_option.notional.amount(),
                    self.num_steps,
                );
                engine
                    .price(
                        &rng,
                        &process,
                        &disc,
                        &initial_state,
                        &payoff,
                        ccy,
                        discount_factor,
                    )
                    .map_err(|e| crate::pricer::PricingError::from_core(e, err_ctx))?
            }
        };

        let pv = result.mean;
        let mut vr = crate::results::ValuationResult::stamped(equity_option.id(), as_of, pv);
        if result.stderr > 0.0 {
            vr.measures
                .insert(crate::metrics::MetricId::custom("mc_stderr"), result.stderr);
        }
        Ok(vr)
    }
}
