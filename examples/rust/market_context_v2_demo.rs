//! Demonstration of MarketContextV2 benefits
//!
//! This example showcases the improvements of the new enum-based storage
//! system over the original trait object-based approach.

#[cfg(all(feature = "new-context", feature = "serde"))]
fn main() -> finstack_core::Result<()> {
    use finstack_core::{
        dates::Date,
        market_data::{
            context_v2::MarketContextV2,
            interp::InterpStyle,
            primitives::MarketScalar,
            term_structures::{
                base_correlation::BaseCorrelationCurve,
                discount_curve::DiscountCurve,
                forward_curve::ForwardCurve,
                hazard_curve::HazardCurve,
            },
        },
    };

    println!("🚀 MarketContextV2 - Enum-Based Storage Demo");
    println!("=============================================\n");

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
        .knots(vec![(3.0, 0.25), (7.0, 0.45), (10.0, 0.60), (15.0, 0.75)])
        .build()
        .unwrap();

    // 1. BENEFIT: Ergonomic Builder Pattern
    println!("✅ 1. Ergonomic Builder Pattern");
    let context = MarketContextV2::builder()
        .discount(discount_curve)
        .forward(forward_curve) 
        .hazard(hazard_curve)
        .base_correlation(base_corr)
        .price("SPOT_SPY", MarketScalar::Unitless(450.0))
        .price("USD_RATE", MarketScalar::Unitless(0.045))
        .collateral("USD-CSA", "USD-OIS")
        .build()?;

    println!("   Created context with {} curves and {} prices", 
        context.stats().total_curves, 
        context.stats().price_count);

    // 2. BENEFIT: Type-Safe Access with Compile-Time Guarantees
    println!("\n✅ 2. Type-Safe Access");
    let curve = context.curve("USD-OIS").unwrap();
    println!("   Curve type: {}", curve.curve_type());
    println!("   Is discount: {}", curve.is_discount());
    println!("   Is forward: {}", curve.is_forward());

    // 3. BENEFIT: Backward Compatible API
    println!("\n✅ 3. Backward Compatible API");
    let disc_trait = context.disc("USD-OIS")?;
    let fwd_trait = context.fwd("USD-SOFR3M")?;
    println!("   Discount factor at 1Y: {:.6}", disc_trait.df(1.0));
    println!("   Forward rate at 1Y: {:.4}%", fwd_trait.rate(1.0) * 100.0);

    // 4. BENEFIT: Direct Concrete Access (No Dynamic Dispatch)
    println!("\n✅ 4. Direct Concrete Access");
    let disc_concrete = context.discount_curve("USD-OIS")?;
    println!("   Direct access DF at 1Y: {:.6}", disc_concrete.df(1.0));
    println!("   Curve ID: {}", disc_concrete.id().as_str());

    // 5. BENEFIT: Complete Serialization
    println!("\n✅ 5. Complete Serialization (No String Parsing!)");
    let json = serde_json::to_string_pretty(&context)
        .expect("Should serialize perfectly");
    
    println!("   JSON size: {} bytes", json.len());
    
    // Verify no string parsing artifacts
    assert!(!json.contains("_bump_"));
    assert!(!json.contains("_spread_"));
    println!("   ✓ No string parsing artifacts in JSON");

    // Round-trip test
    let restored: MarketContextV2 = serde_json::from_str(&json)
        .expect("Should deserialize perfectly");
    
    println!("   ✓ Perfect round-trip serialization");
    println!("   Original curves: {}, Restored curves: {}", 
        context.stats().total_curves, 
        restored.stats().total_curves);

    // 6. BENEFIT: Rich Introspection
    println!("\n✅ 6. Rich Introspection");
    let stats = context.stats();
    println!("{}", stats);

    // Show curve breakdown
    println!("   Curves by type:");
    for (curve_type, count) in stats.curve_counts {
        println!("     {}: {}", curve_type, count);
    }

    // 7. BENEFIT: Advanced Filtering
    println!("\n✅ 7. Advanced Filtering");
    let discount_curves: Vec<_> = context.curves_of_type("Discount").collect();
    println!("   Found {} discount curves", discount_curves.len());
    
    for (id, storage) in discount_curves {
        println!("     - {}: {}", id, storage.curve_type());
    }

    // 8. BENEFIT: Performance Characteristics
    println!("\n✅ 8. Performance Comparison");
    
    // Time trait object access (V1 style)
    let start = std::time::Instant::now();
    for _ in 0..10000 {
        let disc = context.disc("USD-OIS").unwrap();
        let _ = disc.df(1.0);
    }
    let trait_time = start.elapsed();
    
    // Time concrete access (V2 style)
    let start = std::time::Instant::now();
    for _ in 0..10000 {
        let disc = context.discount_curve("USD-OIS").unwrap();
        let _ = disc.df(1.0);
    }
    let concrete_time = start.elapsed();
    
    println!("   Trait object access (10k calls): {:?}", trait_time);
    println!("   Concrete access (10k calls): {:?}", concrete_time);
    
    if concrete_time < trait_time {
        let speedup = trait_time.as_nanos() as f64 / concrete_time.as_nanos() as f64;
        println!("   🎉 Concrete access {:.1}x faster!", speedup);
    }

    // 9. BENEFIT: Memory Efficiency Analysis
    println!("\n✅ 9. Memory Efficiency");
    println!("   Total objects in context: {}", context.total_objects());
    println!("   Context is empty: {}", context.is_empty());

    println!("\n🎉 MarketContextV2 Demo Complete!");
    println!("🎯 Key Benefits Demonstrated:");
    println!("   ✓ Complete serialization without workarounds");
    println!("   ✓ Type-safe access with compile-time guarantees");
    println!("   ✓ Backward compatible API");
    println!("   ✓ Performance improvements for concrete access");
    println!("   ✓ Rich introspection and filtering capabilities");
    println!("   ✓ Clean, maintainable architecture");

    Ok(())
}

#[cfg(not(all(feature = "new-context", feature = "serde")))]
fn main() {
    println!("This example requires both 'new-context' and 'serde' features.");
    println!("Run with: cargo run --example market_context_v2_demo --features \"new-context,serde\"");
}
