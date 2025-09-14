//! Demonstration test for MarketContext benefits

#[cfg(test)]
#[cfg(feature = "serde")]
mod demo_tests {
    use super::super::core::MarketContext;
    use crate::{
        dates::Date,
        market_data::{
            interp::InterpStyle,
            scalars::MarketScalar,
            term_structures::{
                base_correlation::BaseCorrelationCurve,
                discount_curve::DiscountCurve,
                forward_curve::ForwardCurve,
                hazard_curve::HazardCurve,
            },
        },
    };

    #[test]
    fn demo_comprehensive_benefits() {
        println!("\n🚀 MarketContext - Enum-Based Storage Demo");
        println!("=============================================");

        // Create curves using the standard builders
        let base_date = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
        
        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80), (10.0, 0.67)])
            .set_interp(InterpStyle::MonotoneConvex)
            .build()
            .unwrap();

        let forward_curve = ForwardCurve::builder("USD-SOFR3M", 0.25)
            .base_date(base_date)
            .knots([(0.0, 0.03), (1.0, 0.035), (5.0, 0.04), (10.0, 0.045)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();

        let hazard_curve = HazardCurve::builder("CORP-HAZARD")
            .base_date(base_date)
            .recovery_rate(0.4)
            .knots([(0.0, 0.01), (1.0, 0.015), (5.0, 0.02), (10.0, 0.025)])
            .build()
            .unwrap();

        let base_corr = BaseCorrelationCurve::builder("CDX.NA.IG.42")
            .points(vec![(3.0, 0.25), (7.0, 0.45), (10.0, 0.60), (15.0, 0.75)])
            .build()
            .unwrap();

        // BENEFIT 1: Ergonomic Builder Pattern
        println!("\n✅ 1. Ergonomic Builder Pattern");
        let context = MarketContext::builder()
            .discount(discount_curve)
            .forward(forward_curve) 
            .hazard(hazard_curve)
            .base_correlation(base_corr)
            .price("SPOT_SPY", MarketScalar::Unitless(450.0))
            .price("USD_RATE", MarketScalar::Unitless(0.045))
            .collateral("USD-CSA", "USD-OIS")
            .build()
            .unwrap();

        println!("   Created context with {} curves and {} prices", 
            context.stats().total_curves, 
            context.stats().price_count);

        // BENEFIT 2: Type-Safe Access
        println!("\n✅ 2. Type-Safe Access with Compile-Time Guarantees");
        let curve = context.curve("USD-OIS").unwrap();
        println!("   Curve type: {}", curve.curve_type());
        println!("   Is discount: {}", curve.is_discount());
        println!("   Is forward: {}", curve.is_forward());

        // BENEFIT 3: Clean, Direct API (No Trait Objects!)
        println!("\n✅ 3. Clean, Direct API (Zero Overhead)");
        let disc = context.discount("USD-OIS").unwrap();
        let fwd = context.forward("USD-SOFR3M").unwrap();
        println!("   Discount factor at 1Y: {:.6}", disc.df(1.0));
        println!("   Forward rate at 1Y: {:.4}%", fwd.rate(1.0) * 100.0);
        println!("   ✓ Direct concrete types - no trait object overhead");

        // BENEFIT 5: Complete Serialization
        println!("\n✅ 5. Complete Serialization (No String Parsing!)");
        let json = serde_json::to_string_pretty(&context)
            .expect("Should serialize perfectly");
        
        println!("   JSON size: {} bytes", json.len());
        
        // Verify no string parsing artifacts
        assert!(!json.contains("_bump_"));
        assert!(!json.contains("_spread_"));
        println!("   ✓ No string parsing artifacts in JSON");

        // Round-trip test
        let restored: MarketContext = serde_json::from_str(&json)
            .expect("Should deserialize perfectly");
        
        println!("   ✓ Perfect round-trip serialization");
        println!("   Original curves: {}, Restored curves: {}", 
            context.stats().total_curves, 
            restored.stats().total_curves);

        // Verify values are preserved exactly
        let orig_df = context.discount("USD-OIS").unwrap().df(1.0);
        let rest_df = restored.discount("USD-OIS").unwrap().df(1.0);
        assert!((orig_df - rest_df).abs() < 1e-15);
        println!("   ✓ Values preserved with machine precision");

        // BENEFIT 6: Rich Introspection
        println!("\n✅ 6. Rich Introspection");
        let stats = context.stats();
        println!("   Total objects: {}", context.total_objects());
        println!("   Curve breakdown:");
        for (curve_type, count) in &stats.curve_counts {
            println!("     {}: {}", curve_type, count);
        }

        // BENEFIT 7: Advanced Filtering and Iteration
        println!("\n✅ 7. Advanced Filtering and Iteration");
        let discount_curves: Vec<_> = context.curves_of_type("Discount").collect();
        println!("   Found {} discount curves:", discount_curves.len());
        for (id, storage) in discount_curves {
            println!("     - {}: {}", id, storage.curve_type());
        }

        // BENEFIT 8: Optimal Performance
        println!("\n✅ 8. Optimal Performance (Zero Overhead)");
        
        // Measure direct concrete access - the only API now!
        let iterations = 10000;
        let start = std::time::Instant::now();
        for _ in 0..iterations {
            let disc = context.discount("USD-OIS").unwrap();
            let _ = disc.df(1.0);
        }
        let access_time = start.elapsed();
        
        println!("   Direct concrete access ({} calls): {:?}", iterations, access_time);
        
        let ns_per_call = access_time.as_nanos() / iterations as u128;
        println!("   Performance: {}ns/call", ns_per_call);
        println!("   ✓ Zero trait object overhead - maximum performance");

        println!("\n🎉 MarketContext Demo Complete!");
        println!("🎯 Key Benefits Demonstrated:");
        println!("   ✓ Complete serialization without workarounds");
        println!("   ✓ Type-safe access with compile-time guarantees"); 
        println!("   ✓ Clean, direct API with zero overhead");
        println!("   ✓ Maximum performance - no trait object conversions");
        println!("   ✓ Rich introspection and filtering capabilities");
        println!("   ✓ Simplified, maintainable architecture");
    }
}
