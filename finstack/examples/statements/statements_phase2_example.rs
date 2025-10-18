//! Phase 2 Example: DSL Parser and Compiler
//!
//! This example demonstrates the DSL (Domain-Specific Language) engine
//! implemented in Phase 2, including:
//! - Formula parsing
//! - AST inspection
//! - Compilation to core Expr
//! - All supported operators and functions

use finstack_core::expr::{ExprNode, Function};
use finstack_statements::dsl::{compile, parse_and_compile, parse_formula, StmtExpr};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Finstack Statements - Phase 2 DSL Example\n");
    println!("═══════════════════════════════════════════════════════════\n");

    // Example 1: Basic Arithmetic
    example_1_arithmetic()?;

    // Example 2: AST Inspection
    example_2_ast_inspection()?;

    // Example 3: Compilation
    example_3_compilation()?;

    // Example 4: Time-Series Functions
    example_4_time_series()?;

    // Example 5: Rolling Window Functions
    example_5_rolling_windows()?;

    // Example 6: Statistical Functions
    example_6_statistical()?;

    // Example 7: Conditional Expressions
    example_7_conditionals()?;

    // Example 8: Complex Expressions
    example_8_complex()?;

    // Example 9: Error Handling
    example_9_error_handling()?;

    println!("\n═══════════════════════════════════════════════════════════");
    println!("✅ Phase 2 DSL features demonstrated successfully!");
    println!("═══════════════════════════════════════════════════════════\n");

    println!("Next Steps:");
    println!("  • Phase 3: Evaluator to execute formulas");
    println!("  • Phase 4: Forecast methods");
    println!("  • Phase 5: Dynamic metric registry");
    println!("  • Phase 6: Capital structure integration\n");

    Ok(())
}

fn example_1_arithmetic() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Example 1: Basic Arithmetic Operations");
    println!("───────────────────────────────────────────────────────────\n");

    let formulas = vec![
        ("revenue + other_income", "Addition"),
        ("revenue - cogs", "Subtraction"),
        ("revenue * 0.6", "Multiplication"),
        ("gross_profit / revenue", "Division"),
        ("period_num % 4", "Modulo"),
    ];

    for (formula, description) in formulas {
        let ast = parse_formula(formula)?;
        println!("  {}: {}", description, formula);
        match ast {
            StmtExpr::BinOp { op, .. } => {
                println!("    → Operator: {:?}", op);
            }
            _ => println!("    → Parsed successfully"),
        }
    }

    println!();
    Ok(())
}

fn example_2_ast_inspection() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Example 2: AST Structure Inspection");
    println!("───────────────────────────────────────────────────────────\n");

    let formula = "(revenue - cogs) / revenue";
    println!("Formula: {}\n", formula);

    let ast = parse_formula(formula)?;

    println!("AST Structure:");
    print_ast(&ast, 0);

    println!();
    Ok(())
}

fn print_ast(expr: &StmtExpr, indent: usize) {
    let prefix = "  ".repeat(indent);
    match expr {
        StmtExpr::Literal(val) => {
            println!("{}Literal({})", prefix, val);
        }
        StmtExpr::NodeRef(name) => {
            println!("{}NodeRef('{}')", prefix, name);
        }
        StmtExpr::BinOp { op, left, right } => {
            println!("{}BinOp({:?})", prefix, op);
            print_ast(left, indent + 1);
            print_ast(right, indent + 1);
        }
        StmtExpr::UnaryOp { op, operand } => {
            println!("{}UnaryOp({:?})", prefix, op);
            print_ast(operand, indent + 1);
        }
        StmtExpr::Call { func, args } => {
            println!("{}Call('{}')", prefix, func);
            for arg in args {
                print_ast(arg, indent + 1);
            }
        }
        StmtExpr::IfThenElse {
            condition,
            then_expr,
            else_expr,
        } => {
            println!("{}IfThenElse", prefix);
            println!("{}  condition:", prefix);
            print_ast(condition, indent + 2);
            println!("{}  then:", prefix);
            print_ast(then_expr, indent + 2);
            println!("{}  else:", prefix);
            print_ast(else_expr, indent + 2);
        }
        StmtExpr::CSRef {
            component,
            instrument_or_total,
        } => {
            println!(
                "{}CSRef('{}', '{}')",
                prefix, component, instrument_or_total
            );
        }
    }
}

fn example_3_compilation() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚙️  Example 3: Compilation to Core Expr");
    println!("───────────────────────────────────────────────────────────\n");

    let formula = "revenue * 1.05";
    println!("Formula: {}", formula);

    let ast = parse_formula(formula)?;
    let expr = compile(&ast)?;

    println!("  Parsed to AST: {:?}", ast);
    println!("  Compiled to core Expr: {:?}", expr.node);

    // Or use the convenience function
    let expr2 = parse_and_compile(formula)?;
    println!("  Using parse_and_compile(): {:?}", expr2.node);

    println!();
    Ok(())
}

fn example_4_time_series() -> Result<(), Box<dyn std::error::Error>> {
    println!("📈 Example 4: Time-Series Functions");
    println!("───────────────────────────────────────────────────────────\n");

    let formulas = vec![
        ("lag(revenue, 1)", "Previous period value"),
        ("diff(revenue, 1)", "First difference"),
        ("pct_change(revenue, 1)", "Period-over-period growth"),
        (
            "pct_change(revenue, 4)",
            "Year-over-year growth (quarterly)",
        ),
    ];

    for (formula, description) in formulas {
        let expr = parse_and_compile(formula)?;
        println!("  {}", description);
        println!("    Formula: {}", formula);

        // Check if it compiled to a core Function
        if let ExprNode::Call(func, _) = expr.node {
            println!("    → Compiled to core function: {:?}", func);
        } else {
            println!("    → Compiled successfully");
        }
    }

    println!();
    Ok(())
}

fn example_5_rolling_windows() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Example 5: Rolling Window Functions");
    println!("───────────────────────────────────────────────────────────\n");

    let formulas = vec![
        ("rolling_mean(revenue, 4)", "4-period moving average"),
        (
            "rolling_sum(revenue, 4)",
            "4-period rolling sum (TTM for quarterly)",
        ),
        (
            "rolling_sum(revenue, 12)",
            "12-period rolling sum (TTM for monthly)",
        ),
        ("rolling_std(revenue, 4)", "4-period rolling std dev"),
        ("rolling_min(revenue, 4)", "4-period rolling minimum"),
        ("rolling_max(revenue, 4)", "4-period rolling maximum"),
    ];

    for (formula, description) in formulas {
        let expr = parse_and_compile(formula)?;
        println!("  {}", description);
        println!("    Formula: {}", formula);

        if let ExprNode::Call(func, _) = expr.node {
            println!("    → Core function: {:?}", func);
        }
    }

    println!("\n  💡 Tip: TTM (Trailing Twelve Months)");
    println!("     - Quarterly data: rolling_sum(revenue, 4)");
    println!("     - Monthly data:   rolling_sum(revenue, 12)");

    println!();
    Ok(())
}

fn example_6_statistical() -> Result<(), Box<dyn std::error::Error>> {
    println!("📐 Example 6: Statistical Functions");
    println!("───────────────────────────────────────────────────────────\n");

    let formulas = vec![
        ("std(revenue)", "Standard deviation"),
        ("var(revenue)", "Variance"),
        ("median(revenue)", "Median value"),
        ("mean(revenue)", "Mean/average (custom function)"),
    ];

    for (formula, description) in formulas {
        let expr = parse_and_compile(formula)?;
        println!("  {}: {}", description, formula);

        if let ExprNode::Call(func, args) = &expr.node {
            match func {
                Function::Std | Function::Var | Function::Median => {
                    println!("    → Core function: {:?} with {} arg(s)", func, args.len());
                }
                _ => {
                    println!("    → Custom function (synthetic call)");
                }
            }
        }
    }

    println!();
    Ok(())
}

fn example_7_conditionals() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔀 Example 7: Conditional Expressions");
    println!("───────────────────────────────────────────────────────────\n");

    // Simple if-then-else
    let formula1 = "if(revenue > 1000000, revenue * 0.1, 0)";
    println!("Conditional bonus:");
    println!("  Formula: {}", formula1);
    let ast1 = parse_formula(formula1)?;
    if let StmtExpr::IfThenElse { .. } = ast1 {
        println!("    → Parsed as IfThenElse expression");
    }

    // Comparison operators
    println!("\nComparison Operators:");
    let comparisons = vec![
        ("revenue == 1000000", "Equal"),
        ("revenue != 0", "Not equal"),
        ("revenue > 1000000", "Greater than"),
        ("revenue >= 1000000", "Greater than or equal"),
        ("margin < 0.1", "Less than"),
        ("margin <= 0.1", "Less than or equal"),
    ];

    for (formula, op) in comparisons {
        let ast = parse_formula(formula)?;
        if let StmtExpr::BinOp { op: bin_op, .. } = ast {
            println!("  {}: {} → {:?}", op, formula, bin_op);
        }
    }

    // Logical operators
    println!("\nLogical Operators:");
    let formula2 = "revenue > 1000000 and margin > 0.15";
    println!("  Formula: {}", formula2);
    let ast2 = parse_formula(formula2)?;
    if let StmtExpr::BinOp { op, .. } = ast2 {
        println!("    → Operator: {:?}", op);
    }

    println!();
    Ok(())
}

fn example_8_complex() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎭 Example 8: Complex Nested Expressions");
    println!("───────────────────────────────────────────────────────────\n");

    let examples = vec![
        ("(revenue - cogs) / revenue", "Gross margin calculation"),
        (
            "rolling_mean(pct_change(revenue, 1), 4)",
            "4-period average of MoM growth",
        ),
        (
            "debt_balance / rolling_sum(ebitda, 4)",
            "Leverage ratio (Debt/TTM EBITDA)",
        ),
        (
            "if(revenue > 1000000, rolling_mean(revenue, 4), lag(revenue, 1))",
            "Conditional with nested functions",
        ),
        ("(revenue - cogs - opex) / revenue", "Operating margin"),
    ];

    for (formula, description) in examples {
        println!("  {}", description);
        println!("    Formula: {}", formula);

        let expr = parse_and_compile(formula)?;
        println!("    ✓ Parsed and compiled successfully");

        // Show some details about the compiled expression
        match &expr.node {
            ExprNode::Call(_func, args) => {
                println!("    → Top-level call with {} argument(s)", args.len());
            }
            ExprNode::Literal(_) => {
                println!("    → Literal value");
            }
            ExprNode::Column(name) => {
                println!("    → Column reference: {}", name);
            }
            ExprNode::BinOp { .. } => {
                println!("    → Binary operation");
            }
            ExprNode::UnaryOp { .. } => {
                println!("    → Unary operation");
            }
            ExprNode::IfThenElse { .. } => {
                println!("    → Conditional expression");
            }
        }
        println!();
    }

    Ok(())
}

fn example_9_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚠️  Example 9: Error Handling");
    println!("───────────────────────────────────────────────────────────\n");

    let invalid_formulas = vec![
        ("revenue +", "Incomplete expression"),
        ("revenue @@ cogs", "Invalid operator"),
        ("(revenue - cogs", "Unmatched parenthesis"),
        ("", "Empty formula"),
    ];

    for (formula, description) in invalid_formulas {
        println!("  {}: \"{}\"", description, formula);
        match parse_formula(formula) {
            Ok(_) => {
                println!("    ✗ Unexpectedly succeeded");
            }
            Err(e) => {
                println!("    ✓ Error caught: {}", e);
            }
        }
    }

    println!("\n  💡 The parser provides clear error messages to help debug formulas.");

    println!();
    Ok(())
}
