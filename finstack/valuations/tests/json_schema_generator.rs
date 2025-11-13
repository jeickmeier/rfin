//! Generate JSON Schema definitions for instruments.
//!
//! Note: Full schemars-based schema generation requires JsonSchema derives
//! on finstack-core types (InstrumentId, Money, Currency, Date, etc.).
//! This generator creates simplified schemas based on the actual JSON examples.
//!
//! Run with: cargo test --package finstack-valuations --test json_schema_generator -- --ignored --nocapture
//! Output: finstack/valuations/schemas/instrument/1/*.schema.json

use std::fs;
use std::path::Path;

fn ensure_schema_dir() -> std::io::Result<()> {
    let dir = Path::new("schemas/instrument/1");
    fs::create_dir_all(dir)
}

/// Generate a schema from an example JSON file
fn generate_schema_from_example(example_file: &str, schema_file: &str, title: &str, description: &str) -> std::io::Result<()> {
    // Read the example JSON
    let example_path = format!("tests/instruments/json_examples/{}.json", example_file);
    let example_json = fs::read_to_string(&example_path)?;
    
    // Parse to infer structure
    let example: serde_json::Value = serde_json::from_str(&example_json)?;
    
    // Create a basic schema with the structure
    let schema = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "$id": format!("https://finstack.dev/schemas/instrument/1/{}.schema.json", schema_file),
        "title": title,
        "description": description,
        "type": "object",
        "required": ["schema", "instrument"],
        "properties": {
            "schema": {
                "type": "string",
                "const": "finstack.instrument/1",
                "description": "Schema version identifier"
            },
            "instrument": {
                "type": "object",
                "description": format!("The {} instrument definition", title),
                "example": example.get("instrument")
            }
        },
        "additionalProperties": false,
        "examples": [example]
    });
    
    let schema_path = format!("schemas/instrument/1/{}.schema.json", schema_file);
    fs::write(&schema_path, serde_json::to_string_pretty(&schema)?)?;
    println!("✓ Generated schema: {}", schema_path);
    Ok(())
}

#[test]
#[ignore]
fn generate_all_schemas() {
    ensure_schema_dir().unwrap();
    
    println!("\n📋 Generating JSON Schemas from examples...\n");

    generate_schema_from_example(
        "bond",
        "bond",
        "Bond",
        "Fixed income bond instrument with fixed, floating, or amortizing cashflows"
    ).unwrap();

    generate_schema_from_example(
        "credit_default_swap",
        "credit_default_swap",
        "Credit Default Swap",
        "CDS contract providing credit protection on a reference entity"
    ).unwrap();

    generate_schema_from_example(
        "equity",
        "equity",
        "Equity",
        "Spot equity position in a listed stock"
    ).unwrap();

    generate_schema_from_example(
        "equity_option",
        "equity_option",
        "Equity Option",
        "Vanilla equity option with European or American exercise"
    ).unwrap();

    generate_schema_from_example(
        "fx_swap",
        "fx_swap",
        "FX Swap",
        "Foreign exchange swap with near and far legs"
    ).unwrap();

    generate_schema_from_example(
        "trs_equity",
        "trs_equity",
        "Equity Total Return Swap",
        "Total return swap on an equity index or single stock"
    ).unwrap();

    generate_schema_from_example(
        "deposit",
        "deposit",
        "Deposit",
        "Simple single-period deposit instrument"
    ).unwrap();

    generate_schema_from_example(
        "fx_option",
        "fx_option",
        "FX Option",
        "Vanilla foreign exchange option (Garman-Kohlhagen model)"
    ).unwrap();

    generate_schema_from_example(
        "interest_rate_swap",
        "interest_rate_swap",
        "Interest Rate Swap",
        "Fixed-for-floating interest rate swap"
    ).unwrap();

    generate_schema_from_example(
        "forward_rate_agreement",
        "forward_rate_agreement",
        "Forward Rate Agreement",
        "Forward contract on an interest rate (FRA)"
    ).unwrap();

    generate_schema_from_example(
        "swaption",
        "swaption",
        "Swaption",
        "Option on an interest rate swap"
    ).unwrap();

    generate_schema_from_example(
        "interest_rate_future",
        "interest_rate_future",
        "Interest Rate Future",
        "Exchange-traded interest rate future"
    ).unwrap();

    generate_schema_from_example(
        "inflation_swap",
        "inflation_swap",
        "Inflation Swap",
        "Zero-coupon inflation swap"
    ).unwrap();

    generate_schema_from_example(
        "cds_option",
        "cds_option",
        "CDS Option",
        "Option on a CDS spread"
    ).unwrap();

    generate_schema_from_example(
        "asian_option",
        "asian_option",
        "Asian Option",
        "Option with payoff based on average underlying price"
    ).unwrap();

    generate_schema_from_example(
        "barrier_option",
        "barrier_option",
        "Barrier Option",
        "Option with knock-in/knock-out barrier"
    ).unwrap();

    generate_schema_from_example(
        "cms_option",
        "cms_option",
        "CMS Option",
        "Caplet/Floorlet on a constant-maturity swap (CMS) rate"
    ).unwrap();

    generate_schema_from_example(
        "lookback_option",
        "lookback_option",
        "Lookback Option",
        "Option with payoff based on the max/min of the underlying over a period"
    ).unwrap();

    generate_schema_from_example(
        "variance_swap",
        "variance_swap",
        "Variance Swap",
        "Forward contract on realized variance"
    ).unwrap();

    generate_schema_from_example(
        "fx_barrier_option",
        "fx_barrier_option",
        "FX Barrier Option",
        "FX option with barrier feature"
    ).unwrap();

    generate_schema_from_example(
        "quanto_option",
        "quanto_option",
        "Quanto Option",
        "Equity option with payout in a different currency (quanto adjustment)"
    ).unwrap();
    generate_schema_from_example(
        "term_loan",
        "term_loan",
        "Term Loan",
        "Term loan with fixed or floating rate and amortization"
    ).unwrap();
    generate_schema_from_example(
        "revolving_credit",
        "revolving_credit",
        "Revolving Credit Facility",
        "Credit facility with draws/repayments and usage/commitment fees"
    ).unwrap();
    generate_schema_from_example(
        "cds_tranche",
        "cds_tranche",
        "CDS Tranche",
        "Synthetic CDO tranche on a CDS index"
    ).unwrap();
    generate_schema_from_example(
        "basket",
        "basket",
        "Basket",
        "Basket of assets with expense ratio"
    ).unwrap();
    generate_schema_from_example(
        "private_markets_fund",
        "private_markets_fund",
        "Private Markets Fund",
        "Fund with waterfall distributions and events"
    ).unwrap();
    generate_schema_from_example(
        "trs_fixed_income_index",
        "trs_fixed_income_index",
        "Fixed Income Index Total Return Swap",
        "TRS on a fixed income index with financing leg"
    ).unwrap();
    generate_schema_from_example(
        "structured_credit",
        "structured_credit",
        "Structured Credit",
        "ABS/CLO/CMBS/RMBS structure with waterfall"
    ).unwrap();
    generate_schema_from_example(
        "autocallable",
        "autocallable",
        "Autocallable",
        "Autocallable structured product with observation schedule"
    ).unwrap();
    generate_schema_from_example(
        "cliquet_option",
        "cliquet_option",
        "Cliquet Option",
        "Option with periodic reset caps and global cap"
    ).unwrap();
    generate_schema_from_example(
        "range_accrual",
        "range_accrual",
        "Range Accrual",
        "Coupon accrues when underlying stays within a range"
    ).unwrap();
    generate_schema_from_example(
        "repo",
        "repo",
        "Repurchase Agreement (Repo)",
        "Short-term collateralized borrowing/lending"
    ).unwrap();

    generate_schema_from_example(
        "convertible_bond",
        "convertible_bond",
        "Convertible Bond",
        "Bond with embedded equity conversion option"
    ).unwrap();

    generate_schema_from_example(
        "inflation_linked_bond",
        "inflation_linked_bond",
        "Inflation-Linked Bond",
        "Bond with coupons and principal indexed to inflation"
    ).unwrap();

    generate_schema_from_example(
        "cds_index",
        "cds_index",
        "CDS Index",
        "Credit default swap index (e.g., CDX, iTraxx)"
    ).unwrap();

    // Create a union schema referencing all instruments
    let union_schema = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "$id": "https://finstack.dev/schemas/instrument/1/instrument.schema.json",
        "title": "Finstack Instrument",
        "description": "Tagged union of all supported financial instruments",
        "type": "object",
        "required": ["schema", "instrument"],
        "properties": {
            "schema": {
                "type": "string",
                "const": "finstack.instrument/1"
            },
            "instrument": {
                "type": "object",
                "required": ["type", "spec"],
                "properties": {
                    "type": {
                        "type": "string",
                        "enum": [
                            "bond", "credit_default_swap", "equity", "equity_option",
                            "fx_swap", "trs_equity", "deposit", "fx_option",
                            "interest_rate_swap", "forward_rate_agreement",
                            "swaption", "interest_rate_future", "inflation_swap",
                            "cds_option", "asian_option", "barrier_option", "cms_option",
                            "lookback_option", "variance_swap", "fx_barrier_option", "quanto_option", "term_loan",
                            "revolving_credit", "cds_tranche", "basket", "private_markets_fund",
                            "trs_fixed_income_index", "structured_credit", "autocallable",
                            "cliquet_option", "range_accrual",
                            "repo", "convertible_bond", "inflation_linked_bond",
                            "cds_index"
                        ]
                    },
                    "spec": {
                        "type": "object",
                        "description": "Instrument-specific fields (see individual schemas)"
                    }
                }
            }
        }
    });

    fs::write(
        "schemas/instrument/1/instrument.schema.json",
        serde_json::to_string_pretty(&union_schema).unwrap()
    ).unwrap();
    println!("✓ Generated schema: schemas/instrument/1/instrument.schema.json");

    println!("\n✅ Generated {} JSON Schema files!", 31);
    println!("📁 Location: schemas/instrument/1/");
    println!("\nThese schemas can be used for:");
    println!("  - IDE autocomplete and validation");
    println!("  - LLM structured outputs (OpenAI, Anthropic, etc.)");
    println!("  - API documentation generation");
    println!("  - Contract validation in CI/CD");
}

