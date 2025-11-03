//! Example demonstrating name normalization with aliases.
//!
//! Run with: cargo run --example name_normalization_example

use finstack_statements::builder::ModelBuilder;
use finstack_statements::registry::AliasRegistry;
use finstack_statements::Result;
use indexmap::IndexSet;

fn main() -> Result<()> {
    println!("=== Name Normalization Example ===\n");

    // Create alias registry
    let mut registry = AliasRegistry::new();

    // Add custom aliases
    registry.add_alias("rev", "revenue");
    registry.add_alias("sales", "revenue");
    registry.add_alias("cos", "cogs");

    println!("1. Exact Alias Matching\n");
    println!("  'rev' normalizes to: {:?}", registry.normalize("rev"));
    println!(
        "  'sales' normalizes to: {:?}",
        registry.normalize("sales")
    );
    println!("  'cos' normalizes to: {:?}", registry.normalize("cos"));

    // Fuzzy matching
    println!("\n2. Fuzzy Matching\n");
    let available: IndexSet<String> = ["revenue", "cogs", "gross_profit"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    println!(
        "  'revenu' fuzzy matches to: {:?}",
        registry.normalize_fuzzy("revenu", &available)
    );
    println!(
        "  'Cogs' fuzzy matches to: {:?}",
        registry.normalize_fuzzy("Cogs", &available)
    );

    // Load standard aliases
    println!("\n3. Standard Accounting Aliases\n");
    let mut std_registry = AliasRegistry::new();
    std_registry.load_standard_aliases();

    println!("Standard aliases loaded:");
    println!("  'rev' → {:?}", std_registry.normalize("rev"));
    println!("  'ni' → {:?}", std_registry.normalize("ni"));
    println!("  'fcf' → {:?}", std_registry.normalize("fcf"));
    println!("  'opex' → {:?}", std_registry.normalize("opex"));

    // Use in model builder
    println!("\n4. Integration with ModelBuilder\n");
    let _model = ModelBuilder::new("demo")
        .periods("2025Q1..Q2", None)?
        .with_name_normalization()
        .compute("revenue", "100000")?
        .compute("cogs", "revenue * 0.4")?
        .build()?;

    println!("Model built successfully with name normalization enabled");

    Ok(())
}

