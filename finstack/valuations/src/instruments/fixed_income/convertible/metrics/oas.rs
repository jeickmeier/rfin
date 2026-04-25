//! Option-Adjusted Spread (OAS) for convertible bonds.
//!
//! OAS is the constant spread added to the credit/risky discount curve such that
//! the Tsiveriotis-Zhang tree-based model price equals the market-quoted clean
//! price. It isolates the residual credit component after removing the value of
//! embedded equity conversion, call, and put options.
//!
//! When a separate `credit_curve_id` is configured, OAS bumps that curve only
//! (affecting the cash/debt component while leaving equity drift unchanged).
//! When no credit curve is set, the risk-free discount curve is bumped as a
//! fallback, which also shifts the equity component's drift.
//!
//! # Dependencies
//!
//! Requires `quoted_clean_price` in `bond.pricing_overrides.market_quotes`.
//!
//! # Units
//!
//! Returned in **decimal** (e.g., 0.01 = 100bp), consistent with other spread
//! metrics in the library.

use std::cell::Cell;

use crate::instruments::fixed_income::convertible::pricer::{
    calculate_accrued_interest, price_convertible_bond, ConvertibleTreeType,
};
use crate::instruments::fixed_income::convertible::ConvertibleBond;
use crate::metrics::{bump_discount_curve_parallel, MetricCalculator, MetricContext};
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::Result;

pub(crate) struct OasCalculator;

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

        // Bump the credit curve when available (affects cash/debt component only
        // in TZ). Fall back to discount curve when no separate credit curve is set.
        let curve_to_bump = bond
            .credit_curve_id
            .as_ref()
            .unwrap_or(&bond.discount_curve_id);

        // Validate the unbumped pricing path before entering the solver. This
        // surfaces missing curves / vol surfaces / equity IDs with their real
        // error messages instead of letting the solver report opaque "did not
        // converge" failures driven by NaN objective values.
        let _ = price_convertible_bond(bond, base_market, tree_type, as_of)?;

        // Capture the first pricing error from inside the closure so that if the
        // solver bails we can report the underlying cause rather than a generic
        // bracket failure. We keep the *first* error: subsequent failures don't
        // overwrite it. `Cell::take` returns the existing value (clearing it),
        // and `Option::or` keeps the first `Some` while preferring it over a
        // later `Some(e)`. Naively using `if take().is_none() { set(Some(e)) }`
        // would lose the captured error on the second failure (take clears it,
        // is_none() is false, no re-set).
        let captured_err: Cell<Option<finstack_core::Error>> = Cell::new(None);
        let record_err = |e: finstack_core::Error| {
            let prev = captured_err.take();
            captured_err.set(prev.or(Some(e)));
        };
        let objective = |spread: f64| -> f64 {
            let spread_bp = spread * 10_000.0;
            let bumped = match bump_discount_curve_parallel(base_market, curve_to_bump, spread_bp) {
                Ok(m) => m,
                Err(e) => {
                    record_err(e);
                    return f64::NAN;
                }
            };
            match price_convertible_bond(bond, &bumped, tree_type, as_of) {
                Ok(pv) => pv.amount() - target_dirty,
                Err(e) => {
                    record_err(e);
                    f64::NAN
                }
            }
        };

        let solver = BrentSolver::new()
            .tolerance(1e-8)
            .max_iterations(100)
            .bracket_bounds(-0.10, 0.50); // -1000bp to +5000bp in decimal

        match solver.solve(objective, 0.0) {
            Ok(oas) => Ok(oas),
            Err(solver_err) => {
                if let Some(inner) = captured_err.take() {
                    Err(finstack_core::Error::Validation(format!(
                        "Convertible OAS solver failed because pricing failed inside the \
                         objective: {inner}"
                    )))
                } else {
                    Err(solver_err)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    /// Regression test for the Cell-based error capture pattern.
    ///
    /// An earlier version used `if captured_err.take().is_none() { set(Some(e)) }`
    /// which clears the captured value on the second call (take consumes the
    /// existing Some, is_none() is false on the just-cleared Cell, no re-set
    /// happens). The fix is `take().or(Some(e))` so the first error is always
    /// retained across N failures.
    #[test]
    fn record_err_keeps_first_error_across_multiple_failures() {
        use std::cell::Cell;

        let captured_err: Cell<Option<finstack_core::Error>> = Cell::new(None);
        let record_err = |e: finstack_core::Error| {
            let prev = captured_err.take();
            captured_err.set(prev.or(Some(e)));
        };

        // Simulate the solver objective firing three errors in sequence.
        record_err(finstack_core::Error::Validation("first".into()));
        record_err(finstack_core::Error::Validation("second".into()));
        record_err(finstack_core::Error::Validation("third".into()));

        let captured = captured_err.take();
        assert!(captured.is_some(), "first error should be retained");
        let msg = captured.map(|e| e.to_string()).unwrap_or_default();
        assert!(
            msg.contains("first"),
            "expected first error to be preserved, got: {msg}"
        );
    }
}
