//! Property-based tests for curve monotonicity and no-arbitrage.
//!
//! Key Properties:
//! - Discount factors: DF(t2) ≤ DF(t1) for t2 > t1 (strictly decreasing)
//! - Forward rates: f(t1, t2) ≥ minimum floor (e.g., -50bp)
//! - Zero rates: z(t2) can be ≥ or ≤ z(t1) (depends on curve shape)

use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::DiscountCurve;
use proptest::prelude::*;
use time::Month;

/// Generate valid (monotonically decreasing) discount factors
fn valid_discount_factors() -> impl Strategy<Value = Vec<(f64, f64)>> {
    // Generate 3-7 knot points with decreasing DFs
    (3usize..=7).prop_flat_map(|n| {
        // Generate times: 0.0, then increasing
        let times: Vec<f64> = (0..n).map(|i| i as f64 + (i as f64) * 0.5).collect();

        // Generate strictly decreasing DFs starting from 1.0
        // Use narrower range (0.92..0.98) to avoid extreme forward rates
        let dfs_strategy = prop::collection::vec(0.92..0.98, n - 1).prop_map(move |rates| {
            let mut dfs = vec![1.0]; // Start at DF=1.0
            for rate in rates {
                let prev = *dfs.last().unwrap();
                dfs.push(prev * rate); // Each DF is 92-98% of previous (2-8% decline per period)
            }
            dfs
        });

        (Just(times), dfs_strategy).prop_map(|(t, df)| t.into_iter().zip(df).collect())
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    #[ignore = "Property test: 50 iterations"]
    fn prop_discount_factors_decrease(
        knots in valid_discount_factors(),
    ) {
        let base_date = Date::from_calendar_date(2025, Month::January, 15).unwrap();

        // Build curve - should succeed with valid monotonic data
        let curve = DiscountCurve::builder("PROP-TEST")
            .base_date(base_date)
            .knots(knots.clone())
            .build();

        prop_assert!(curve.is_ok(), "Valid monotonic curve should build: {:?}", curve.err());

        let curve = curve.unwrap();

        // Property: DF(t2) < DF(t1) for all t2 > t1
        for i in 0..knots.len() - 1 {
            let (t1, _df1) = knots[i];
            let (t2, _df2) = knots[i + 1];

            let evaluated_df1 = curve.df(t1);
            let evaluated_df2 = curve.df(t2);

            prop_assert!(
                evaluated_df2 < evaluated_df1,
                "DF({}) = {:.6} should be < DF({}) = {:.6}",
                t2, evaluated_df2, t1, evaluated_df1
            );
        }
    }

    #[test]
    #[ignore = "Property test: 50 iterations"]
    fn prop_invalid_curves_rejected(
        valid_knots in valid_discount_factors(),
        bad_index in 1usize..5,
    ) {
        let base_date = Date::from_calendar_date(2025, Month::January, 15).unwrap();

        // Create invalid curve by making one DF increase
        let mut invalid_knots = valid_knots.clone();
        if bad_index < invalid_knots.len() - 1 {
            // Make DF at bad_index+1 larger than DF at bad_index
            let (_, df_prev) = invalid_knots[bad_index - 1];
            invalid_knots[bad_index].1 = df_prev * 1.01; // Increase instead of decrease

            // Build curve - should fail validation
            let result = DiscountCurve::builder("INVALID-PROP")
                .base_date(base_date)
                .knots(invalid_knots)
                .build();

            prop_assert!(
                result.is_err(),
                "Non-monotonic curve should be rejected"
            );
        }
    }

    #[test]
    #[ignore = "Property test: 50 iterations"]
    fn prop_zero_rates_positive_for_normal_curves(
        knots in valid_discount_factors(),
    ) {
        let base_date = Date::from_calendar_date(2025, Month::January, 15).unwrap();

        let curve = DiscountCurve::builder("ZERO-PROP")
            .base_date(base_date)
            .knots(knots.clone())
            .build()
            .unwrap();

        // Property: For normal (decreasing DF) curves, zero rates are typically positive
        for (t, _df) in &knots {
            if *t > 0.0 {
                let zero = curve.zero(*t);

                // Zero rates should be in reasonable range for normal curves
                prop_assert!(
                    zero > -0.10 && zero < 0.25,
                    "Zero rate {:.4} at t={} outside reasonable range",
                    zero, t
                );
            }
        }
    }
}
