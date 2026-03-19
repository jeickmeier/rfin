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

use crate::instruments::fixed_income::convertible::pricer::{
    calculate_accrued_interest, price_convertible_bond, ConvertibleTreeType,
};
use crate::instruments::fixed_income::convertible::ConvertibleBond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::Result;

pub struct ImpliedVolCalculator;

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

        let underlying_id = bond
            .underlying_equity_id
            .as_deref()
            .ok_or_else(|| {
                finstack_core::Error::internal(
                    "convertible implied vol requires underlying_equity_id",
                )
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

        let objective = |vol: f64| -> f64 {
            let bumped = base_market
                .clone()
                .insert_price(&vol_id, MarketScalar::Unitless(vol));
            match price_convertible_bond(bond, &bumped, tree_type, as_of) {
                Ok(pv) => pv.amount() - target_dirty,
                Err(_) => f64::NAN,
            }
        };

        let solver = BrentSolver::new()
            .tolerance(1e-6)
            .max_iterations(100)
            .bracket_bounds(0.001, 3.0); // 0.1% to 300% vol

        let implied_vol = solver.solve(objective, 0.25)?;

        Ok(implied_vol)
    }
}
