//! Enhanced YTM solver with FinancePy-inspired improvements.
//!
//! Provides a robust yield-to-maturity solver using Newton-Raphson with
//! intelligent initial guesses and automatic fallback to Brent's method.

use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::math::root_finding::newton_raphson;
use finstack_core::money::Money;
use finstack_core::{Result, F};
use finstack_core::dates::Frequency;

use super::helpers::{df_and_derivative_from_yield, df_from_yield, YieldCompounding};
/// Pricing specification for YTM solving and pricing from yield.
#[derive(Clone, Copy, Debug)]
pub struct YtmPricingSpec {
    pub day_count: DayCount,
    pub notional: Money,
    pub coupon_rate: F,
    pub compounding: YieldCompounding,
    pub frequency: Frequency,
}

/// Configuration for YTM solver
#[derive(Clone, Debug)]
pub struct YtmSolverConfig {
    /// Tolerance for convergence (FinancePy uses 1e-12)
    pub tolerance: F,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Whether to use smart initial guess
    pub use_smart_guess: bool,
    /// Whether to use Newton-Raphson (vs direct to Brent)
    pub use_newton: bool,
}

impl Default for YtmSolverConfig {
    fn default() -> Self {
        Self {
            tolerance: 1e-12, // FinancePy precision
            max_iterations: 50,
            use_smart_guess: true,
            use_newton: true,
        }
    }
}

/// Enhanced YTM solver with FinancePy-inspired improvements
pub struct YtmSolver {
    config: YtmSolverConfig,
}

impl Default for YtmSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl YtmSolver {
    /// Create a new YTM solver with default configuration
    pub fn new() -> Self {
        Self {
            config: YtmSolverConfig::default(),
        }
    }

    /// Create a YTM solver with custom configuration
    pub fn with_config(config: YtmSolverConfig) -> Self {
        Self { config }
    }

    /// Solve for yield-to-maturity given bond cashflows and target price
    ///
    /// # Arguments
    /// * `cashflows` - Vector of (date, amount) tuples
    /// * `as_of` - Valuation date
    /// * `target_price` - Target dirty price
    /// * `spec` - Pricing spec including day count, notional, coupon, compounding, frequency
    ///
    /// # Returns
    /// The yield-to-maturity that produces the target price
    pub fn solve(
        &self,
        cashflows: &[(Date, Money)],
        as_of: Date,
        target_price: Money,
        spec: YtmPricingSpec,
    ) -> Result<F> {
        let target = target_price.amount();

        // Calculate initial guess
        let initial_guess = if self.config.use_smart_guess {
            self.calculate_initial_guess(
                cashflows,
                as_of,
                target_price,
                spec.day_count,
                spec.notional,
                spec.coupon_rate,
            )?
        } else {
            spec.coupon_rate // Simple fallback
        };

        // Define the price function and its derivative
        let price_fn = |y: f64| -> f64 {
            self.calculate_price(
                cashflows,
                as_of,
                y,
                spec.day_count,
                spec.compounding,
                spec.frequency,
            ) - target
        };

        let price_derivative = |y: f64| -> f64 {
            self.calculate_price_derivative(
                cashflows,
                as_of,
                y,
                spec.day_count,
                spec.compounding,
                spec.frequency,
            )
        };

        if self.config.use_newton {
            // Try Newton-Raphson with automatic fallback
            // Try standard Newton first; if it fails, fallback to Brent
            match newton_raphson(
                price_fn,
                price_derivative,
                initial_guess,
                self.config.tolerance,
                self.config.max_iterations,
            ) {
                Ok(root) => Ok(root),
                Err(_) => {
                    use finstack_core::math::root_finding::brent;
                    let (a, b) = self.determine_bracket(&price_fn, initial_guess);
                    brent(
                        price_fn,
                        a,
                        b,
                        self.config.tolerance,
                        self.config.max_iterations,
                    )
                }
            }
        } else {
            // Direct to Brent's method
            use finstack_core::math::root_finding::brent;

            let (a, b) = self.determine_bracket(&price_fn, initial_guess);
            brent(
                price_fn,
                a,
                b,
                self.config.tolerance,
                self.config.max_iterations,
            )
        }
    }

    /// Calculate price for a given yield
    fn calculate_price(
        &self,
        cashflows: &[(Date, Money)],
        as_of: Date,
        yield_rate: F,
        day_count: DayCount,
        comp: YieldCompounding,
        freq: Frequency,
    ) -> F {
        let mut price = 0.0;

        for &(date, amount) in cashflows {
            if date <= as_of {
                continue;
            }

            let t = DiscountCurve::year_fraction(as_of, date, day_count);
            if t > 0.0 {
                let df = df_from_yield(yield_rate, t, comp, freq);
                price += amount.amount() * df;
            }
        }

        price
    }

    /// Calculate price derivative with respect to yield
    fn calculate_price_derivative(
        &self,
        cashflows: &[(Date, Money)],
        as_of: Date,
        yield_rate: F,
        day_count: DayCount,
        comp: YieldCompounding,
        freq: Frequency,
    ) -> F {
        let mut derivative = 0.0;

        for &(date, amount) in cashflows {
            if date <= as_of {
                continue;
            }

            let t = DiscountCurve::year_fraction(as_of, date, day_count);
            if t > 0.0 {
                let (_df, ddf_dy) = df_and_derivative_from_yield(yield_rate, t, comp, freq);
                derivative += amount.amount() * ddf_dy;
            }
        }

        derivative
    }

    /// Calculate smart initial guess for YTM (FinancePy approach)
    fn calculate_initial_guess(
        &self,
        cashflows: &[(Date, Money)],
        as_of: Date,
        target_price: Money,
        day_count: DayCount,
        notional: Money,
        coupon_rate: F,
    ) -> Result<F> {
        // Method 1: Current yield (coupon / price)
        let current_yield = coupon_rate * notional.amount() / target_price.amount();

        // Method 2: Approximate yield considering maturity
        let maturity = cashflows
            .last()
            .map(|(date, _)| *date)
            .ok_or(finstack_core::error::InputError::TooFewPoints)?;

        let years_to_maturity = DiscountCurve::year_fraction(as_of, maturity, day_count);

        if years_to_maturity <= 0.0 {
            return Ok(current_yield);
        }

        // Calculate approximate yield considering pull-to-par effect
        let price_pct = target_price.amount() / notional.amount();
        let pull_to_par = (1.0 / price_pct - 1.0) / years_to_maturity;

        // Weighted average of current yield and pull-to-par adjustment
        let initial_guess = current_yield + 0.5 * pull_to_par;

        // Bound the initial guess to reasonable range
        Ok(initial_guess.clamp(-0.5, 0.5))
    }

    /// Determine bracket for Brent's method
    fn determine_bracket<Func>(&self, f: &Func, _initial: f64) -> (f64, f64)
    where
        Func: Fn(f64) -> f64,
    {
        // Start with a reasonable bracket for yields
        let mut a = -0.99; // -99% yield (deep discount)
        let mut b = 1.0; // 100% yield

        // Check if initial bracket works
        if f(a) * f(b) < 0.0 {
            return (a, b);
        }

        // Try wider bracket
        a = -0.99;
        b = 5.0; // 500% yield (distressed)

        if f(a) * f(b) < 0.0 {
            return (a, b);
        }

        // Last resort: very wide bracket
        (-0.99, 10.0)
    }
}

/// Convenience function for solving YTM with default configuration
pub fn solve_ytm(
    cashflows: &[(Date, Money)],
    as_of: Date,
    target_price: Money,
    spec: YtmPricingSpec,
) -> Result<F> {
    let solver = YtmSolver::new();
    solver.solve(cashflows, as_of, target_price, spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_ytm_solver_par_bond() {
        // Par bond should have YTM = coupon rate
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let _maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

        let notional = Money::new(1000.0, Currency::USD);
        let coupon_rate = 0.05;

        // Simple annual cashflows
        let mut cashflows = vec![];
        for year in 1..=5 {
            let date = Date::from_calendar_date(2025 + year, Month::January, 1).unwrap();
            if year < 5 {
                cashflows.push((date, Money::new(50.0, Currency::USD))); // Coupon
            } else {
                cashflows.push((date, Money::new(1050.0, Currency::USD))); // Coupon + Principal
            }
        }

        let solver = YtmSolver::new();
        let ytm = solver
            .solve(
                &cashflows,
                as_of,
                notional, // Par price
                YtmPricingSpec {
                    day_count: DayCount::Act365F,
                    notional,
                    coupon_rate,
                    compounding: YieldCompounding::Street,
                    frequency: Frequency::annual(),
                },
            )
            .unwrap();

        assert!(
            (ytm - coupon_rate).abs() < 1e-4,
            "YTM {} should approximately equal coupon rate {}",
            ytm,
            coupon_rate
        );
    }

    #[test]
    fn test_ytm_solver_discount_bond() {
        // Discount bond should have YTM > coupon rate
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let notional = Money::new(1000.0, Currency::USD);
        let coupon_rate = 0.05;
        let discount_price = Money::new(950.0, Currency::USD);

        // Simple cashflows
        let cashflows = vec![
            (
                Date::from_calendar_date(2026, Month::January, 1).unwrap(),
                Money::new(50.0, Currency::USD),
            ),
            (
                Date::from_calendar_date(2027, Month::January, 1).unwrap(),
                Money::new(1050.0, Currency::USD),
            ),
        ];

        let solver = YtmSolver::new();
        let ytm = solver
            .solve(
                &cashflows,
                as_of,
                discount_price,
                YtmPricingSpec {
                    day_count: DayCount::Act365F,
                    notional,
                    coupon_rate,
                    compounding: YieldCompounding::Street,
                    frequency: Frequency::annual(),
                },
            )
            .unwrap();

        assert!(ytm > coupon_rate);
        assert!(ytm < 0.10); // Reasonable bound
    }

    #[test]
    fn test_ytm_solver_premium_bond() {
        // Premium bond should have YTM < coupon rate
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let notional = Money::new(1000.0, Currency::USD);
        let coupon_rate = 0.05;
        let premium_price = Money::new(1050.0, Currency::USD);

        // Simple cashflows
        let cashflows = vec![
            (
                Date::from_calendar_date(2026, Month::January, 1).unwrap(),
                Money::new(50.0, Currency::USD),
            ),
            (
                Date::from_calendar_date(2027, Month::January, 1).unwrap(),
                Money::new(1050.0, Currency::USD),
            ),
        ];

        let solver = YtmSolver::new();
        let ytm = solver
            .solve(
                &cashflows,
                as_of,
                premium_price,
                YtmPricingSpec {
                    day_count: DayCount::Act365F,
                    notional,
                    coupon_rate,
                    compounding: YieldCompounding::Street,
                    frequency: Frequency::annual(),
                },
            )
            .unwrap();

        assert!(ytm < coupon_rate);
        assert!(ytm > 0.0);
    }
}
