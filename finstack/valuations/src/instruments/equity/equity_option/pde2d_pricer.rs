//! Equity option Heston PDE pricer using 2D Craig-Sneyd ADI finite differences.
//!
//! Solves the Heston PDE in (log-spot, variance) coordinates on a tensor-product
//! grid using the Craig-Sneyd ADI splitting scheme. Heston model parameters are
//! sourced from market scalars with sensible defaults.

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

use crate::instruments::common_impl::models::pde::{Grid1D, Grid2D, HestonPde, Solver2D};
use crate::instruments::common_impl::parameters::OptionType;

/// Equity option pricer using 2D ADI PDE (Craig-Sneyd) with Heston stochastic
/// volatility dynamics.
///
/// Solves the Heston PDE on a tensor-product (log-spot x variance) grid.
/// Heston parameters are read from market scalars using the same convention
/// as [`EquityOptionHestonFourierPricer`].
pub(crate) struct EquityOptionHestonPdePricer {
    /// Number of spatial grid points along the x (log-spot) axis.
    space_points_x: usize,
    /// Number of spatial grid points along the v (variance) axis.
    space_points_v: usize,
    /// Number of time steps.
    time_steps: usize,
}

impl Default for EquityOptionHestonPdePricer {
    fn default() -> Self {
        Self {
            space_points_x: 200,
            space_points_v: 80,
            time_steps: 100,
        }
    }
}

use crate::instruments::common_impl::helpers::get_unitless_scalar;

impl EquityOptionHestonPdePricer {
    /// Price the equity option via the 2D Heston PDE.
    fn price_internal(
        &self,
        inst: &EquityOption,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<Money, PricingError> {
        let inputs = collect_inputs_extended(inst, market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;
        let spot = inputs.spot;
        let r = inputs.r;
        let q = inputs.q;
        let t = inputs.t_vol;
        let ccy = inst.notional.currency();

        if t <= 0.0 {
            let intrinsic = match inst.option_type {
                OptionType::Call => (spot - inst.strike).max(0.0),
                OptionType::Put => (inst.strike - spot).max(0.0),
            };
            return Ok(Money::new(intrinsic * inst.notional.amount(), ccy));
        }

        // Fetch Heston parameters from market data (same convention as Fourier pricer)
        let kappa = get_unitless_scalar(market, "HESTON_KAPPA", 2.0);
        let theta_v = get_unitless_scalar(market, "HESTON_THETA", 0.04);
        let sigma_v = get_unitless_scalar(market, "HESTON_SIGMA_V", 0.3);
        let rho = get_unitless_scalar(market, "HESTON_RHO", -0.7);
        let v0 = get_unitless_scalar(market, "HESTON_V0", 0.04);

        // Validate Heston parameters
        if kappa <= 0.0 || theta_v <= 0.0 || sigma_v <= 0.0 || v0 <= 0.0 {
            return Err(PricingError::model_failure_with_context(
                format!(
                    "Invalid Heston parameters: kappa={kappa}, theta={theta_v}, \
                     sigma_v={sigma_v}, v0={v0} — all must be positive"
                ),
                PricingErrorContext::default(),
            ));
        }
        if rho <= -1.0 || rho >= 1.0 {
            return Err(PricingError::model_failure_with_context(
                format!("Invalid Heston correlation rho={rho} — must be in (-1, 1)"),
                PricingErrorContext::default(),
            ));
        }

        let is_call = matches!(inst.option_type, OptionType::Call);

        let pde = HestonPde {
            r,
            q,
            kappa,
            theta_v,
            sigma_v,
            rho,
            strike: inst.strike,
            is_call,
        };

        // X-grid: log-spot concentrated near ln(strike)
        let x_min = (spot * 0.05).ln();
        let x_max = (spot * 10.0).ln();
        let gx = Grid1D::sinh_concentrated(x_min, x_max, self.space_points_x, spot.ln(), 0.1)
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        // V-grid: variance from near-zero to well above long-run level
        let v_min = 0.001;
        let v_max = 1.5_f64.max(5.0 * theta_v);
        let gv = Grid1D::sinh_concentrated(v_min, v_max, self.space_points_v, theta_v, 0.15)
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        let grid = Grid2D::new(gx, gv);

        let solver = Solver2D::builder()
            .grid(grid)
            .craig_sneyd(self.time_steps)
            .build()
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        let solution = solver.solve(&pde, t);
        let price = solution.interpolate(spot.ln(), v0);

        Ok(Money::new(price * inst.notional.amount(), ccy))
    }
}

impl Pricer for EquityOptionHestonPdePricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::EquityOption, ModelKey::PdeAdi2D)
    }

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

        let pv = self.price_internal(equity_option, market, as_of)?;

        Ok(ValuationResult::stamped(equity_option.id(), as_of, pv))
    }
}
