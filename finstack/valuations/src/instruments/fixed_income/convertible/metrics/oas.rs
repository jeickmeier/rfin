//! Option-Adjusted Spread (OAS) for convertible bonds.
//!
//! OAS is the constant spread added to the risk-free discount curve such that
//! the tree-based model price equals the market-quoted clean price. It isolates
//! the credit component after removing the value of embedded equity conversion,
//! call, and put options.
//!
//! # Dependencies
//!
//! Requires `quoted_clean_price` in `bond.pricing_overrides.market_quotes`.
//!
//! # Units
//!
//! Returned in **decimal** (e.g., 0.01 = 100bp), consistent with other spread
//! metrics in the library.

use crate::instruments::fixed_income::convertible::pricer::{
    calculate_accrued_interest, price_convertible_bond, ConvertibleTreeType,
};
use crate::instruments::fixed_income::convertible::ConvertibleBond;
use crate::metrics::{bump_discount_curve_parallel, MetricCalculator, MetricContext};
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::Result;

pub struct OasCalculator;

impl MetricCalculator for OasCalculator {
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
        let base_market = context.curves.as_ref();
        let curve_id = &bond.discount_curve_id;

        let objective = |spread: f64| -> f64 {
            let bumped = match bump_discount_curve_parallel(base_market, curve_id, spread) {
                Ok(m) => m,
                Err(_) => return f64::NAN,
            };
            match price_convertible_bond(bond, &bumped, tree_type, as_of) {
                Ok(pv) => pv.amount() - target_dirty,
                Err(_) => f64::NAN,
            }
        };

        let solver = BrentSolver::new()
            .tolerance(1e-8)
            .max_iterations(100)
            .bracket_bounds(-0.10, 0.50); // -1000bp to +5000bp

        let oas = solver.solve(objective, 0.0)?;

        Ok(oas)
    }
}
