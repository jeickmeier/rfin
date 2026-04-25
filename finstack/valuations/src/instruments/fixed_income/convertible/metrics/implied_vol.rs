//! Implied volatility calculator for convertible bonds.
//!
//! Solves for the equity volatility that makes the tree-based model price
//! equal the market-quoted clean price. This is the convertible bond analog
//! of implied volatility for equity options.
//!
//! # Dependencies
//!
//! Requires `quoted_clean_price` in `bond.pricing_overrides.market_quotes`.
//!
//! # Units
//!
//! Returned as a decimal fraction (e.g., 0.25 = 25% volatility).

use std::cell::Cell;

use crate::instruments::fixed_income::convertible::pricer::{
    calculate_accrued_interest, price_convertible_bond, ConvertibleTreeType,
};
use crate::instruments::fixed_income::convertible::ConvertibleBond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::Result;

pub(crate) struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let bond: &ConvertibleBond = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= bond.maturity {
            return Ok(0.0);
        }

        let quoted_clean = bond
            .pricing_overrides
            .market_quotes
            .quoted_clean_price
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::InputError::NotFound {
                    id: "pricing_overrides.market_quotes.quoted_clean_price".to_string(),
                })
            })?;

        let accrued = calculate_accrued_interest(bond, as_of)?;
        let target_dirty = quoted_clean + accrued;

        let tree_type = ConvertibleTreeType::Binomial(100);

        let underlying_id = bond.underlying_equity_id.as_deref().ok_or_else(|| {
            finstack_core::Error::internal("convertible implied vol requires underlying_equity_id")
        })?;

        // Resolve the volatility ID using the same candidate logic as the pricer
        let mut vol_candidates: Vec<String> = Vec::new();
        if let Some(id) = bond.attributes.get_meta("vol_surface_id") {
            vol_candidates.push(id.to_string());
        }
        vol_candidates.push(format!("{}-VOL", underlying_id));
        if let Some(stripped) = underlying_id.strip_suffix("-SPOT") {
            vol_candidates.push(format!("{}-VOL", stripped));
        }

        let vol_id = vol_candidates
            .iter()
            .find(|id| {
                context.curves.get_price(id.as_str()).is_ok()
                    || context.curves.get_surface(id).is_ok()
            })
            .cloned()
            .unwrap_or_else(|| format!("{}-VOL", underlying_id));

        let base_market = context.curves.as_ref();

        // Validate the unbumped pricing path before entering the solver so that
        // missing equity / vol / curve inputs surface their real error messages
        // rather than appearing as opaque solver convergence failures.
        let _ = price_convertible_bond(bond, base_market, tree_type, as_of)?;

        // Capture the first pricing error so a downstream solver failure can
        // report the underlying cause. See the OAS solver for why
        // `take().or(Some(e))` is required instead of `if take().is_none()`.
        let captured_err: Cell<Option<finstack_core::Error>> = Cell::new(None);
        let record_err = |e: finstack_core::Error| {
            let prev = captured_err.take();
            captured_err.set(prev.or(Some(e)));
        };
        let objective = |vol: f64| -> f64 {
            let bumped = base_market
                .clone()
                .insert_price(&vol_id, MarketScalar::Unitless(vol));
            match price_convertible_bond(bond, &bumped, tree_type, as_of) {
                Ok(pv) => pv.amount() - target_dirty,
                Err(e) => {
                    record_err(e);
                    f64::NAN
                }
            }
        };

        let solver = BrentSolver::new()
            .tolerance(1e-6)
            .max_iterations(100)
            .bracket_bounds(0.001, 3.0); // 0.1% to 300% vol

        match solver.solve(objective, 0.25) {
            Ok(implied_vol) => Ok(implied_vol),
            Err(solver_err) => {
                if let Some(inner) = captured_err.take() {
                    Err(finstack_core::Error::Validation(format!(
                        "Convertible implied vol solver failed because pricing failed inside \
                         the objective: {inner}"
                    )))
                } else {
                    Err(solver_err)
                }
            }
        }
    }
}
