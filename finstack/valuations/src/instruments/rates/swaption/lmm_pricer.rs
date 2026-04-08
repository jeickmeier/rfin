//! Bermudan swaption pricer using LMM/BGM Monte Carlo dynamics.
//!
//! Wraps the standalone [`price_bermudan_lmm`] engine in the [`Pricer`] trait
//! so it can be dispatched via the pricing registry under
//! `(BermudanSwaption, LmmMonteCarlo)`.

use crate::instruments::common_impl::helpers::year_fraction;
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::rates::swaption::pricing::lmm_bermudan::{
    price_bermudan_lmm, LmmBermudanConfig,
};
use crate::instruments::rates::swaption::BermudanSwaption;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::traits::Discounting;
use finstack_core::money::Money;
use finstack_monte_carlo::process::lmm::LmmParams;

/// Bermudan swaption pricer using LMM/BGM Monte Carlo with LSMC exercise.
///
/// Builds [`LmmParams`] from the swaption's underlying swap schedule and
/// market discount curve, then delegates to [`price_bermudan_lmm`] for
/// LSMC-based Bermudan exercise valuation.
///
/// # Parameter Construction
///
/// Forward rates are bootstrapped from the discount curve at the swap's
/// fixed-leg tenor schedule.  A flat 2-factor loading structure is used
/// (principal component approximation with linear decay), which provides
/// a reasonable starting point for uncalibrated pricing.  For production
/// use, factor loadings should be calibrated to the swaption volatility
/// surface.
#[derive(Default)]
pub struct BermudanSwaptionLmmPricer {
    config: LmmBermudanConfig,
}

impl BermudanSwaptionLmmPricer {
    /// Build LMM parameters from a Bermudan swaption and its discount curve.
    ///
    /// Constructs the tenor schedule from the fixed-leg frequency, bootstraps
    /// forward rates from discount factors, and applies a flat 2-factor
    /// loading structure with linear decay. Base volatility is read from the
    /// swaption's vol surface (ATM at midpoint expiry) when available,
    /// falling back to 12% if the surface is missing.
    fn build_lmm_params(
        swaption: &BermudanSwaption,
        disc: &dyn Discounting,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<LmmParams, PricingError> {
        let swap_start_yf =
            year_fraction(swaption.day_count, as_of, swaption.swap_start).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;
        let swap_end_yf =
            year_fraction(swaption.day_count, as_of, swaption.swap_end).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        // Determine the accrual period from the fixed leg frequency
        let tenor_months = swaption.fixed_freq.months().unwrap_or(6) as f64;
        let period = tenor_months / 12.0;
        if period <= 0.0 {
            return Err(PricingError::model_failure_with_context(
                "Fixed leg frequency must be positive".to_string(),
                PricingErrorContext::default(),
            ));
        }

        // Build tenor schedule from swap_start to swap_end
        let mut tenors: Vec<f64> = Vec::new();
        let mut t = swap_start_yf;
        while t < swap_end_yf - 1e-10 {
            tenors.push(t);
            t += period;
        }
        tenors.push(swap_end_yf);

        let num_forwards = tenors.len() - 1;
        if num_forwards == 0 {
            return Err(PricingError::model_failure_with_context(
                "LMM requires at least one forward rate period".to_string(),
                PricingErrorContext::default(),
            ));
        }

        // Accrual factors: tau_i = T_{i+1} - T_i
        let accrual_factors: Vec<f64> = tenors.windows(2).map(|w| w[1] - w[0]).collect();

        // Bootstrap forward rates from discount factors:
        //   F_i = (DF(T_i) / DF(T_{i+1}) - 1) / tau_i
        let mut initial_forwards: Vec<f64> = Vec::with_capacity(num_forwards);
        for i in 0..num_forwards {
            let df_start = disc.df(tenors[i]);
            let df_end = disc.df(tenors[i + 1]);
            let tau = accrual_factors[i];
            let fwd = if df_end > 1e-15 && tau > 1e-15 {
                (df_start / df_end - 1.0) / tau
            } else {
                0.03 // fallback
            };
            initial_forwards.push(fwd);
        }

        // Zero displacement (no negative-rate shift)
        let displacements = vec![0.005; num_forwards];

        // Flat 2-factor loading structure with linear decay:
        //   lambda_i = [base * (1 - alpha * i/N), base * alpha * i/N, 0]
        // This approximates the first two principal components of swaption
        // correlation matrices.
        //
        // Base vol is read from the swaption's vol surface (ATM at the
        // midpoint tenor), falling back to 12% if unavailable.
        let mid_tenor = 0.5 * (swap_start_yf + swap_end_yf);
        let atm_forward = initial_forwards
            .get(num_forwards / 2)
            .copied()
            .unwrap_or(0.03);
        let base_vol = market
            .get_surface(swaption.vol_surface_id.as_str())
            .ok()
            .map(|surf| surf.value_clamped(mid_tenor, atm_forward))
            .unwrap_or(0.12);
        let alpha = 0.4; // decay parameter
        let mut vol_row: Vec<[f64; 3]> = Vec::with_capacity(num_forwards);
        for i in 0..num_forwards {
            let frac = i as f64 / num_forwards.max(1) as f64;
            let l1 = base_vol * (1.0 - alpha * frac);
            let l2 = base_vol * alpha * frac;
            vol_row.push([l1, l2, 0.0]);
        }
        let vol_values = vec![vol_row]; // single vol period (no breakpoints)
        let vol_times: Vec<f64> = vec![]; // empty => single period

        LmmParams::try_new(
            num_forwards,
            2, // 2-factor model
            tenors,
            accrual_factors,
            displacements,
            vol_times,
            vol_values,
            initial_forwards,
        )
        .map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })
    }
}

impl Pricer for BermudanSwaptionLmmPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::BermudanSwaption, ModelKey::LmmMonteCarlo)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Downcast to BermudanSwaption
        let swaption = instrument
            .as_any()
            .downcast_ref::<BermudanSwaption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::BermudanSwaption, instrument.key())
            })?;

        // Get discount curve
        let disc = market
            .get_discount(swaption.discount_curve_id.as_str())
            .map_err(|e| {
                PricingError::missing_market_data_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        // Check if expired
        let ttm = swaption.time_to_maturity(as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;
        if ttm <= 0.0 {
            return Ok(ValuationResult::stamped(
                swaption.id.as_str(),
                as_of,
                Money::new(0.0, swaption.notional.currency()),
            ));
        }

        // Build LMM parameters from market data
        let lmm_params = Self::build_lmm_params(swaption, disc.as_ref(), market, as_of)?;

        // Extract exercise times as year fractions
        let exercise_times = swaption
            .bermudan_schedule
            .exercise_times(as_of, swaption.day_count)
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        if exercise_times.is_empty() {
            return Ok(ValuationResult::stamped(
                swaption.id.as_str(),
                as_of,
                Money::new(0.0, swaption.notional.currency()),
            ));
        }

        // Strike and payer/receiver flag
        let strike = swaption.strike_f64().map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;
        let is_payer =
            swaption.option_type == crate::instruments::common_impl::parameters::OptionType::Call;
        let notional = swaption.notional.amount();
        let currency = swaption.notional.currency();

        // Terminal discount factor P(0, T_N) for the last tenor
        let t_terminal = lmm_params.tenors.last().copied().unwrap_or(ttm);
        let df_terminal = disc.df(t_terminal);

        // Price via LSMC with LMM dynamics
        let estimate = price_bermudan_lmm(
            &lmm_params,
            &exercise_times,
            strike,
            is_payer,
            notional,
            df_terminal,
            currency,
            &self.config,
        )
        .map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        let mut result = ValuationResult::stamped(swaption.id.as_str(), as_of, estimate.mean);
        if estimate.stderr > 0.0 {
            result.measures.insert(
                crate::metrics::MetricId::custom("mc_stderr"),
                estimate.stderr,
            );
        }
        Ok(result)
    }
}
