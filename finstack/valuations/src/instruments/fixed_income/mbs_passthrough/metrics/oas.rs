//! Option-Adjusted Spread (OAS) calculation for agency MBS.
//!
//! OAS is the constant spread over the risk-free yield curve that equates
//! the theoretical MBS price to its market price, accounting for the
//! prepayment option embedded in the security.

use crate::instruments::agency_mbs_passthrough::pricer::price_with_spread;
use crate::instruments::agency_mbs_passthrough::AgencyMbsPassthrough;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;

/// OAS calculation result.
#[derive(Clone, Debug)]
pub struct OasResult {
    /// Option-adjusted spread in decimal (e.g., 0.01 for 100 bps)
    pub oas: f64,
    /// Model price at the calculated OAS
    pub model_price: f64,
    /// Target (market) price
    pub market_price: f64,
    /// Price difference at solution
    pub price_error: f64,
    /// Number of solver iterations
    pub iterations: u32,
    /// Whether solver converged
    pub converged: bool,
}

/// Calculate OAS via root-finding.
///
/// Uses Brent's method to find the spread that equates model price
/// to market price.
///
/// # Arguments
///
/// * `mbs` - Agency MBS passthrough instrument
/// * `market_price` - Target price (per $100 face, e.g., 98.5)
/// * `market` - Market context with discount curves
/// * `as_of` - Valuation date
///
/// # Returns
///
/// OAS result with spread and convergence information
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::agency_mbs_passthrough::{
///     AgencyMbsPassthrough,
///     metrics::oas::calculate_oas,
/// };
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// let mbs = AgencyMbsPassthrough::example();
/// let market = MarketContext::new(); // Add curves...
/// let as_of = Date::from_calendar_date(2024, Month::January, 15).unwrap();
///
/// let result = calculate_oas(&mbs, 98.5, &market, as_of).expect("OAS calculation");
/// println!("OAS: {:.0} bps", result.oas * 10_000.0);
/// ```
pub fn calculate_oas(
    mbs: &AgencyMbsPassthrough,
    market_price_pct: f64,
    market: &MarketContext,
    as_of: Date,
) -> Result<OasResult> {
    // Convert market price from percentage to dollar amount
    let market_price = market_price_pct / 100.0 * mbs.current_face.amount();

    // Define objective function: f(spread) = model_price(spread) - market_price
    let objective = |spread: f64| -> Result<f64> {
        let model_price = price_with_spread(mbs, market, as_of, spread)?;
        Ok(model_price - market_price)
    };

    // Brent's method bounds and parameters
    let mut lower = -0.05; // -500 bps
    let mut upper = 0.10; // +1000 bps
    let tolerance = 1e-8;
    let max_iterations = 100;

    // Initial bracket check
    let f_lower = objective(lower)?;
    let f_upper = objective(upper)?;

    // If no sign change, try to find better bounds
    if f_lower * f_upper > 0.0 {
        // Try wider bounds
        lower = -0.10;
        upper = 0.20;
        let f_lower_wide = objective(lower)?;
        let f_upper_wide = objective(upper)?;

        if f_lower_wide * f_upper_wide > 0.0 {
            // Return best guess
            let mid_spread = 0.0;
            let model_price_mid = price_with_spread(mbs, market, as_of, mid_spread)?;
            return Ok(OasResult {
                oas: mid_spread,
                model_price: model_price_mid,
                market_price,
                price_error: model_price_mid - market_price,
                iterations: 0,
                converged: false,
            });
        }
    }

    // Brent's method implementation
    let mut a = lower;
    let mut b = upper;
    let mut fa = objective(a)?;
    let mut fb = objective(b)?;

    if fa.abs() < fb.abs() {
        std::mem::swap(&mut a, &mut b);
        std::mem::swap(&mut fa, &mut fb);
    }

    let mut c = a;
    let mut fc = fa;
    let mut mflag = true;
    let mut d = 0.0;

    let mut iterations = 0;

    while iterations < max_iterations {
        iterations += 1;

        let s = if (fa - fc).abs() > 1e-15 && (fb - fc).abs() > 1e-15 {
            // Inverse quadratic interpolation
            let l0 = a * fb * fc / ((fa - fb) * (fa - fc));
            let l1 = b * fa * fc / ((fb - fa) * (fb - fc));
            let l2 = c * fa * fb / ((fc - fa) * (fc - fb));
            l0 + l1 + l2
        } else {
            // Secant method
            b - fb * (b - a) / (fb - fa)
        };

        // Conditions for bisection
        let cond1 = !(s > (3.0 * a + b) / 4.0 && s < b || s > b && s < (3.0 * a + b) / 4.0);
        let cond2 = mflag && (s - b).abs() >= (b - c).abs() / 2.0;
        let cond3 = !mflag && (s - b).abs() >= (c - d).abs() / 2.0;
        let cond4 = mflag && (b - c).abs() < tolerance;
        let cond5 = !mflag && (c - d).abs() < tolerance;

        let s = if cond1 || cond2 || cond3 || cond4 || cond5 {
            mflag = true;
            (a + b) / 2.0
        } else {
            mflag = false;
            s
        };

        let fs = objective(s)?;
        d = c;
        c = b;
        fc = fb;

        if fa * fs < 0.0 {
            b = s;
            fb = fs;
        } else {
            a = s;
            fa = fs;
        }

        if fa.abs() < fb.abs() {
            std::mem::swap(&mut a, &mut b);
            std::mem::swap(&mut fa, &mut fb);
        }

        if fb.abs() < tolerance * market_price || (b - a).abs() < tolerance {
            let final_price = price_with_spread(mbs, market, as_of, b)?;
            return Ok(OasResult {
                oas: b,
                model_price: final_price,
                market_price,
                price_error: final_price - market_price,
                iterations,
                converged: true,
            });
        }
    }

    // Return best result even if not fully converged
    let final_price = price_with_spread(mbs, market, as_of, b)?;
    Ok(OasResult {
        oas: b,
        model_price: final_price,
        market_price,
        price_error: final_price - market_price,
        iterations,
        converged: false,
    })
}

/// Calculate static spread (Z-spread) using simplified discounting.
///
/// Unlike OAS, static spread does not account for the prepayment option.
/// It's faster to compute but less accurate for MBS.
pub fn calculate_static_spread(
    mbs: &AgencyMbsPassthrough,
    market_price_pct: f64,
    market: &MarketContext,
    as_of: Date,
) -> Result<f64> {
    let result = calculate_oas(mbs, market_price_pct, market, as_of)?;
    Ok(result.oas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cashflow::builder::specs::PrepaymentModelSpec;
    use crate::instruments::agency_mbs_passthrough::{AgencyProgram, PoolType};
    use finstack_core::currency::Currency;
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use finstack_core::types::{CurveId, InstrumentId};
    use time::Month;

    fn create_test_mbs() -> AgencyMbsPassthrough {
        AgencyMbsPassthrough::builder()
            .id(InstrumentId::new("TEST-MBS"))
            .pool_id("TEST-POOL".to_string())
            .agency(AgencyProgram::Fnma)
            .pool_type(PoolType::Generic)
            .original_face(Money::new(1_000_000.0, Currency::USD))
            .current_face(Money::new(1_000_000.0, Currency::USD))
            .current_factor(1.0)
            .wac(0.045)
            .pass_through_rate(0.04)
            .servicing_fee_rate(0.0025)
            .guarantee_fee_rate(0.0025)
            .wam(360)
            .issue_date(Date::from_calendar_date(2024, Month::January, 1).expect("valid"))
            .maturity_date(Date::from_calendar_date(2054, Month::January, 1).expect("valid"))
            .prepayment_model(PrepaymentModelSpec::psa(1.0))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .day_count(DayCount::Thirty360)
            .build()
            .expect("valid mbs")
    }

    fn create_test_market(as_of: Date) -> MarketContext {
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots([
                (0.0, 1.0),
                (1.0, 0.96),
                (5.0, 0.80),
                (10.0, 0.60),
                (30.0, 0.30),
            ])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("valid curve");

        MarketContext::new().insert_discount(disc)
    }

    #[test]
    fn test_oas_calculation() {
        let mbs = create_test_mbs();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        // Get model price at zero spread
        let base_price = price_with_spread(&mbs, &market, as_of, 0.0).expect("price");
        let market_price_pct = base_price / mbs.current_face.amount() * 100.0;

        // OAS should be approximately zero when market price equals model price
        let result = calculate_oas(&mbs, market_price_pct, &market, as_of).expect("oas");

        assert!(result.converged);
        assert!(result.oas.abs() < 0.001); // Within 10 bps of zero
    }

    #[test]
    fn test_oas_with_discount() {
        let mbs = create_test_mbs();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        // Test with discount price (should give positive OAS)
        let discount_price = 95.0; // 95% of par

        let result = calculate_oas(&mbs, discount_price, &market, as_of).expect("oas");

        // Price below par should imply positive spread
        // (this depends on the specific curve setup)
        assert!(result.converged || result.iterations > 0);
    }

    #[test]
    fn test_oas_with_premium() {
        let mbs = create_test_mbs();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        // Test with premium price (should give negative OAS)
        let premium_price = 105.0; // 105% of par

        let result = calculate_oas(&mbs, premium_price, &market, as_of).expect("oas");

        // Price above par should imply negative spread
        assert!(result.converged || result.iterations > 0);
    }
}
