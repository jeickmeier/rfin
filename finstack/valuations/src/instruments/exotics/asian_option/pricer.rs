//! Asian option pricers (Monte Carlo and analytical).

// Common imports for all pricers
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::exotics::asian_option::types::AsianOption;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

// MC-specific imports
#[cfg(feature = "mc")]
use finstack_monte_carlo::engine::PathCaptureConfig;
#[cfg(feature = "mc")]
use finstack_monte_carlo::estimate::Estimate;
#[cfg(feature = "mc")]
use finstack_monte_carlo::payoff::asian::{AsianCall, AsianPut};
#[cfg(feature = "mc")]
use finstack_monte_carlo::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};
#[cfg(feature = "mc")]
use finstack_monte_carlo::process::gbm::{GbmParams, GbmProcess};
#[cfg(feature = "mc")]
use finstack_monte_carlo::results::MoneyEstimate;
#[cfg(feature = "mc")]
use finstack_monte_carlo::variance_reduction::control_variate::{
    apply_control_variate, covariance,
};

/// Asian option Monte Carlo pricer.
#[cfg(feature = "mc")]
pub struct AsianOptionMcPricer {
    config: PathDependentPricerConfig,
}

#[cfg(feature = "mc")]
impl AsianOptionMcPricer {
    /// Create a new Asian option MC pricer with default config.
    pub fn new() -> Self {
        Self {
            config: PathDependentPricerConfig::default(),
        }
    }

    fn merged_path_config(&self, inst: &AsianOption) -> PathDependentPricerConfig {
        let mut c = self.config.clone();
        if let Some(n) = inst.pricing_overrides.model_config.mc_paths {
            if n > 0 {
                c.num_paths = n;
            }
        }
        c
    }

    /// Price an Asian option using Monte Carlo.
    fn price_internal(
        &self,
        inst: &AsianOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        // Get time to maturity
        let t = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

        let (hist_sum, hist_prod_log, hist_count) = inst.accumulated_state(as_of);

        if t <= 0.0 {
            // Expired: use realized average
            let average = if hist_count > 0 {
                match inst.averaging_method {
                    crate::instruments::exotics::asian_option::types::AveragingMethod::Arithmetic => {
                        hist_sum / hist_count as f64
                    }
                    crate::instruments::exotics::asian_option::types::AveragingMethod::Geometric => {
                        (hist_prod_log / hist_count as f64).exp()
                    }
                }
            } else {
                // Fallback
                let spot_scalar = curves.get_price(&inst.spot_id)?;
                match spot_scalar {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                    finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
                }
            };

            let intrinsic = match inst.option_type {
                crate::instruments::OptionType::Call => (average - inst.strike).max(0.0),
                crate::instruments::OptionType::Put => (inst.strike - average).max(0.0),
            };
            return Ok(Money::new(
                intrinsic * inst.notional.amount(),
                inst.notional.currency(),
            ));
        }

        // Get discount curve
        let disc_curve = curves.get_discount(inst.discount_curve_id.as_str())?;
        let discount_factor = disc_curve.df_between_dates(as_of, inst.expiry)?;
        // Keep drift consistent with date-based discounting: exp(-r * t) == DF(as_of, maturity).
        let r = if t > 0.0 && discount_factor > 0.0 {
            -discount_factor.ln() / t
        } else {
            0.0
        };

        // Get spot
        let spot_scalar = curves.get_price(&inst.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        // Get dividend yield
        let q = crate::instruments::common_impl::helpers::resolve_optional_dividend_yield(
            curves,
            inst.div_yield_id.as_ref(),
        )?;

        // Get volatility
        let vol_surface = curves.get_surface(inst.vol_surface_id.as_str())?;
        let sigma = vol_surface.value_clamped(t, inst.strike);

        // Create GBM process
        let gbm_params = GbmParams::new(r, q, sigma);
        let process = GbmProcess::new(gbm_params);

        let base_cfg = self.merged_path_config(inst);

        // Map fixing dates to time steps
        // Use configurable steps per year with a minimum cap
        let steps_per_year = base_cfg.steps_per_year;
        let num_steps = ((t * steps_per_year).round() as usize).max(base_cfg.min_steps);
        let mut fixing_steps = Vec::new();
        for &fixing_date in &inst.fixing_dates {
            let fixing_t =
                inst.day_count
                    .year_fraction(as_of, fixing_date, DayCountCtx::default())?;
            if fixing_t > 0.0 && fixing_t <= t {
                let step = (fixing_t / t * num_steps as f64).round() as usize;
                let clamped = step.min(num_steps.saturating_sub(1)).max(0);
                fixing_steps.push(clamped);
            }
        }
        fixing_steps.sort();
        fixing_steps.dedup();

        // Create payoff
        let averaging = match inst.averaging_method {
            crate::instruments::exotics::asian_option::types::AveragingMethod::Arithmetic => {
                finstack_monte_carlo::payoff::asian::AveragingMethod::Arithmetic
            }
            crate::instruments::exotics::asian_option::types::AveragingMethod::Geometric => {
                finstack_monte_carlo::payoff::asian::AveragingMethod::Geometric
            }
        };

        // Derive deterministic seed from instrument ID and scenario
        #[cfg(feature = "mc")]
        use finstack_monte_carlo::seed;

        let seed = if let Some(ref scenario) = inst.pricing_overrides.metrics.mc_seed_scenario {
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
            base_cfg.seed
        };

        // Create config with derived seed
        let mut config = base_cfg;
        config.seed = seed;

        // If arithmetic averaging, apply geometric-Asian control variate for variance reduction
        let result_money = match (inst.averaging_method, inst.option_type) {
            (
                crate::instruments::exotics::asian_option::types::AveragingMethod::Arithmetic,
                crate::instruments::OptionType::Call,
            ) => {
                // Use path capture to get per-path discounted payoffs for covariance
                let mut cfg_cap = config.clone();
                cfg_cap.path_capture = PathCaptureConfig::all().with_payoffs();
                let pricer_cap = PathDependentPricer::new(cfg_cap);

                // Arithmetic payoff
                let arith_payoff = AsianCall::with_history(
                    inst.strike,
                    inst.notional.amount(),
                    finstack_monte_carlo::payoff::asian::AveragingMethod::Arithmetic,
                    fixing_steps.clone(),
                    hist_sum,
                    hist_prod_log,
                    hist_count,
                );
                let arith_full = pricer_cap.price_with_paths(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &arith_payoff,
                    inst.notional.currency(),
                    discount_factor,
                )?;

                // Geometric payoff (same RNG via same seed)
                let geom_payoff = AsianCall::with_history(
                    inst.strike,
                    inst.notional.amount(),
                    finstack_monte_carlo::payoff::asian::AveragingMethod::Geometric,
                    fixing_steps.clone(),
                    hist_sum,
                    hist_prod_log,
                    hist_count,
                );
                let geom_full = pricer_cap.price_with_paths(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &geom_payoff,
                    inst.notional.currency(),
                    discount_factor,
                )?;

                // Extract per-path discounted payoffs
                // paths should be Some when path_capture is enabled in price_with_paths
                let xs: Vec<f64> = arith_full
                    .paths
                    .as_ref()
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Path capture enabled but paths not captured".into(),
                        )
                    })?
                    .paths
                    .iter()
                    .map(|p| p.final_value)
                    .collect();
                let ys: Vec<f64> = geom_full
                    .paths
                    .as_ref()
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Path capture enabled but paths not captured".into(),
                        )
                    })?
                    .paths
                    .iter()
                    .map(|p| p.final_value)
                    .collect();

                let n = xs.len();
                // Sample means
                let mean_x = xs.iter().sum::<f64>() / n as f64;
                let mean_y = ys.iter().sum::<f64>() / n as f64;
                // Sample variances (unbiased)
                let var_x = if n > 1 {
                    xs.iter().map(|v| (v - mean_x).powi(2)).sum::<f64>() / (n as f64 - 1.0)
                } else {
                    0.0
                };
                let var_y = if n > 1 {
                    ys.iter().map(|v| (v - mean_y).powi(2)).sum::<f64>() / (n as f64 - 1.0)
                } else {
                    0.0
                };
                // Covariance
                let cov_xy = covariance(&xs, &ys);

                // Analytical value of geometric Asian (control)
                // Use standard formula if not seasoned, otherwise we should ideally use adjusted.
                // But since our Geometric Analytical doesn't support seasoning, we can't use CV safely?
                // Actually, if we are seasoned, we can't use the analytical control variate unless we implement the adjustment.
                // If hist_count > 0, we should probably disable CV or implement the adjustment.
                // For now, let's disable CV if seasoned to avoid wrong adjustment.

                if hist_count > 0 {
                    // Fallback to plain MC if seasoned (CV disabled)
                    MoneyEstimate::from_estimate(
                        Estimate {
                            mean: mean_x,
                            stderr: (var_x / n as f64).sqrt(),
                            ci_95: (
                                mean_x - 1.96 * (var_x / n as f64).sqrt(),
                                mean_x + 1.96 * (var_x / n as f64).sqrt(),
                            ),
                            num_paths: n,
                            std_dev: None,
                            median: None,
                            percentile_25: None,
                            percentile_75: None,
                            min: None,
                            max: None,
                            num_skipped: 0,
                        },
                        inst.notional.currency(),
                    )
                    .mean
                } else {
                    let control_analytical = geometric_asian_call(
                        spot,
                        inst.strike,
                        t,
                        r,
                        q,
                        sigma,
                        inst.fixing_dates.len(),
                    );

                    let adj = apply_control_variate(
                        mean_x,
                        var_x,
                        mean_y,
                        var_y,
                        cov_xy,
                        control_analytical,
                        n,
                    );
                    MoneyEstimate::from_estimate(adj, inst.notional.currency()).mean
                }
            }
            (
                crate::instruments::exotics::asian_option::types::AveragingMethod::Arithmetic,
                crate::instruments::OptionType::Put,
            ) => {
                let mut cfg_cap = config.clone();
                cfg_cap.path_capture = PathCaptureConfig::all().with_payoffs();
                let pricer_cap = PathDependentPricer::new(cfg_cap);

                let arith_payoff = AsianPut::with_history(
                    inst.strike,
                    inst.notional.amount(),
                    finstack_monte_carlo::payoff::asian::AveragingMethod::Arithmetic,
                    fixing_steps.clone(),
                    hist_sum,
                    hist_prod_log,
                    hist_count,
                );
                let arith_full = pricer_cap.price_with_paths(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &arith_payoff,
                    inst.notional.currency(),
                    discount_factor,
                )?;

                let geom_payoff = AsianPut::with_history(
                    inst.strike,
                    inst.notional.amount(),
                    finstack_monte_carlo::payoff::asian::AveragingMethod::Geometric,
                    fixing_steps.clone(),
                    hist_sum,
                    hist_prod_log,
                    hist_count,
                );
                let geom_full = pricer_cap.price_with_paths(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &geom_payoff,
                    inst.notional.currency(),
                    discount_factor,
                )?;

                // paths should be Some when path_capture is enabled in price_with_paths
                let xs: Vec<f64> = arith_full
                    .paths
                    .as_ref()
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Path capture enabled but paths not captured".into(),
                        )
                    })?
                    .paths
                    .iter()
                    .map(|p| p.final_value)
                    .collect();
                let ys: Vec<f64> = geom_full
                    .paths
                    .as_ref()
                    .ok_or_else(|| {
                        finstack_core::Error::Validation(
                            "Path capture enabled but paths not captured".into(),
                        )
                    })?
                    .paths
                    .iter()
                    .map(|p| p.final_value)
                    .collect();
                let n = xs.len();
                let mean_x = xs.iter().sum::<f64>() / n as f64;
                let mean_y = ys.iter().sum::<f64>() / n as f64;
                let var_x = if n > 1 {
                    xs.iter().map(|v| (v - mean_x).powi(2)).sum::<f64>() / (n as f64 - 1.0)
                } else {
                    0.0
                };
                let var_y = if n > 1 {
                    ys.iter().map(|v| (v - mean_y).powi(2)).sum::<f64>() / (n as f64 - 1.0)
                } else {
                    0.0
                };
                let cov_xy = covariance(&xs, &ys);

                if hist_count > 0 {
                    MoneyEstimate::from_estimate(
                        Estimate {
                            mean: mean_x,
                            stderr: (var_x / n as f64).sqrt(),
                            ci_95: (
                                mean_x - 1.96 * (var_x / n as f64).sqrt(),
                                mean_x + 1.96 * (var_x / n as f64).sqrt(),
                            ),
                            num_paths: n,
                            std_dev: None,
                            median: None,
                            percentile_25: None,
                            percentile_75: None,
                            min: None,
                            max: None,
                            num_skipped: 0,
                        },
                        inst.notional.currency(),
                    )
                    .mean
                } else {
                    let control_analytical = geometric_asian_put(
                        spot,
                        inst.strike,
                        t,
                        r,
                        q,
                        sigma,
                        inst.fixing_dates.len(),
                    );

                    let adj = apply_control_variate(
                        mean_x,
                        var_x,
                        mean_y,
                        var_y,
                        cov_xy,
                        control_analytical,
                        n,
                    );
                    MoneyEstimate::from_estimate(adj, inst.notional.currency()).mean
                }
            }
            // Geometric averaging (no CV needed) or fallback path
            _ => {
                let pricer = PathDependentPricer::new(config);
                match inst.option_type {
                    crate::instruments::OptionType::Call => {
                        let payoff = AsianCall::with_history(
                            inst.strike,
                            inst.notional.amount(),
                            averaging,
                            fixing_steps,
                            hist_sum,
                            hist_prod_log,
                            hist_count,
                        );
                        pricer
                            .price(
                                &process,
                                spot,
                                t,
                                num_steps,
                                &payoff,
                                inst.notional.currency(),
                                discount_factor,
                            )?
                            .mean
                    }
                    crate::instruments::OptionType::Put => {
                        let payoff = AsianPut::with_history(
                            inst.strike,
                            inst.notional.amount(),
                            averaging,
                            fixing_steps,
                            hist_sum,
                            hist_prod_log,
                            hist_count,
                        );
                        pricer
                            .price(
                                &process,
                                spot,
                                t,
                                num_steps,
                                &payoff,
                                inst.notional.currency(),
                                discount_factor,
                            )?
                            .mean
                    }
                }
            }
        };

        Ok(result_money)
    }

    /// Price with LRM Greeks (delta, vega) convenience.
    #[allow(clippy::too_many_lines)]
    #[allow(dead_code)] // May be used by external bindings or tests
    pub(crate) fn price_with_lrm_greeks_internal(
        &self,
        inst: &AsianOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<(Money, Option<(f64, f64)>)> {
        // Reuse the same setup as price_internal
        let t = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

        let (hist_sum, hist_prod_log, hist_count) = inst.accumulated_state(as_of);

        if t <= 0.0 {
            // Expired, return 0 value/greeks (or intrinsic if we care about cashflows, but greeks usually 0)
            // For simplicity, we return 0 value if expired in LRM greeks context as sensitivity is 0.
            // Or should we return intrinsic? Since this is pricing + greeks, we should return value.
            let average = if hist_count > 0 {
                match inst.averaging_method {
                    crate::instruments::exotics::asian_option::types::AveragingMethod::Arithmetic => {
                        hist_sum / hist_count as f64
                    }
                    crate::instruments::exotics::asian_option::types::AveragingMethod::Geometric => {
                        (hist_prod_log / hist_count as f64).exp()
                    }
                }
            } else {
                let spot_scalar = curves.get_price(&inst.spot_id)?;
                match spot_scalar {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                    finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
                }
            };
            let intrinsic = match inst.option_type {
                crate::instruments::OptionType::Call => (average - inst.strike).max(0.0),
                crate::instruments::OptionType::Put => (inst.strike - average).max(0.0),
            };

            return Ok((
                Money::new(intrinsic * inst.notional.amount(), inst.notional.currency()),
                None,
            ));
        }

        let disc_curve = curves.get_discount(inst.discount_curve_id.as_str())?;
        let discount_factor = disc_curve.df_between_dates(as_of, inst.expiry)?;
        // Keep drift consistent with date-based discounting: exp(-r * t) == DF(as_of, maturity).
        let r = if t > 0.0 && discount_factor > 0.0 {
            -discount_factor.ln() / t
        } else {
            0.0
        };

        let spot_scalar = curves.get_price(&inst.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let q = crate::instruments::common_impl::helpers::resolve_optional_dividend_yield(
            curves,
            inst.div_yield_id.as_ref(),
        )?;

        let vol_surface = curves.get_surface(inst.vol_surface_id.as_str())?;
        let sigma = vol_surface.value_clamped(t, inst.strike);

        let gbm_params = GbmParams::new(r, q, sigma);
        let process = GbmProcess::new(gbm_params);

        let base_cfg = self.merged_path_config(inst);

        let steps_per_year = base_cfg.steps_per_year;
        let num_steps = ((t * steps_per_year).round() as usize).max(base_cfg.min_steps);

        // Fixing steps mapping
        let mut fixing_steps = Vec::new();
        for &fixing_date in &inst.fixing_dates {
            let fixing_t =
                inst.day_count
                    .year_fraction(as_of, fixing_date, DayCountCtx::default())?;
            if fixing_t > 0.0 && fixing_t <= t {
                let step = (fixing_t / t * num_steps as f64).round() as usize;
                let clamped = step.min(num_steps.saturating_sub(1)).max(0);
                fixing_steps.push(clamped);
            }
        }

        fixing_steps.sort();
        fixing_steps.dedup();

        let averaging = match inst.averaging_method {
            crate::instruments::exotics::asian_option::types::AveragingMethod::Arithmetic => {
                finstack_monte_carlo::payoff::asian::AveragingMethod::Arithmetic
            }
            crate::instruments::exotics::asian_option::types::AveragingMethod::Geometric => {
                finstack_monte_carlo::payoff::asian::AveragingMethod::Geometric
            }
        };

        // Seed handling
        #[cfg(feature = "mc")]
        use finstack_monte_carlo::seed;
        let seed = if let Some(ref scenario) = inst.pricing_overrides.metrics.mc_seed_scenario {
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
            base_cfg.seed
        };
        let mut cfg = base_cfg;
        cfg.seed = seed;
        let pricer = PathDependentPricer::new(cfg);

        let (est, greeks) = match inst.option_type {
            crate::instruments::OptionType::Call => {
                let payoff = finstack_monte_carlo::payoff::asian::AsianCall::with_history(
                    inst.strike,
                    inst.notional.amount(),
                    averaging,
                    fixing_steps,
                    hist_sum,
                    hist_prod_log,
                    hist_count,
                );
                pricer.price_with_lrm_greeks(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &payoff,
                    inst.notional.currency(),
                    discount_factor,
                    r,
                    q,
                    sigma,
                )?
            }
            crate::instruments::OptionType::Put => {
                let payoff = finstack_monte_carlo::payoff::asian::AsianPut::with_history(
                    inst.strike,
                    inst.notional.amount(),
                    averaging,
                    fixing_steps,
                    hist_sum,
                    hist_prod_log,
                    hist_count,
                );
                pricer.price_with_lrm_greeks(
                    &process,
                    spot,
                    t,
                    num_steps,
                    &payoff,
                    inst.notional.currency(),
                    discount_factor,
                    r,
                    q,
                    sigma,
                )?
            }
        };

        Ok((est.mean, greeks))
    }
}

#[cfg(feature = "mc")]
impl Default for AsianOptionMcPricer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "mc")]
impl Pricer for AsianOptionMcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::AsianOption, ModelKey::MonteCarloGBM)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let asian = instrument
            .as_any()
            .downcast_ref::<AsianOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::AsianOption, instrument.key())
            })?;

        let pv = self.price_internal(asian, market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(asian.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
#[cfg(feature = "mc")]
pub(crate) fn compute_pv(
    inst: &AsianOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<Money> {
    let pricer = AsianOptionMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}

/// Present value with LRM Greeks via Monte Carlo.
#[allow(dead_code)] // May be used by external bindings or tests
#[cfg(feature = "mc")]
pub fn npv_with_lrm_greeks(
    inst: &AsianOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<(Money, Option<(f64, f64)>)> {
    let pricer = AsianOptionMcPricer::new();
    pricer.price_with_lrm_greeks_internal(inst, curves, as_of)
}

// ========================= ANALYTICAL PRICERS =========================

use crate::instruments::common_impl::helpers::collect_black_scholes_inputs;
use crate::instruments::common_impl::models::closed_form::asian::{
    arithmetic_asian_call_tw, arithmetic_asian_put_tw, geometric_asian_call, geometric_asian_put,
};

/// Geometric Asian option analytical pricer.
pub struct AsianOptionAnalyticalGeometricPricer;

impl AsianOptionAnalyticalGeometricPricer {
    /// Create a new analytical geometric Asian option pricer
    pub fn new() -> Self {
        Self
    }
}

impl Default for AsianOptionAnalyticalGeometricPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for AsianOptionAnalyticalGeometricPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::AsianOption, ModelKey::AsianGeometricBS)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let asian = instrument
            .as_any()
            .downcast_ref::<AsianOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::AsianOption, instrument.key())
            })?;

        // Use standardized input collection
        let (spot, r, q, sigma, t) = collect_black_scholes_inputs(
            &asian.spot_id,
            &asian.discount_curve_id,
            asian.div_yield_id.as_ref(),
            &asian.vol_surface_id,
            asian.strike,
            asian.expiry,
            asian.day_count,
            market,
            as_of,
        )
        .map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        let (sum, log_prod, count) = asian.accumulated_state(as_of);
        let total_fixings = asian.fixing_dates.len();

        if t <= 0.0 {
            // Handle expired option using realized average
            let average = if count > 0 {
                match asian.averaging_method {
                    crate::instruments::exotics::asian_option::types::AveragingMethod::Arithmetic => {
                        sum / count as f64
                    }
                    crate::instruments::exotics::asian_option::types::AveragingMethod::Geometric => {
                        (log_prod / count as f64).exp()
                    }
                }
            } else {
                // Fallback if no fixings recorded (unlikely for expired option)
                spot
            };

            let intrinsic = match asian.option_type {
                crate::instruments::OptionType::Call => (average - asian.strike).max(0.0),
                crate::instruments::OptionType::Put => (asian.strike - average).max(0.0),
            };
            return Ok(ValuationResult::stamped(
                asian.id(),
                as_of,
                Money::new(
                    intrinsic * asian.notional.amount(),
                    asian.notional.currency(),
                ),
            ));
        }

        // Seasoned Geometric Asian requires adjusted strike formula not yet implemented.
        // The adjustment involves: K_eff = (n·K - G_past) / (n - m) where G_past is the
        // geometric average of past fixings. For now, fall back to Monte Carlo.
        if count > 0 {
            return Err(PricingError::model_failure_with_context(
                format!(
                    "Seasoned Geometric Asian analytical pricing not supported ({} of {} fixings already observed). \
                    For seasoned options, use Monte Carlo pricing via `npv_mc()` method or set \
                    `averaging_method = Arithmetic` which supports seasoning via Turnbull-Wakeman.",
                    count, total_fixings
                ),
                PricingErrorContext::default(),
            ));
        }

        let price = match asian.option_type {
            crate::instruments::OptionType::Call => {
                geometric_asian_call(spot, asian.strike, t, r, q, sigma, total_fixings)
            }
            crate::instruments::OptionType::Put => {
                geometric_asian_put(spot, asian.strike, t, r, q, sigma, total_fixings)
            }
        };

        let pv = Money::new(price * asian.notional.amount(), asian.notional.currency());
        Ok(ValuationResult::stamped(asian.id(), as_of, pv))
    }
}

/// Arithmetic Asian option semi-analytical pricer (Turnbull-Wakeman).
pub struct AsianOptionSemiAnalyticalTwPricer;

impl AsianOptionSemiAnalyticalTwPricer {
    /// Create a new Turnbull-Wakeman approximation Asian option pricer
    pub fn new() -> Self {
        Self
    }
}

impl Default for AsianOptionSemiAnalyticalTwPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for AsianOptionSemiAnalyticalTwPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::AsianOption, ModelKey::AsianTurnbullWakeman)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        use crate::instruments::common_impl::helpers::collect_black_scholes_inputs_df;

        let asian = instrument
            .as_any()
            .downcast_ref::<AsianOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::AsianOption, instrument.key())
            })?;

        // Use DF-based input collection for time-consistent discounting
        let bs_inputs = collect_black_scholes_inputs_df(
            &asian.spot_id,
            &asian.discount_curve_id,
            asian.div_yield_id.as_ref(),
            &asian.vol_surface_id,
            asian.strike,
            asian.expiry,
            asian.day_count,
            market,
            as_of,
        )
        .map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        let spot = bs_inputs.spot;
        let df_expiry = bs_inputs.df;
        let q = bs_inputs.q;
        let sigma = bs_inputs.sigma;
        let t = bs_inputs.t;
        // Derive r_eff for TW formula (which still needs a rate for moment calculations)
        let r = bs_inputs.r_eff();

        let (sum, _, count) = asian.accumulated_state(as_of);
        let total_fixings = asian.fixing_dates.len();

        if t <= 0.0 {
            let average = if count > 0 { sum / count as f64 } else { spot };
            let intrinsic = match asian.option_type {
                crate::instruments::OptionType::Call => (average - asian.strike).max(0.0),
                crate::instruments::OptionType::Put => (asian.strike - average).max(0.0),
            };
            return Ok(ValuationResult::stamped(
                asian.id(),
                as_of,
                Money::new(
                    intrinsic * asian.notional.amount(),
                    asian.notional.currency(),
                ),
            ));
        }

        let future_fixings = total_fixings.saturating_sub(count);
        if future_fixings == 0 {
            // Deterministic case (all fixings past, but not expired?)
            let average = sum / total_fixings as f64;
            let payoff = match asian.option_type {
                crate::instruments::OptionType::Call => (average - asian.strike).max(0.0),
                crate::instruments::OptionType::Put => (asian.strike - average).max(0.0),
            };
            // Use the date-based DF from inputs
            return Ok(ValuationResult::stamped(
                asian.id(),
                as_of,
                Money::new(
                    payoff * df_expiry * asian.notional.amount(),
                    asian.notional.currency(),
                ),
            ));
        }

        let n = total_fixings as f64;
        let m = future_fixings as f64;
        let k = asian.strike;

        let numerator = n * k - sum;
        let k_eff = numerator / m;
        let scale = m / n;

        let price = if k_eff < 0.0 {
            match asian.option_type {
                crate::instruments::OptionType::Call => {
                    // Deep ITM: PV = DF_T * (Expected_Avg - K)
                    // Expected_Avg = (Past_Sum + Sum(F_i)) / N
                    // F_i = forward price for fixing date i
                    //     = S * exp(-q*t_i) / df_i  (GK forward formula)
                    //
                    // We compute each forward using date-based DFs for consistency.
                    let disc_curve = market
                        .get_discount(asian.discount_curve_id.as_str())
                        .map_err(|e| {
                            PricingError::model_failure_with_context(
                                e.to_string(),
                                PricingErrorContext::default(),
                            )
                        })?;

                    let mut sum_fwd = 0.0;
                    for date in &asian.fixing_dates {
                        if *date > as_of {
                            let t_i = asian
                                .day_count
                                .year_fraction(as_of, *date, DayCountCtx::default())
                                .map_err(|e| {
                                    PricingError::model_failure_with_context(
                                        e.to_string(),
                                        PricingErrorContext::default(),
                                    )
                                })?;
                            // Get date-based DF for this fixing
                            let df_i = disc_curve.df_between_dates(as_of, *date).map_err(|e| {
                                PricingError::model_failure_with_context(
                                    e.to_string(),
                                    PricingErrorContext::default(),
                                )
                            })?;
                            // GK forward: F_i = S * exp(-q*t_i) / df_i
                            let forward_i = spot * (-q * t_i).exp() / df_i;
                            sum_fwd += forward_i;
                        }
                    }
                    let expected_avg = (sum + sum_fwd) / n;
                    // Use date-based DF for final discounting
                    (expected_avg - k).max(0.0) * df_expiry
                }
                crate::instruments::OptionType::Put => {
                    // Deep ITM put (k_eff < 0): past fixings already push average above K.
                    // Compute expected average using forwards and return discounted payoff.
                    // PV = DF_T * max(K - Expected_Avg, 0)
                    let disc_curve = market
                        .get_discount(asian.discount_curve_id.as_str())
                        .map_err(|e| {
                            PricingError::model_failure_with_context(
                                e.to_string(),
                                PricingErrorContext::default(),
                            )
                        })?;

                    let mut sum_fwd = 0.0;
                    for date in &asian.fixing_dates {
                        if *date > as_of {
                            let t_i = asian
                                .day_count
                                .year_fraction(as_of, *date, DayCountCtx::default())
                                .map_err(|e| {
                                    PricingError::model_failure_with_context(
                                        e.to_string(),
                                        PricingErrorContext::default(),
                                    )
                                })?;
                            let df_i = disc_curve.df_between_dates(as_of, *date).map_err(|e| {
                                PricingError::model_failure_with_context(
                                    e.to_string(),
                                    PricingErrorContext::default(),
                                )
                            })?;
                            let forward_i = spot * (-q * t_i).exp() / df_i;
                            sum_fwd += forward_i;
                        }
                    }
                    let expected_avg = (sum + sum_fwd) / n;
                    (k - expected_avg).max(0.0) * df_expiry
                }
            }
        } else {
            let unscaled = match asian.option_type {
                crate::instruments::OptionType::Call => {
                    arithmetic_asian_call_tw(spot, k_eff, t, r, q, sigma, future_fixings)
                }
                crate::instruments::OptionType::Put => {
                    arithmetic_asian_put_tw(spot, k_eff, t, r, q, sigma, future_fixings)
                }
            };
            unscaled * scale
        };

        let pv = Money::new(price * asian.notional.amount(), asian.notional.currency());
        Ok(ValuationResult::stamped(asian.id(), as_of, pv))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::models::closed_form::asian::{
        geometric_asian_call, geometric_asian_put,
    };
    use crate::instruments::exotics::asian_option::{AsianOption, AveragingMethod};
    use crate::instruments::OptionType;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{Date, DayCount, DayCountCtx};
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::market_data::surfaces::VolSurface;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::types::{CurveId, InstrumentId};
    use time::Month;

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, Month::try_from(month).expect("valid month"), day)
            .expect("valid date")
    }

    fn market(as_of: Date, spot: f64, vol: f64, rate: f64, div_yield: f64) -> MarketContext {
        let discount = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 1.0), (5.0, (-rate * 5.0).exp())])
            .interp(InterpStyle::LogLinear)
            .build()
            .expect("discount curve");

        let vol_surface = VolSurface::builder("SPX-VOL")
            .expiries(&[0.25, 0.5, 1.0, 2.0])
            .strikes(&[80.0, 90.0, 100.0, 110.0, 120.0])
            .row(&[vol, vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol, vol])
            .build()
            .expect("vol surface");

        MarketContext::new()
            .insert(discount)
            .insert_surface(vol_surface)
            .insert_price("SPX-SPOT", MarketScalar::Unitless(spot))
            .insert_price("SPX-DIV", MarketScalar::Unitless(div_yield))
    }

    fn market_without_vol(as_of: Date, spot: f64, rate: f64, div_yield: f64) -> MarketContext {
        let discount = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 1.0), (5.0, (-rate * 5.0).exp())])
            .interp(InterpStyle::LogLinear)
            .build()
            .expect("discount curve");

        MarketContext::new()
            .insert(discount)
            .insert_price("SPX-SPOT", MarketScalar::Unitless(spot))
            .insert_price("SPX-DIV", MarketScalar::Unitless(div_yield))
    }

    fn asian_option(
        averaging: AveragingMethod,
        option_type: OptionType,
        expiry: Date,
        strike: f64,
        fixing_dates: Vec<Date>,
    ) -> AsianOption {
        AsianOption::builder()
            .id(InstrumentId::new("ASIAN-TEST"))
            .underlying_ticker("SPX".to_string())
            .strike(strike)
            .option_type(option_type)
            .averaging_method(averaging)
            .expiry(expiry)
            .fixing_dates(fixing_dates)
            .notional(Money::new(1.0, Currency::USD))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".into())
            .vol_surface_id(CurveId::new("SPX-VOL"))
            .div_yield_id_opt(Some(CurveId::new("SPX-DIV")))
            .pricing_overrides(Default::default())
            .attributes(Default::default())
            .build()
            .expect("asian option")
    }

    #[test]
    fn geometric_pricer_matches_kemna_vorst_call_benchmark() {
        let as_of = date(2025, 1, 2);
        let expiry = date(2026, 1, 2);
        let fixing_dates = vec![
            date(2025, 4, 2),
            date(2025, 7, 2),
            date(2025, 10, 2),
            date(2026, 1, 2),
        ];

        let spot = 100.0;
        let strike = 100.0;
        let vol = 0.20;
        let rate = 0.05;
        let div_yield = 0.00;

        let market = market(as_of, spot, vol, rate, div_yield);
        let option = asian_option(
            AveragingMethod::Geometric,
            OptionType::Call,
            expiry,
            strike,
            fixing_dates.clone(),
        );

        let pv = option.value(&market, as_of).expect("asian pv").amount();

        let t = option
            .day_count
            .year_fraction(as_of, expiry, DayCountCtx::default())
            .expect("year fraction");
        let expected =
            geometric_asian_call(spot, strike, t, rate, div_yield, vol, fixing_dates.len());
        let expected_money = Money::new(expected, Currency::USD).amount();

        assert!((pv - expected_money).abs() < 1e-12);
    }

    #[test]
    fn geometric_pricer_matches_kemna_vorst_put_benchmark() {
        let as_of = date(2025, 1, 2);
        let expiry = date(2026, 1, 2);
        let fixing_dates = vec![date(2025, 6, 2), date(2026, 1, 2)];

        let spot = 100.0;
        let strike = 110.0;
        let vol = 0.25;
        let rate = 0.03;
        let div_yield = 0.01;

        let market = market(as_of, spot, vol, rate, div_yield);
        let option = asian_option(
            AveragingMethod::Geometric,
            OptionType::Put,
            expiry,
            strike,
            fixing_dates.clone(),
        );

        let pv = option.value(&market, as_of).expect("asian pv").amount();

        let t = option
            .day_count
            .year_fraction(as_of, expiry, DayCountCtx::default())
            .expect("year fraction");
        let expected =
            geometric_asian_put(spot, strike, t, rate, div_yield, vol, fixing_dates.len());
        let expected_money = Money::new(expected, Currency::USD).amount();

        assert!((pv - expected_money).abs() < 1e-12);
    }

    #[test]
    fn turnbull_wakeman_respects_fully_realized_average_payoff() {
        let as_of = date(2025, 7, 1);
        let expiry = date(2025, 12, 31);
        let fixing_dates = vec![
            date(2025, 1, 31),
            date(2025, 2, 28),
            date(2025, 3, 31),
            date(2025, 4, 30),
            date(2025, 5, 31),
            date(2025, 6, 30),
        ];

        let mut option = asian_option(
            AveragingMethod::Arithmetic,
            OptionType::Call,
            expiry,
            100.0,
            fixing_dates.clone(),
        );
        option.past_fixings = fixing_dates
            .iter()
            .copied()
            .zip([102.0, 101.0, 103.0, 104.0, 100.0, 105.0])
            .collect();

        let market = market(as_of, 100.0, 0.20, 0.05, 0.0);
        let pv = option.value(&market, as_of).expect("asian pv").amount();

        let average = [102.0, 101.0, 103.0, 104.0, 100.0, 105.0]
            .iter()
            .sum::<f64>()
            / fixing_dates.len() as f64;
        let payoff = (average - 100.0).max(0.0);
        let df = market.get_discount("USD-OIS").expect("discount").df(option
            .day_count
            .year_fraction(as_of, expiry, DayCountCtx::default())
            .expect("year fraction"));
        let expected_money = Money::new(payoff * df, Currency::USD).amount();

        assert!((pv - expected_money).abs() < 1e-12);
    }

    #[cfg(feature = "mc")]
    #[test]
    fn mc_pricer_expired_uses_realized_arithmetic_average() {
        let as_of = date(2025, 6, 30);
        let fixing_dates = vec![date(2025, 4, 30), date(2025, 5, 31), date(2025, 6, 30)];
        let mut option = asian_option(
            AveragingMethod::Arithmetic,
            OptionType::Call,
            as_of,
            100.0,
            fixing_dates.clone(),
        );
        option.past_fixings = fixing_dates
            .iter()
            .copied()
            .zip([90.0, 110.0, 120.0])
            .collect();

        let pv = AsianOptionMcPricer::new()
            .price_internal(&option, &market(as_of, 999.0, 0.20, 0.05, 0.0), as_of)
            .expect("expired MC price")
            .amount();

        let expected = ((90.0_f64 + 110.0 + 120.0) / 3.0 - 100.0).max(0.0);
        assert!((pv - expected).abs() < 1e-12);
    }

    #[cfg(feature = "mc")]
    #[test]
    fn mc_pricer_expired_without_fixings_falls_back_to_spot() {
        let as_of = date(2025, 6, 30);
        let option = asian_option(
            AveragingMethod::Geometric,
            OptionType::Put,
            as_of,
            100.0,
            vec![date(2025, 3, 31), date(2025, 6, 30)],
        );

        let pv = AsianOptionMcPricer::new()
            .price_internal(&option, &market(as_of, 80.0, 0.20, 0.05, 0.0), as_of)
            .expect("expired MC price")
            .amount();

        assert!((pv - 20.0).abs() < 1e-12);
    }

    #[cfg(feature = "mc")]
    #[test]
    fn expired_lrm_wrapper_returns_intrinsic_and_no_greeks() {
        let as_of = date(2025, 6, 30);
        let fixing_dates = vec![date(2025, 5, 31), date(2025, 6, 30)];
        let mut option = asian_option(
            AveragingMethod::Arithmetic,
            OptionType::Put,
            as_of,
            100.0,
            fixing_dates.clone(),
        );
        option.past_fixings = fixing_dates.iter().copied().zip([95.0, 85.0]).collect();

        let (pv, greeks) =
            npv_with_lrm_greeks(&option, &market(as_of, 999.0, 0.20, 0.05, 0.0), as_of)
                .expect("expired lrm price");

        assert!((pv.amount() - 10.0).abs() < 1e-12);
        assert_eq!(greeks, None);
    }

    #[cfg(feature = "mc")]
    #[test]
    fn mc_price_dyn_wraps_market_data_errors() {
        let as_of = date(2025, 1, 2);
        let expiry = date(2025, 7, 2);
        let option = asian_option(
            AveragingMethod::Arithmetic,
            OptionType::Call,
            expiry,
            100.0,
            vec![date(2025, 3, 2), expiry],
        );

        let err = AsianOptionMcPricer::new()
            .price_dyn(&option, &market_without_vol(as_of, 100.0, 0.05, 0.0), as_of)
            .expect_err("missing vol surface should be wrapped");
        assert!(err.to_string().contains("SPX-VOL"));
    }

    #[test]
    fn analytical_geometric_expired_without_fixings_falls_back_to_spot() {
        let as_of = date(2025, 6, 30);
        let option = asian_option(
            AveragingMethod::Geometric,
            OptionType::Call,
            as_of,
            100.0,
            vec![date(2025, 3, 31), date(2025, 6, 30)],
        );

        let pv = AsianOptionAnalyticalGeometricPricer::new()
            .price_dyn(&option, &market(as_of, 125.0, 0.20, 0.05, 0.0), as_of)
            .expect("expired analytical price")
            .value
            .amount();

        assert!((pv - 25.0).abs() < 1e-12);
    }

    #[test]
    fn analytical_geometric_price_dyn_wraps_market_data_errors() {
        let as_of = date(2025, 1, 2);
        let expiry = date(2025, 7, 2);
        let option = asian_option(
            AveragingMethod::Geometric,
            OptionType::Call,
            expiry,
            100.0,
            vec![date(2025, 3, 2), expiry],
        );

        let err = AsianOptionAnalyticalGeometricPricer::new()
            .price_dyn(&option, &market_without_vol(as_of, 100.0, 0.05, 0.0), as_of)
            .expect_err("missing vol surface should be wrapped");
        assert!(err.to_string().contains("SPX-VOL"));
    }

    #[test]
    fn analytical_geometric_rejects_seasoned_option() {
        let as_of = date(2025, 7, 1);
        let expiry = date(2025, 12, 31);
        let fixing_dates = vec![
            date(2025, 1, 31),
            date(2025, 3, 31),
            date(2025, 6, 30),
            date(2025, 9, 30),
            expiry,
        ];

        let mut option = asian_option(
            AveragingMethod::Geometric,
            OptionType::Call,
            expiry,
            100.0,
            fixing_dates.clone(),
        );
        option.past_fixings = fixing_dates
            .iter()
            .take(2)
            .copied()
            .zip([101.0, 103.0])
            .collect();

        let err = AsianOptionAnalyticalGeometricPricer::new()
            .price_dyn(&option, &market(as_of, 100.0, 0.20, 0.05, 0.0), as_of)
            .expect_err("seasoned geometric analytical pricing should be rejected");
        assert!(err
            .to_string()
            .contains("Seasoned Geometric Asian analytical pricing not supported"));
    }

    #[test]
    fn turnbull_wakeman_all_fixings_past_put_discounts_deterministic_payoff() {
        let as_of = date(2025, 7, 1);
        let expiry = date(2025, 12, 31);
        let fixing_dates = vec![
            date(2025, 1, 31),
            date(2025, 2, 28),
            date(2025, 3, 31),
            date(2025, 4, 30),
            date(2025, 5, 31),
            date(2025, 6, 30),
        ];

        let mut option = asian_option(
            AveragingMethod::Arithmetic,
            OptionType::Put,
            expiry,
            100.0,
            fixing_dates.clone(),
        );
        option.past_fixings = fixing_dates
            .iter()
            .copied()
            .zip([92.0, 95.0, 97.0, 96.0, 94.0, 98.0])
            .collect();

        let market = market(as_of, 100.0, 0.20, 0.05, 0.0);
        let pv = AsianOptionSemiAnalyticalTwPricer::new()
            .price_dyn(&option, &market, as_of)
            .expect("deterministic TW price")
            .value
            .amount();

        let average =
            [92.0, 95.0, 97.0, 96.0, 94.0, 98.0].iter().sum::<f64>() / fixing_dates.len() as f64;
        let payoff = (100.0 - average).max(0.0);
        let df = market.get_discount("USD-OIS").expect("discount").df(option
            .day_count
            .year_fraction(as_of, expiry, DayCountCtx::default())
            .expect("year fraction"));
        let expected = Money::new(payoff * df, Currency::USD).amount();

        assert!((pv - expected).abs() < 1e-12);
    }

    #[test]
    fn turnbull_wakeman_negative_effective_strike_call_uses_forward_average_branch() {
        let as_of = date(2025, 4, 2);
        let expiry = date(2025, 10, 2);
        let fixing_dates = vec![
            date(2025, 1, 2),
            date(2025, 2, 2),
            date(2025, 3, 2),
            date(2025, 4, 2),
            date(2025, 5, 2),
            date(2025, 6, 2),
            date(2025, 7, 2),
            date(2025, 8, 2),
            date(2025, 9, 2),
            date(2025, 10, 2),
        ];

        let mut option = asian_option(
            AveragingMethod::Arithmetic,
            OptionType::Call,
            expiry,
            100.0,
            fixing_dates.clone(),
        );
        option.past_fixings = fixing_dates
            .iter()
            .take(4)
            .copied()
            .zip([250.0, 240.0, 260.0, 255.0])
            .collect();

        let market = market(as_of, 100.0, 0.20, 0.05, 0.01);
        let pv = AsianOptionSemiAnalyticalTwPricer::new()
            .price_dyn(&option, &market, as_of)
            .expect("deep ITM TW price")
            .value
            .amount();

        let disc_curve = market.get_discount("USD-OIS").expect("discount");
        let df_expiry = disc_curve.df(option
            .day_count
            .year_fraction(as_of, expiry, DayCountCtx::default())
            .expect("year fraction"));
        let spot = 100.0;
        let q = 0.01;
        let sum_past = [250.0, 240.0, 260.0, 255.0].iter().sum::<f64>();
        let n = fixing_dates.len() as f64;
        let mut sum_fwd = 0.0;
        for date in fixing_dates.iter().copied().filter(|d| *d > as_of) {
            let t_i = option
                .day_count
                .year_fraction(as_of, date, DayCountCtx::default())
                .expect("fixing year fraction");
            let df_i = disc_curve
                .df_between_dates(as_of, date)
                .expect("date-based discount factor");
            sum_fwd += spot * (-q * t_i).exp() / df_i;
        }
        let expected_avg = (sum_past + sum_fwd) / n;
        let expected = Money::new(
            (expected_avg - option.strike).max(0.0) * df_expiry,
            Currency::USD,
        )
        .amount();

        assert!((pv - expected).abs() < 1e-12);
    }

    #[test]
    fn turnbull_wakeman_negative_effective_strike_put_clamps_to_zero() {
        let as_of = date(2025, 4, 2);
        let expiry = date(2025, 10, 2);
        let fixing_dates = vec![
            date(2025, 1, 2),
            date(2025, 2, 2),
            date(2025, 3, 2),
            date(2025, 4, 2),
            date(2025, 5, 2),
            date(2025, 6, 2),
            date(2025, 7, 2),
            date(2025, 8, 2),
            date(2025, 9, 2),
            date(2025, 10, 2),
        ];

        let mut option = asian_option(
            AveragingMethod::Arithmetic,
            OptionType::Put,
            expiry,
            100.0,
            fixing_dates.clone(),
        );
        option.past_fixings = fixing_dates
            .iter()
            .take(4)
            .copied()
            .zip([250.0, 240.0, 260.0, 255.0])
            .collect();

        let pv = AsianOptionSemiAnalyticalTwPricer::new()
            .price_dyn(&option, &market(as_of, 100.0, 0.20, 0.05, 0.01), as_of)
            .expect("deep OTM TW put should price")
            .value
            .amount();

        assert_eq!(pv, 0.0);
    }

    #[cfg(feature = "mc")]
    #[test]
    fn mc_pricer_expired_without_fixings_uses_price_scalar_spot_fallback() {
        let as_of = date(2025, 6, 30);
        let option = asian_option(
            AveragingMethod::Arithmetic,
            OptionType::Call,
            as_of,
            100.0,
            vec![date(2025, 3, 31), date(2025, 6, 30)],
        );

        let market = market(as_of, 80.0, 0.20, 0.05, 0.0).insert_price(
            "SPX-SPOT",
            MarketScalar::Price(Money::new(125.0, Currency::USD)),
        );

        let pv = AsianOptionMcPricer::new()
            .price_internal(&option, &market, as_of)
            .expect("expired MC price")
            .amount();

        assert!((pv - 25.0).abs() < 1e-12);
    }

    #[test]
    fn turnbull_wakeman_price_dyn_wraps_market_data_errors() {
        let as_of = date(2025, 1, 2);
        let expiry = date(2025, 7, 2);
        let option = asian_option(
            AveragingMethod::Arithmetic,
            OptionType::Put,
            expiry,
            100.0,
            vec![date(2025, 3, 2), expiry],
        );

        let err = AsianOptionSemiAnalyticalTwPricer::new()
            .price_dyn(&option, &market_without_vol(as_of, 100.0, 0.05, 0.0), as_of)
            .expect_err("missing vol surface should be wrapped");
        assert!(err.to_string().contains("SPX-VOL"));
    }
}
