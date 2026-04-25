//! Equity option Heston Monte Carlo pricer.
//!
//! Prices European equity options under the Heston stochastic volatility model
//! using Monte Carlo simulation with the QE (Quadratic Exponential) discretization
//! scheme.

use crate::instruments::common_impl::models::closed_form::heston::HestonParams as ClosedFormHestonParams;
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::equity::equity_option::pricer::collect_inputs_extended;
use crate::instruments::equity::equity_option::types::EquityOption;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

use finstack_monte_carlo::discretization::qe_heston::QeHeston;
use finstack_monte_carlo::engine::McEngine;
use finstack_monte_carlo::payoff::vanilla::{EuropeanCall, EuropeanPut};
use finstack_monte_carlo::process::heston::{HestonParams, HestonProcess};
use finstack_monte_carlo::rng::philox::PhiloxRng;
use finstack_monte_carlo::seed;
use finstack_monte_carlo::time_grid::TimeGrid;

/// Equity option Heston Monte Carlo pricer.
///
/// Prices European equity options under the Heston stochastic volatility model
/// using QE discretization. Heston parameters are sourced from market scalars
/// (HESTON_KAPPA, HESTON_THETA, etc.) with sensible defaults.
pub(crate) struct EquityOptionHestonMcPricer {
    num_paths: usize,
    steps_per_year: f64,
}

impl EquityOptionHestonMcPricer {
    /// Create a new Heston MC pricer with default configuration.
    pub(crate) fn new() -> Self {
        use crate::instruments::common_impl::helpers::mc_defaults;
        Self {
            num_paths: mc_defaults::DEFAULT_MC_PATHS,
            steps_per_year: mc_defaults::DEFAULT_STEPS_PER_YEAR,
        }
    }

    /// Price an equity option using Heston Monte Carlo.
    fn price_internal(
        &self,
        inst: &EquityOption,
        market: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<(Money, f64)> {
        let inputs = collect_inputs_extended(inst, market, as_of)?;
        let (spot, r, q, _sigma, t) = (inputs.spot, inputs.r, inputs.q, inputs.sigma, inputs.t_vol);
        let ccy = inst.notional.currency();

        if t <= 0.0 {
            let intrinsic = match inst.option_type {
                crate::instruments::common_impl::parameters::OptionType::Call => {
                    (spot - inst.strike).max(0.0)
                }
                crate::instruments::common_impl::parameters::OptionType::Put => {
                    (inst.strike - spot).max(0.0)
                }
            };
            return Ok((Money::new(intrinsic * inst.notional.amount(), ccy), 0.0));
        }

        // Heston parameters: source from market scalars and fall back to
        // centralized defaults; validation (positive κ/θ/σᵥ/v₀, ρ ∈ (−1, 1))
        // is enforced by `from_market`. We then convert to the MC engine's
        // own `HestonParams` struct.
        let cf_params = ClosedFormHestonParams::from_market(market, r, q)?;
        let heston_params = HestonParams::new(
            cf_params.r,
            cf_params.q,
            cf_params.kappa,
            cf_params.theta,
            cf_params.sigma_v,
            cf_params.rho,
            cf_params.v0,
        )?;
        let process = HestonProcess::new(heston_params);
        let discretization = QeHeston::new();

        // Build time grid and engine
        let num_steps = ((t * self.steps_per_year).round() as usize).max(10);
        let time_grid = TimeGrid::uniform(t, num_steps)?;
        let maturity_step = time_grid.num_steps();

        let num_paths = crate::instruments::common_impl::helpers::resolve_mc_paths(
            inst.pricing_overrides.model_config.mc_paths,
            self.num_paths,
        )?;

        // Derive deterministic seed
        let seed_val = if let Some(ref scenario) = inst.pricing_overrides.metrics.mc_seed_scenario {
            seed::derive_seed(&inst.id, scenario)
        } else {
            seed::derive_seed(&inst.id, "base")
        };

        let engine = McEngine::builder()
            .num_paths(num_paths)
            .seed(seed_val)
            .time_grid(time_grid)
            .build()?;

        let rng = PhiloxRng::new(seed_val);
        let discount_factor = (-r * t).exp();

        // Initial state: [spot, v0]
        let initial_state = [spot, cf_params.v0];

        let result = match inst.option_type {
            crate::instruments::common_impl::parameters::OptionType::Call => {
                let payoff = EuropeanCall::new(inst.strike, inst.notional.amount(), maturity_step);
                engine.price(
                    &rng,
                    &process,
                    &discretization,
                    &initial_state,
                    &payoff,
                    ccy,
                    discount_factor,
                )?
            }
            crate::instruments::common_impl::parameters::OptionType::Put => {
                let payoff = EuropeanPut::new(inst.strike, inst.notional.amount(), maturity_step);
                engine.price(
                    &rng,
                    &process,
                    &discretization,
                    &initial_state,
                    &payoff,
                    ccy,
                    discount_factor,
                )?
            }
        };

        Ok((result.mean, result.stderr))
    }
}

impl Default for EquityOptionHestonMcPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for EquityOptionHestonMcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::EquityOption, ModelKey::MonteCarloHeston)
    }

    #[tracing::instrument(
        name = "equity_option.heston_mc.price_dyn",
        level = "debug",
        skip(self, instrument, market),
        fields(
            inst_id = %instrument.id(),
            as_of = %as_of,
            num_paths = self.num_paths,
        ),
        err,
    )]
    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let equity_option = instrument
            .as_any()
            .downcast_ref::<EquityOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::EquityOption, instrument.key())
            })?;

        let (pv, stderr) = self
            .price_internal(equity_option, market, as_of)
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::from_instrument(equity_option)
                        .model(ModelKey::MonteCarloHeston),
                )
            })?;

        let mut result = ValuationResult::stamped(equity_option.id(), as_of, pv);
        if stderr > 0.0 {
            result
                .measures
                .insert(crate::metrics::MetricId::custom("mc_stderr"), stderr);
        }
        Ok(result)
    }
}
