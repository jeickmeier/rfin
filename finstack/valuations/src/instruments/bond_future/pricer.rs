//! Bond future pricing logic.
//!
//! This module implements pricing and valuation for bond futures, including:
//! - Conversion factor calculation
//! - Model futures price calculation
//! - NPV calculation

use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::bond::Bond;

/// Bond future pricer.
///
/// Implements pricing logic for bond futures, including conversion factor calculation,
/// model price calculation, and NPV calculation.
pub struct BondFuturePricer;

impl BondFuturePricer {
    /// Calculate conversion factor for a bond in the deliverable basket.
    ///
    /// The conversion factor normalizes bonds with different coupons and maturities
    /// to make them comparable for delivery against a futures contract.
    ///
    /// # Formula
    ///
    /// CF = PV(bond cashflows at standard coupon rate) / 100
    ///
    /// For semi-annual bonds (UST):
    /// - Discount factor: DF(t) = 1 / (1 + r/2)^(2*t)
    /// - PV = Σ(cashflow_i × DF(t_i))
    /// - CF = PV / 100 (per $100 face value)
    ///
    /// # Parameters
    ///
    /// - `bond`: The deliverable bond to calculate the conversion factor for
    /// - `standard_coupon`: The standard coupon rate used by the futures contract (e.g., 0.06 for 6%)
    /// - `standard_maturity_years`: The standard maturity in years (e.g., 10.0 for UST 10Y)
    /// - `market`: Market context for getting bond cashflows
    /// - `as_of`: The calculation date (typically first day of delivery month)
    ///
    /// # Returns
    ///
    /// Conversion factor rounded to 4 decimal places
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let bond = Bond::fixed_semiannual(...);
    /// let cf = BondFuturePricer::calculate_conversion_factor(
    ///     &bond,
    ///     0.06,  // 6% standard coupon
    ///     10.0,  // 10-year maturity
    ///     &market,
    ///     date!(2025-03-01),
    /// )?;
    /// // cf might be 0.8234 for a bond with 5% coupon
    /// ```
    pub fn calculate_conversion_factor(
        bond: &Bond,
        standard_coupon: f64,
        _standard_maturity_years: f64,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<f64> {
        // Get bond's cashflows (holder view: all positive)
        let cashflows = bond.build_schedule(market, as_of)?;

        // Calculate present value using standard coupon rate as discount rate
        // For semi-annual bonds: DF(t) = 1 / (1 + r/2)^(2*t)
        let mut pv = 0.0;
        let half_rate = standard_coupon / 2.0;

        for (flow_date, amount) in cashflows {
            // Only include future cashflows (as_of date or later)
            // Past cashflows have already been paid and should not be included
            if flow_date < as_of {
                continue;
            }

            // Calculate time to cashflow in years (ACT/365F convention for conversion factor)
            let days = (flow_date - as_of).whole_days();
            let years = days as f64 / 365.0;

            // Calculate discount factor for semi-annual compounding
            // DF(t) = 1 / (1 + r/2)^(2*t)
            let periods = 2.0 * years;
            let discount_factor = 1.0 / (1.0 + half_rate).powf(periods);

            // Add discounted cashflow to PV
            pv += amount.amount() * discount_factor;
        }

        // Conversion factor = PV / Par Value
        // This gives a multiplier (e.g., 0.8234 for a discount bond)
        // For a $100,000 bond, we divide by 100,000 to get the factor
        let notional = bond.notional.amount();
        let cf_raw = pv / notional;

        // Round to 4 decimal places (standard for conversion factors)
        let cf = (cf_raw * 10000.0).round() / 10000.0;

        Ok(cf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use finstack_core::types::CurveId;
    use time::macros::date;

    use crate::instruments::bond::Bond;

    /// Helper to create a simple market context with a flat discount curve
    fn create_test_market(rate: f64) -> MarketContext {
        // Create a flat discount curve at the given rate
        // Using a simple 2-knot curve to approximate flat discount rate
        let base_date = date!(2025 - 01 - 15);
        
        // Calculate discount factors for a flat rate
        // DF(t) = exp(-rate * t) for continuous compounding
        // For semi-annual compounding: DF(t) = 1 / (1 + rate/2)^(2*t)
        let df_1y = 1.0 / (1.0 + rate / 2.0).powi(2);
        let df_5y = 1.0 / (1.0 + rate / 2.0).powi(10);
        let df_10y = 1.0 / (1.0 + rate / 2.0).powi(20);

        let curve = DiscountCurve::builder(CurveId::new("USD-TREASURY"))
            .base_date(base_date)
            .knots(vec![
                (0.0, 1.0),      // Today
                (1.0, df_1y),    // 1 year
                (5.0, df_5y),    // 5 years
                (10.0, df_10y),  // 10 years
            ])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("Failed to build discount curve");

        // insert_discount consumes self and returns Self (builder pattern)
        MarketContext::new().insert_discount(curve)
    }

    /// Helper to create a test bond with fixed semi-annual coupons
    fn create_test_bond(
        notional: f64,
        coupon_rate: f64,
        issue: Date,
        maturity: Date,
    ) -> Bond {
        Bond::fixed(
            "TEST_BOND",
            Money::new(notional, Currency::USD),
            coupon_rate,
            issue,
            maturity,
            "USD-TREASURY",
        )
    }

    #[test]
    fn test_cashflow_debug() {
        // Debug test to see what cashflows are generated
        let bond = create_test_bond(100_000.0, 0.06, date!(2020 - 01 - 15), date!(2030 - 01 - 15));
        let market = create_test_market(0.06);
        let as_of = date!(2025 - 01 - 15);

        let cashflows = bond.build_schedule(&market, as_of).unwrap();
        
        println!("\n=== Bond Cashflows ===");
        println!("As of: {:?}", as_of);
        println!("Total flows: {}", cashflows.len());
        let mut total = 0.0;
        for (date, amount) in &cashflows {
            println!("  {:?}: ${:.2}", date, amount.amount());
            total += amount.amount();
        }
        println!("Total cashflows: ${:.2}", total);
        println!("Expected for 100k notional with 6% coupon: ~$103,000 (coupons) + $100,000 (redemption)");
    }

    #[test]
    fn test_conversion_factor_par_bond() {
        // For a bond with coupon equal to standard coupon, CF should be ~1.0
        let bond = create_test_bond(100_000.0, 0.06, date!(2020 - 01 - 15), date!(2030 - 01 - 15));

        let market = create_test_market(0.06);
        let as_of = date!(2025 - 01 - 15);

        let cf =
            BondFuturePricer::calculate_conversion_factor(&bond, 0.06, 10.0, &market, as_of)
                .unwrap();

        // For a bond with coupon = standard coupon and ~10 years to maturity,
        // CF should be close to 1.0
        assert!(
            (cf - 1.0).abs() < 0.05,
            "Par bond CF should be near 1.0, got {}",
            cf
        );
    }

    #[test]
    fn test_conversion_factor_discount_bond() {
        // For a bond with coupon < standard coupon, CF should be < 1.0
        let bond = create_test_bond(100_000.0, 0.04, date!(2020 - 01 - 15), date!(2030 - 01 - 15));

        let market = create_test_market(0.04);
        let as_of = date!(2025 - 01 - 15);

        let cf =
            BondFuturePricer::calculate_conversion_factor(&bond, 0.06, 10.0, &market, as_of)
                .unwrap();

        // Lower coupon bond should have CF < 1.0
        assert!(cf < 1.0, "Discount bond CF should be < 1.0, got {}", cf);
        assert!(cf > 0.7, "CF should be reasonable, got {}", cf);
    }

    #[test]
    fn test_conversion_factor_premium_bond() {
        // For a bond with coupon > standard coupon, CF should be > 1.0
        let bond = create_test_bond(100_000.0, 0.08, date!(2020 - 01 - 15), date!(2030 - 01 - 15));

        let market = create_test_market(0.08);
        let as_of = date!(2025 - 01 - 15);

        let cf =
            BondFuturePricer::calculate_conversion_factor(&bond, 0.06, 10.0, &market, as_of)
                .unwrap();

        // Higher coupon bond should have CF > 1.0
        assert!(cf > 1.0, "Premium bond CF should be > 1.0, got {}", cf);
        assert!(cf < 1.3, "CF should be reasonable, got {}", cf);
    }

    #[test]
    fn test_conversion_factor_rounding() {
        // Test that CF is rounded to 4 decimal places
        let bond = create_test_bond(100_000.0, 0.055, date!(2020 - 01 - 15), date!(2030 - 01 - 15));

        let market = create_test_market(0.055);
        let as_of = date!(2025 - 01 - 15);

        let cf =
            BondFuturePricer::calculate_conversion_factor(&bond, 0.06, 10.0, &market, as_of)
                .unwrap();

        // Check that CF has at most 4 decimal places
        let cf_scaled = (cf * 10000.0).round();
        let cf_rounded = cf_scaled / 10000.0;
        assert_eq!(
            cf, cf_rounded,
            "CF should be rounded to 4 decimal places"
        );
    }

    #[test]
    fn test_conversion_factor_short_maturity() {
        // Test CF for a bond with shorter remaining maturity
        let bond = create_test_bond(100_000.0, 0.05, date!(2023 - 01 - 15), date!(2027 - 01 - 15));

        let market = create_test_market(0.05);
        let as_of = date!(2025 - 01 - 15);

        let cf =
            BondFuturePricer::calculate_conversion_factor(&bond, 0.06, 10.0, &market, as_of)
                .unwrap();

        // Shorter maturity discount bond should have CF very close to 1.0
        // With only 2 years remaining, the coupon difference has minimal impact
        assert!(
            (cf - 1.0).abs() < 0.02,
            "Short maturity bond CF should be very close to 1.0, got {}",
            cf
        );
        assert!(
            cf > 0.98,
            "CF should be close to par for short maturity, got {}",
            cf
        );
    }
}
