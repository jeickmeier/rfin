#!/usr/bin/env rust-script
//! Example demonstrating the new Polars-based InflationIndex API
//!
//! ```cargo
//! [dependencies]
//! finstack-core = { path = "../finstack/core" }
//! time = "0.3"
//! polars = "0.49"
//! ```

use finstack_core::dates::{InflationIndex, InflationIndexBuilder, InflationInterpolation, InflationLag};
use finstack_core::{Currency, Date};
use time::Month;

fn main() -> finstack_core::Result<()> {
    println!("=== Polars-based InflationIndex Demo ===\n");

    // Helper function for dates
    let make_date = |year, month: u8, day| {
        Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
    };

    // 1. Create a US CPI index with historical observations
    println!("1. Creating US CPI index with monthly observations:");
    let observations = vec![
        (make_date(2023, 1, 31), 299.170),
        (make_date(2023, 2, 28), 300.840),
        (make_date(2023, 3, 31), 301.836),
        (make_date(2023, 4, 30), 302.918),
        (make_date(2023, 5, 31), 303.294),
        (make_date(2023, 6, 30), 303.841),
        (make_date(2023, 7, 31), 304.348),
        (make_date(2023, 8, 31), 305.537),
        (make_date(2023, 9, 30), 306.269),
        (make_date(2023, 10, 31), 306.139),
        (make_date(2023, 11, 30), 306.060),
        (make_date(2023, 12, 31), 306.746),
    ];

    let cpi = InflationIndex::new("US-CPI-U", observations.clone(), Currency::USD)?;
    println!("  Created {} with {} observations", cpi.id, observations.len());
    
    let (start, end) = cpi.date_range()?;
    println!("  Date range: {} to {}", start, end);

    // 2. Test different interpolation methods
    println!("\n2. Comparing interpolation methods for mid-month value:");
    let test_date = make_date(2023, 6, 15);
    
    let step_value = cpi.value_on(test_date)?;
    println!("  Step (default): {:.3}", step_value);
    
    let linear_cpi = InflationIndex::new("US-CPI-U", observations.clone(), Currency::USD)?
        .with_interpolation(InflationInterpolation::Linear);
    let linear_value = linear_cpi.value_on(test_date)?;
    println!("  Linear: {:.3}", linear_value);

    // 3. Calculate inflation rates
    println!("\n3. Calculating inflation rates:");
    let q1_start = make_date(2023, 1, 1);
    let q1_end = make_date(2023, 3, 31);
    let q1_ratio = cpi.ratio(q1_start, q1_end)?;
    println!("  Q1 2023: {:.2}%", (q1_ratio - 1.0) * 100.0);

    let h1_start = make_date(2023, 1, 1);
    let h1_end = make_date(2023, 6, 30);
    let h1_ratio = cpi.ratio(h1_start, h1_end)?;
    println!("  H1 2023: {:.2}%", (h1_ratio - 1.0) * 100.0);

    let ytd_start = make_date(2023, 1, 1);
    let ytd_end = make_date(2023, 12, 31);
    let ytd_ratio = cpi.ratio(ytd_start, ytd_end)?;
    println!("  Full year 2023: {:.2}%", (ytd_ratio - 1.0) * 100.0);

    // 4. Demonstrate lag policies (important for inflation-linked bonds)
    println!("\n4. Testing lag policies for TIPS/ILB calculations:");
    let settlement = make_date(2023, 10, 15);
    
    let no_lag = cpi.value_on(settlement)?;
    println!("  No lag: {:.3}", no_lag);
    
    let cpi_3m_lag = InflationIndex::new("US-CPI-U", observations.clone(), Currency::USD)?
        .with_lag(InflationLag::Months(3));
    let lag_3m = cpi_3m_lag.value_on(settlement)?;
    println!("  3-month lag: {:.3} (typical for US TIPS)", lag_3m);

    let cpi_2m_lag = InflationIndex::new("US-CPI-U", observations.clone(), Currency::USD)?
        .with_lag(InflationLag::Months(2));
    let lag_2m = cpi_2m_lag.value_on(settlement)?;
    println!("  2-month lag: {:.3} (typical for UK ILBs)", lag_2m);

    // 5. Using the builder pattern
    println!("\n5. Using the builder pattern for UK RPI:");
    let uk_rpi = InflationIndexBuilder::new("UK-RPI", Currency::GBP)
        .add_observation(make_date(2023, 1, 31), 348.7)
        .add_observation(make_date(2023, 2, 28), 351.2)
        .add_observation(make_date(2023, 3, 31), 352.6)
        .add_observation(make_date(2023, 4, 30), 354.0)
        .with_interpolation(InflationInterpolation::Linear)
        .with_lag(InflationLag::Months(2))
        .build()?;

    println!("  Created {}", uk_rpi.id);
    let uk_value = uk_rpi.value_on(make_date(2023, 6, 15))?;
    println!("  Value on 2023-06-15 with 2-month lag: {:.1}", uk_value);

    // 6. Access underlying Polars DataFrame for advanced operations
    println!("\n6. Leveraging Polars DataFrame capabilities:");
    let df = cpi.as_dataframe();
    println!("  DataFrame shape: {:?}", df.shape());
    println!("  Column names: {:?}", df.get_column_names());
    
    // You can now use all Polars operations on this DataFrame
    // e.g., df.filter(), df.group_by(), df.join(), etc.
    
    println!("\n✅ All tests completed successfully!");
    Ok(())
}
