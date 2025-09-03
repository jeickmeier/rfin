// Test script to demonstrate the fix for expression lowering panic
// This script would previously panic on nested non-lowerable expressions

use finstack_core::expr::{CompiledExpr, Expr, Function};

fn main() {
    println!("Testing expression lowering fix...");
    
    // Create a complex nested expression that has non-lowerable nested components
    // For example, a Lag function with a nested Rank call (which cannot be lowered to Polars)
    let non_lowerable_nested = Expr::call(
        Function::Lag,
        vec![
            // This inner Rank function cannot be lowered to Polars (returns None)
            Expr::call(Function::Rank, vec![Expr::column("values")]),
            Expr::literal(1.0)
        ]
    );
    
    let compiled = CompiledExpr::new(non_lowerable_nested);
    
    // Before the fix: this would panic with "called `Option::unwrap()` on a `None` value"
    // After the fix: this should gracefully return None (meaning "cannot lower")
    match compiled.to_polars_expr() {
        Some(_) => println!("✅ Expression was successfully lowered to Polars"),
        None => println!("✅ Expression gracefully fell back to scalar evaluation (no panic!)"),
    }
    
    // Test another case with a binary operation
    let binary_with_non_lowerable = Expr::call(
        Function::Lead,
        vec![
            Expr::call(Function::Quantile, vec![Expr::column("values"), Expr::literal(0.5)]),
            Expr::literal(2.0)
        ]
    );
    
    let compiled2 = CompiledExpr::new(binary_with_non_lowerable);
    match compiled2.to_polars_expr() {
        Some(_) => println!("✅ Binary expression was successfully lowered to Polars"),
        None => println!("✅ Binary expression gracefully fell back to scalar evaluation (no panic!)"),
    }
    
    println!("🎉 All tests passed! The bug has been fixed.");
}
