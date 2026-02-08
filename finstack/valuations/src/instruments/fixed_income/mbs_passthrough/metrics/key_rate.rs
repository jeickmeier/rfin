//! Key-rate DV01 for agency MBS.
//!
//! Key-rate DV01 measures the sensitivity of MBS price to shifts in
//! specific points along the yield curve, providing insight into
//! curve risk exposure by maturity bucket.
#![allow(dead_code)] // Public API items may be used by external bindings

use crate::instruments::fixed_income::mbs_passthrough::pricer::price_mbs;
use crate::instruments::fixed_income::mbs_passthrough::AgencyMbsPassthrough;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::HashMap;
use finstack_core::Result;

/// Standard key rate tenors for MBS analysis.
pub const STANDARD_TENORS: &[(&str, f64)] = &[
    ("2Y", 2.0),
    ("5Y", 5.0),
    ("10Y", 10.0),
    ("20Y", 20.0),
    ("30Y", 30.0),
];

/// Key-rate DV01 result.
#[derive(Clone, Debug)]
pub struct KeyRateDv01Result {
    /// DV01 by tenor (map from tenor label to DV01 value)
    pub dv01_by_tenor: HashMap<String, f64>,
    /// Total DV01 (sum of key rates)
    pub total_dv01: f64,
    /// Base price used in calculation
    pub base_price: f64,
}

/// Calculate key-rate DV01 for standard tenors.
///
/// Key-rate DV01 measures price sensitivity to a 1 basis point increase
/// at specific points on the yield curve.
///
/// # Arguments
///
/// * `mbs` - Agency MBS passthrough instrument
/// * `market` - Market context with discount curves
/// * `as_of` - Valuation date
///
/// # Returns
///
/// Map from tenor label to DV01 value (negative of price change per 1bp up)
///
/// # Examples
///
/// ```text
/// use finstack_valuations::instruments::fixed_income::mbs_passthrough::{
///     AgencyMbsPassthrough,
///     metrics::key_rate::key_rate_dv01,
/// };
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// let mbs = AgencyMbsPassthrough::example();
/// let market = MarketContext::new(); // Add curves...
/// let as_of = Date::from_calendar_date(2024, Month::January, 15).unwrap();
///
/// let result = key_rate_dv01(&mbs, &market, as_of).expect("key rate dv01");
/// for (tenor, dv01) in &result.dv01_by_tenor {
///     println!("{}: ${:.2}", tenor, dv01);
/// }
/// ```
pub fn key_rate_dv01(
    mbs: &AgencyMbsPassthrough,
    market: &MarketContext,
    as_of: Date,
) -> Result<KeyRateDv01Result> {
    key_rate_dv01_with_tenors(mbs, market, as_of, STANDARD_TENORS)
}

/// Calculate key-rate DV01 for custom tenors.
///
/// # Arguments
///
/// * `mbs` - Agency MBS passthrough instrument
/// * `market` - Market context with discount curves
/// * `as_of` - Valuation date
/// * `tenors` - Slice of (label, years) pairs
pub fn key_rate_dv01_with_tenors(
    mbs: &AgencyMbsPassthrough,
    market: &MarketContext,
    as_of: Date,
    tenors: &[(&str, f64)],
) -> Result<KeyRateDv01Result> {
    let shock_bps = 1.0; // 1 basis point
    let shock = shock_bps / 10_000.0;

    // Get base price
    let base_price = price_mbs(mbs, market, as_of)?.amount();

    let mut dv01_map = HashMap::default();
    let mut total_dv01 = 0.0;

    for (label, tenor_years) in tenors {
        // Create market with bumped key rate
        let bumped_market = bump_key_rate(market, &mbs.discount_curve_id, *tenor_years, shock)?;

        // Get bumped price
        let bumped_price = price_mbs(mbs, &bumped_market, as_of)?.amount();

        // DV01 = -dP for 1bp shock
        let dv01 = -(bumped_price - base_price);

        dv01_map.insert(label.to_string(), dv01);
        total_dv01 += dv01;
    }

    Ok(KeyRateDv01Result {
        dv01_by_tenor: dv01_map,
        total_dv01,
        base_price,
    })
}

/// Bump a single key rate point on the discount curve.
///
/// Uses a triangular bump that peaks at the target tenor and
/// fades to zero at neighboring tenors.
fn bump_key_rate(
    market: &MarketContext,
    curve_id: &finstack_core::types::CurveId,
    target_tenor: f64,
    bump: f64,
) -> Result<MarketContext> {
    let original = market.get_discount(curve_id)?;
    let base_date = original.base_date();
    let day_count = original.day_count();

    // Define standard grid points for interpolation
    let grid_points = [
        0.0, 0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0,
    ];

    // Calculate bump weights using triangular kernel
    let bumped_knots: Vec<(f64, f64)> = grid_points
        .iter()
        .map(|&t| {
            let df = original.df(t);

            // Convert to continuous rate
            let rate = if t > 0.0 { -df.ln() / t } else { 0.0 };

            // Calculate triangular weight for this point
            let weight = triangular_weight(t, target_tenor);

            // Apply bumped rate
            let bumped_rate = rate + bump * weight;
            let bumped_df = if t > 0.0 {
                (-bumped_rate * t).exp()
            } else {
                1.0
            };

            (t, bumped_df)
        })
        .collect();

    let bumped_curve = DiscountCurve::builder(curve_id.as_str())
        .base_date(base_date)
        .day_count(day_count)
        .knots(bumped_knots)
        .interp(finstack_core::math::interp::InterpStyle::Linear)
        .build()?;

    Ok(market.clone().insert_discount(bumped_curve))
}

/// Calculate triangular weight for key rate bump.
///
/// Weight is 1.0 at the target tenor and decays linearly to 0
/// at neighboring tenors.
fn triangular_weight(t: f64, target: f64) -> f64 {
    // Define neighboring tenors for each target
    let (left, right) = match target {
        t if t <= 2.0 => (0.0, 5.0),
        t if t <= 5.0 => (2.0, 10.0),
        t if t <= 10.0 => (5.0, 20.0),
        t if t <= 20.0 => (10.0, 30.0),
        _ => (20.0, 40.0),
    };

    if (t - target).abs() < 1e-6 {
        1.0
    } else if t < target {
        if t <= left {
            0.0
        } else {
            (t - left) / (target - left)
        }
    } else if t >= right {
        0.0
    } else {
        (right - t) / (right - target)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::cashflow::builder::specs::PrepaymentModelSpec;
    use crate::instruments::fixed_income::mbs_passthrough::{AgencyProgram, PoolType};
    use finstack_core::currency::Currency;
    use finstack_core::dates::DayCount;
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
            .interp(InterpStyle::Linear)
            .build()
            .expect("valid curve");

        MarketContext::new().insert_discount(disc)
    }

    #[test]
    fn test_key_rate_dv01() {
        let mbs = create_test_mbs();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        let result = key_rate_dv01(&mbs, &market, as_of).expect("key rate dv01");

        // Should have all standard tenors
        assert_eq!(result.dv01_by_tenor.len(), STANDARD_TENORS.len());

        // Each DV01 should be a reasonable value
        for (tenor, dv01) in &result.dv01_by_tenor {
            assert!(
                dv01.abs() < result.base_price * 0.01,
                "DV01 for {} seems too large: {}",
                tenor,
                dv01
            );
        }
    }

    #[test]
    fn test_key_rate_sum_approximates_total() {
        let mbs = create_test_mbs();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        let result = key_rate_dv01(&mbs, &market, as_of).expect("key rate dv01");

        // Total DV01 should be sum of key rates
        let sum: f64 = result.dv01_by_tenor.values().sum();
        assert!((result.total_dv01 - sum).abs() < 1e-10);
    }

    #[test]
    fn test_triangular_weight() {
        // At target, weight should be 1.0
        assert!((triangular_weight(5.0, 5.0) - 1.0).abs() < 1e-10);

        // At neighbors, weight should be 0.0 or 1.0
        assert!(triangular_weight(0.0, 5.0) < 0.5);
        assert!(triangular_weight(10.0, 5.0) < 0.5);

        // In between, weight should interpolate
        let w = triangular_weight(3.0, 5.0);
        assert!(w > 0.0 && w < 1.0);
    }
}
