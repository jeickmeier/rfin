//! Example demonstrating interest rate compounding convention conversions.
//!
//! This example shows how to use the rate conversion utilities to convert
//! between different compounding conventions commonly used in financial markets.

use finstack_core::dates::rate_conversions::*;

fn main() -> Result<(), finstack_core::Error> {
    println!("=== Interest Rate Compounding Convention Conversions ===\n");

    // Example 1: US Treasury rates (semi-annual to continuous for zero curve construction)
    println!("Example 1: US Treasury Rate Conversion");
    println!("----------------------------------------");
    let treasury_rate = 0.025; // 2.5% semi-annual
    let continuous = periodic_to_continuous(treasury_rate, 2)?;
    println!("Treasury rate (semi-annual): {:.4}%", treasury_rate * 100.0);
    println!("Continuous equivalent:       {:.4}%", continuous * 100.0);
    println!("Usage: Zero curve bootstrapping\n");

    // Example 2: Money market rates (simple to periodic for swap pricing)
    println!("Example 2: LIBOR/SOFR Rate Conversion");
    println!("--------------------------------------");
    let libor_3m = 0.035; // 3.5% simple rate
    let year_fraction = 0.25; // 3 months
    let swap_rate = simple_to_periodic(libor_3m, year_fraction, 2)?;
    println!("LIBOR 3M (simple):          {:.4}%", libor_3m * 100.0);
    println!("Swap rate (semi-annual):    {:.4}%", swap_rate * 100.0);
    println!("Usage: Interest rate swap pricing\n");

    // Example 3: Corporate bond rates (annual to continuous for option pricing)
    println!("Example 3: Corporate Bond Rate Conversion");
    println!("------------------------------------------");
    let corporate_annual = 0.05; // 5% annual
    let continuous_corp = periodic_to_continuous(corporate_annual, 1)?;
    println!("Corporate bond (annual):    {:.4}%", corporate_annual * 100.0);
    println!("Continuous equivalent:      {:.4}%", continuous_corp * 100.0);
    println!("Usage: Black-Scholes option pricing\n");

    // Example 4: Round-trip verification
    println!("Example 4: Round-Trip Conversion Accuracy");
    println!("------------------------------------------");
    let original_rate = 0.04; // 4%
    let step1 = periodic_to_continuous(original_rate, 2)?;
    let step2 = continuous_to_periodic(step1, 4)?; // Convert to quarterly
    let step3 = periodic_to_continuous(step2, 4)?;
    let final_rate = continuous_to_periodic(step3, 2)?; // Back to semi-annual
    
    println!("Original rate (semi-annual): {:.10}%", original_rate * 100.0);
    println!("After round-trip:            {:.10}%", final_rate * 100.0);
    println!("Precision preserved:         {:.2e}", (original_rate - final_rate).abs());
    println!();

    // Example 5: Multi-frequency comparison
    println!("Example 5: Equivalent Rates Across Frequencies");
    println!("-----------------------------------------------");
    let base_annual = 0.06; // 6% annual
    println!("Base rate (annual):          {:.4}%", base_annual * 100.0);
    
    let continuous_eq = periodic_to_continuous(base_annual, 1)?;
    println!("Continuous equivalent:       {:.4}%", continuous_eq * 100.0);
    
    let semi_annual = continuous_to_periodic(continuous_eq, 2)?;
    println!("Semi-annual equivalent:      {:.4}%", semi_annual * 100.0);
    
    let quarterly = continuous_to_periodic(continuous_eq, 4)?;
    println!("Quarterly equivalent:        {:.4}%", quarterly * 100.0);
    
    let monthly = continuous_to_periodic(continuous_eq, 12)?;
    println!("Monthly equivalent:          {:.4}%", monthly * 100.0);
    println!();

    // Example 6: Negative rates (modern market scenario)
    println!("Example 6: Negative Interest Rates");
    println!("-----------------------------------");
    let negative_rate = -0.005; // -0.5% (European markets)
    let neg_continuous = periodic_to_continuous(negative_rate, 2)?;
    println!("Negative rate (semi-annual): {:.4}%", negative_rate * 100.0);
    println!("Continuous equivalent:       {:.4}%", neg_continuous * 100.0);
    println!("Note: Common in European markets\n");

    // Example 7: Simple vs. Periodic comparison
    println!("Example 7: Simple vs. Periodic Rates");
    println!("-------------------------------------");
    let simple = 0.05; // 5% simple
    let period = 1.0; // 1 year
    let periodic = simple_to_periodic(simple, period, 2)?;
    let back_to_simple = periodic_to_simple(periodic, period, 2)?;
    
    println!("Simple rate (1 year):        {:.4}%", simple * 100.0);
    println!("Periodic equivalent:         {:.4}%", periodic * 100.0);
    println!("Back to simple:              {:.4}%", back_to_simple * 100.0);
    println!("Difference for 1 year:       {:.2} bps", (periodic - simple) * 10000.0);
    println!();

    println!("=== All conversions completed successfully ===");
    Ok(())
}

