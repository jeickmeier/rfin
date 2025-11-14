//! Generate example JSON files for all supported instruments.
//!
//! This test generates canonical JSON examples for each instrument type using
//! the `example()` constructors defined on each instrument.
//!
//! Run with: cargo test --package finstack-valuations --test json_examples_generator -- --ignored --nocapture
//! Output files go to: finstack/valuations/tests/instruments/json_examples/
//!
//! Status: Currently generates examples for all instruments that have example() methods.
//! To add more instruments, add example() methods to their type files.

use finstack_valuations::instruments::*;
use std::fs;
use std::path::Path;

fn ensure_output_dir() -> std::io::Result<()> {
    let dir = Path::new("tests/instruments/json_examples");
    fs::create_dir_all(dir)
}

fn write_example(filename: &str, instrument: json_loader::InstrumentJson) -> std::io::Result<()> {
    let envelope = json_loader::InstrumentEnvelope {
        schema: "finstack.instrument/1".to_string(),
        instrument,
    };

    let json = serde_json::to_string_pretty(&envelope).unwrap();
    let path = format!("tests/instruments/json_examples/{}.json", filename);
    fs::write(&path, json)?;
    println!("✓ Generated: {}", path);
    Ok(())
}

#[test]
#[ignore] // Run explicitly with --ignored flag
fn generate_all_json_examples() {
    ensure_output_dir().unwrap();

    println!("\n📝 Generating JSON examples for all instruments...\n");

    let mut count = 0;

    // Fixed Income
    write_example("bond", json_loader::InstrumentJson::Bond(Bond::example())).unwrap();
    count += 1;

    // Credit
    write_example(
        "credit_default_swap",
        json_loader::InstrumentJson::CreditDefaultSwap(CreditDefaultSwap::example()),
    )
    .unwrap();
    count += 1;

    // Equity
    write_example(
        "equity",
        json_loader::InstrumentJson::Equity(Equity::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "equity_option",
        json_loader::InstrumentJson::EquityOption(EquityOption::example()),
    )
    .unwrap();
    count += 1;

    // FX
    write_example(
        "fx_swap",
        json_loader::InstrumentJson::FxSwap(FxSwap::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "fx_option",
        json_loader::InstrumentJson::FxOption(FxOption::example()),
    )
    .unwrap();
    count += 1;

    // Rates
    write_example(
        "interest_rate_swap",
        json_loader::InstrumentJson::InterestRateSwap(InterestRateSwap::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "forward_rate_agreement",
        json_loader::InstrumentJson::ForwardRateAgreement(ForwardRateAgreement::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "swaption",
        json_loader::InstrumentJson::Swaption(Swaption::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "interest_rate_future",
        json_loader::InstrumentJson::InterestRateFuture(InterestRateFuture::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "inflation_swap",
        json_loader::InstrumentJson::InflationSwap(InflationSwap::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "cds_option",
        json_loader::InstrumentJson::CdsOption(CdsOption::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "asian_option",
        json_loader::InstrumentJson::AsianOption(AsianOption::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "barrier_option",
        json_loader::InstrumentJson::BarrierOption(BarrierOption::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "cms_option",
        json_loader::InstrumentJson::CmsOption(CmsOption::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "lookback_option",
        json_loader::InstrumentJson::LookbackOption(LookbackOption::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "variance_swap",
        json_loader::InstrumentJson::VarianceSwap(VarianceSwap::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "fx_barrier_option",
        json_loader::InstrumentJson::FxBarrierOption(FxBarrierOption::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "quanto_option",
        json_loader::InstrumentJson::QuantoOption(QuantoOption::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "autocallable",
        json_loader::InstrumentJson::Autocallable(Autocallable::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "cliquet_option",
        json_loader::InstrumentJson::CliquetOption(CliquetOption::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "range_accrual",
        json_loader::InstrumentJson::RangeAccrual(RangeAccrual::example()),
    )
    .unwrap();
    count += 1;

    // TRS
    write_example(
        "trs_equity",
        json_loader::InstrumentJson::TrsEquity(EquityTotalReturnSwap::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "trs_fixed_income_index",
        json_loader::InstrumentJson::TrsFixedIncomeIndex(FIIndexTotalReturnSwap::example()),
    )
    .unwrap();
    count += 1;

    // Other
    write_example(
        "deposit",
        json_loader::InstrumentJson::Deposit(Deposit::example()),
    )
    .unwrap();
    count += 1;
    write_example("repo", json_loader::InstrumentJson::Repo(Repo::example())).unwrap();
    count += 1;
    write_example(
        "convertible_bond",
        json_loader::InstrumentJson::ConvertibleBond(ConvertibleBond::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "inflation_linked_bond",
        json_loader::InstrumentJson::InflationLinkedBond(InflationLinkedBond::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "cds_index",
        json_loader::InstrumentJson::CDSIndex(CDSIndex::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "term_loan",
        json_loader::InstrumentJson::TermLoan(TermLoan::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "revolving_credit",
        json_loader::InstrumentJson::RevolvingCredit(RevolvingCredit::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "cds_tranche",
        json_loader::InstrumentJson::CdsTranche(CdsTranche::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "basket",
        json_loader::InstrumentJson::Basket(Basket::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "basket_with_instruments",
        json_loader::InstrumentJson::Basket(Basket::example_with_instruments()),
    )
    .unwrap();
    count += 1;
    write_example(
        "private_markets_fund",
        json_loader::InstrumentJson::PrivateMarketsFund(PrivateMarketsFund::example()),
    )
    .unwrap();
    count += 1;
    write_example(
        "structured_credit",
        json_loader::InstrumentJson::StructuredCredit(Box::new(StructuredCredit::example())),
    )
    .unwrap();
    count += 1;

    println!("\n✅ Successfully generated {} example JSON files!", count);
    println!("📁 Location: tests/instruments/json_examples/");
    println!("\nThese files demonstrate the complete JSON contract for each instrument type.");
    println!("They can be used for:");
    println!("  - Documentation and examples");
    println!("  - Integration testing");
    println!("  - LLM structured output templates");
    println!("  - JSON schema validation");
    println!("\n💡 To add more examples:");
    println!("  1. Add `pub fn example() -> Self` to the instrument type");
    println!("  2. Add a write_example() call here");
    println!("  3. Re-run this generator");
}
