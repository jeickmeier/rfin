//! CMO tranche OAS calculation.
//!
//! OAS for CMO tranches requires running the waterfall at multiple
//! spread levels to find the spread that equates model price to market.

use crate::instruments::agency_cmo::pricer::generate_tranche_cashflows;
use crate::instruments::agency_cmo::AgencyCmo;
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;

/// CMO tranche OAS result.
#[derive(Clone, Debug)]
pub struct CmoOasResult {
    /// Option-adjusted spread (decimal)
    pub oas: f64,
    /// Model price at OAS
    pub model_price: f64,
    /// Market price (target)
    pub market_price: f64,
    /// Iterations to converge
    pub iterations: u32,
    /// Whether converged
    pub converged: bool,
}

/// Calculate OAS for a CMO tranche.
///
/// Uses the same Brent's method approach as MBS OAS but with
/// waterfall-generated tranche cashflows.
pub fn calculate_tranche_oas(
    cmo: &AgencyCmo,
    market_price_pct: f64,
    market: &MarketContext,
    as_of: Date,
) -> Result<CmoOasResult> {
    let tranche = cmo.reference_tranche().ok_or_else(|| {
        finstack_core::Error::Validation(format!("Tranche {} not found", cmo.reference_tranche_id))
    })?;

    let market_price = market_price_pct / 100.0 * tranche.current_face.amount();

    // Price function with spread
    let price_at_spread = |spread: f64| -> Result<f64> {
        let tranche_cfs = generate_tranche_cashflows(cmo, as_of, None)?;
        let discount_curve = market.get_discount_ref(&cmo.discount_curve_id)?;
        let day_count = DayCount::Thirty360;

        let mut pv = 0.0;
        for cf in &tranche_cfs {
            let years = day_count.year_fraction(as_of, cf.payment_date, DayCountCtx::default())?;
            let base_df = discount_curve.df(years);
            let spread_adj = (-spread * years).exp();
            let df = base_df * spread_adj;
            pv += cf.total * df;
        }

        Ok(pv)
    };

    // Brent's method
    let mut lower = -0.05;
    let mut upper = 0.10;
    let tolerance = 1e-8;
    let max_iterations = 100;

    let f_lower = price_at_spread(lower)? - market_price;
    let f_upper = price_at_spread(upper)? - market_price;

    if f_lower * f_upper > 0.0 {
        // Try wider bounds
        lower = -0.10;
        upper = 0.20;
    }

    let mut a = lower;
    let mut b = upper;
    let mut fa = price_at_spread(a)? - market_price;
    let mut fb = price_at_spread(b)? - market_price;

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
            let l0 = a * fb * fc / ((fa - fb) * (fa - fc));
            let l1 = b * fa * fc / ((fb - fa) * (fb - fc));
            let l2 = c * fa * fb / ((fc - fa) * (fc - fb));
            l0 + l1 + l2
        } else {
            b - fb * (b - a) / (fb - fa)
        };

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

        let fs = price_at_spread(s)? - market_price;
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
            let final_price = price_at_spread(b)?;
            return Ok(CmoOasResult {
                oas: b,
                model_price: final_price,
                market_price,
                iterations,
                converged: true,
            });
        }
    }

    let final_price = price_at_spread(b)?;
    Ok(CmoOasResult {
        oas: b,
        model_price: final_price,
        market_price,
        iterations,
        converged: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use time::Month;

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
    fn test_tranche_oas() {
        let cmo = AgencyCmo::example();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        // Get model price at zero spread to use as market price
        let tranche_cfs = generate_tranche_cashflows(&cmo, as_of, None).expect("cfs");
        let disc = market
            .get_discount_ref(&cmo.discount_curve_id)
            .expect("curve");
        let day_count = DayCount::Thirty360;

        let mut model_price = 0.0;
        for cf in &tranche_cfs {
            let years = day_count
                .year_fraction(as_of, cf.payment_date, DayCountCtx::default())
                .expect("yf");
            model_price += cf.total * disc.df(years);
        }

        let tranche = cmo.reference_tranche().expect("tranche");
        let price_pct = model_price / tranche.current_face.amount() * 100.0;

        // OAS should be near zero
        let result = calculate_tranche_oas(&cmo, price_pct, &market, as_of).expect("oas");

        assert!(result.oas.abs() < 0.01);
    }
}
