//! Market compliance golden tests for valuation parity.
//!
//! These tests validate pricing against committed reference values and use
//! standard USD market conventions where the fixture does not specify them.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::irs::{InterestRateSwap, IrsLegConventions, PayReceive};
use finstack_valuations::instruments::InstrumentJson;
use serde::Deserialize;
use std::collections::HashMap;
use time::Month;

const DEFAULT_FWD_TENOR_YEARS: f64 = 0.25;
const RATES_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/integration/golden/data/market_compliance/rates.json"
));
const CREDIT_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/integration/golden/data/market_compliance/credit.json"
));
const FX_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/integration/golden/data/market_compliance/fx.json"
));
const EQUITY_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/integration/golden/data/market_compliance/equity.json"
));

#[derive(Debug, Deserialize)]
struct GoldenRoot {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    test_cases: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct CurveInput {
    id: String,
    reference_date: String,
    points: Vec<[f64; 2]>,
    day_count: String,
}

#[derive(Debug, Deserialize)]
struct BondInput {
    id: String,
    notional: f64,
    currency: Currency,
    coupon_rate: f64,
    maturity: String,
    issue_date: String,
}

#[derive(Debug, Deserialize)]
struct BondInputs {
    bond: BondInput,
    discount_curve: CurveInput,
}

#[derive(Debug, Deserialize)]
struct SwapInput {
    id: String,
    notional: f64,
    fixed_rate: f64,
    start_date: String,
    maturity_date: String,
    pay_receive: PayReceive,
}

#[derive(Debug, Deserialize)]
struct SwapInputs {
    swap: SwapInput,
    discount_curve: CurveInput,
    forward_curve: CurveInput,
}

#[derive(Debug, Deserialize)]
struct ExpectedPv {
    present_value: f64,
    tolerance: f64,
    currency: Option<Currency>,
}

#[derive(Debug, Deserialize)]
struct BondCase {
    valuation_date: Option<String>,
    inputs: BondInputs,
    expected: ExpectedPv,
}

#[derive(Debug, Deserialize)]
struct SwapCase {
    valuation_date: Option<String>,
    inputs: SwapInputs,
    expected: ExpectedPv,
}

#[derive(Deserialize)]
struct GenericParityCase {
    valuation_date: String,
    instrument: InstrumentJson,
    market_context: MarketContext,
    expected: ExpectedPv,
}

fn parse_date(value: &str) -> Date {
    let mut iter = value.split('-');
    let year: i32 = iter
        .next()
        .unwrap_or_else(|| panic!("missing year in date '{value}'"))
        .parse()
        .unwrap_or_else(|e| panic!("invalid year in date '{value}': {e}"));
    let month: u8 = iter
        .next()
        .unwrap_or_else(|| panic!("missing month in date '{value}'"))
        .parse()
        .unwrap_or_else(|e| panic!("invalid month in date '{value}': {e}"));
    let day: u8 = iter
        .next()
        .unwrap_or_else(|| panic!("missing day in date '{value}'"))
        .parse()
        .unwrap_or_else(|e| panic!("invalid day in date '{value}': {e}"));
    Date::from_calendar_date(
        year,
        Month::try_from(month)
            .unwrap_or_else(|_| panic!("invalid month {month} in date '{value}'")),
        day,
    )
    .unwrap_or_else(|e| panic!("invalid date '{value}': {e}"))
}

fn build_discount_curve(input: &CurveInput) -> DiscountCurve {
    let base = parse_date(&input.reference_date);
    DiscountCurve::builder(input.id.as_str())
        .base_date(base)
        .day_count(parse_day_count(&input.day_count))
        .knots(input.points.iter().map(|p| (p[0], p[1])))
        .build()
        .expect("discount curve should build")
}

fn build_forward_curve(input: &CurveInput) -> ForwardCurve {
    let base = parse_date(&input.reference_date);
    ForwardCurve::builder(input.id.as_str(), DEFAULT_FWD_TENOR_YEARS)
        .base_date(base)
        .day_count(parse_day_count(&input.day_count))
        .knots(input.points.iter().map(|p| (p[0], p[1])))
        .build()
        .expect("forward curve should build")
}

fn parse_day_count(value: &str) -> DayCount {
    let normalized = value.trim().to_ascii_lowercase().replace('-', "_");
    match normalized.as_str() {
        "act_360" | "act360" => DayCount::Act360,
        "act_365f" | "act365f" | "act_365_fixed" => DayCount::Act365F,
        "act_365l" | "act365l" => DayCount::Act365L,
        "thirty360" | "30_360" | "30_360_us" => DayCount::Thirty360,
        "thirty_e360" | "30e_360" => DayCount::ThirtyE360,
        "act_act" | "actact" => DayCount::ActAct,
        "act_act_isma" | "actact_isma" => DayCount::ActActIsma,
        "bus252" | "bus_252" => DayCount::Bus252,
        other => panic!("unsupported day count '{other}'"),
    }
}

fn usd_term_swap_conventions() -> IrsLegConventions {
    IrsLegConventions {
        fixed_freq: Tenor::semi_annual(),
        float_freq: Tenor::quarterly(),
        fixed_dc: DayCount::Thirty360,
        float_dc: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        payment_calendar_id: Some("usny".to_string()),
        fixing_calendar_id: Some("usny".to_string()),
        stub: StubKind::None,
        reset_lag_days: 2,
        payment_delay_days: 0,
    }
}

fn load_fixture(data: &str, label: &str) -> GoldenRoot {
    serde_json::from_str(data).unwrap_or_else(|err| {
        panic!("{} fixture should parse: {}", label, err);
    })
}

fn load_rates_root() -> GoldenRoot {
    load_fixture(RATES_FIXTURE, "rates")
}

fn load_credit_root() -> GoldenRoot {
    load_fixture(CREDIT_FIXTURE, "credit")
}

fn load_fx_root() -> GoldenRoot {
    load_fixture(FX_FIXTURE, "fx")
}

fn load_equity_root() -> GoldenRoot {
    load_fixture(EQUITY_FIXTURE, "equity")
}

fn fixture_ready(root: &GoldenRoot, label: &str) -> bool {
    let status = root
        .status
        .as_deref()
        .unwrap_or("unknown")
        .to_ascii_lowercase();
    if status == "certified" {
        true
    } else {
        eprintln!("Skipping {label} parity fixtures (status={status})");
        false
    }
}

fn assert_within_tolerance(actual: f64, expected: f64, tolerance: f64, label: &str) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "{label} outside tolerance: actual={actual}, expected={expected}, tol={tolerance}, diff={diff}"
    );
}

fn assert_expected_pv(pv: Money, expected: &ExpectedPv, label: &str) {
    if let Some(currency) = expected.currency {
        assert_eq!(
            pv.currency(),
            currency,
            "{label} currency mismatch: actual={}, expected={}",
            pv.currency(),
            currency
        );
    }
    assert_within_tolerance(
        pv.amount(),
        expected.present_value,
        expected.tolerance,
        label,
    );
}

fn run_generic_parity_cases(root: GoldenRoot, label: &str) {
    assert!(
        !root.test_cases.is_empty(),
        "{} fixture has no test cases",
        label
    );

    for (name, value) in root.test_cases {
        let case: GenericParityCase = serde_json::from_value(value).unwrap_or_else(|err| {
            panic!("{} fixture case '{}' failed to parse: {}", label, name, err);
        });
        let instrument = case.instrument.into_boxed().unwrap_or_else(|err| {
            panic!(
                "{} fixture case '{}' failed to build instrument: {}",
                label, name, err
            );
        });
        let as_of = parse_date(&case.valuation_date);
        let pv = instrument
            .value(&case.market_context, as_of)
            .unwrap_or_else(|err| {
                panic!("{} fixture case '{}' failed to price: {}", label, name, err);
            });
        assert_expected_pv(pv, &case.expected, &format!("{label}/{name} PV"));
    }
}

#[test]
fn test_market_compliance_fixture_smoke() {
    let root = load_rates_root();

    let bond_case = root
        .test_cases
        .get("bond_pricing_treasury")
        .expect("bond_pricing_treasury case missing");
    let bond_case: BondCase = serde_json::from_value(bond_case.clone()).expect("bond case parse");
    let _ = build_discount_curve(&bond_case.inputs.discount_curve);
    let _ = parse_date(&bond_case.inputs.bond.issue_date);
    let _ = parse_date(&bond_case.inputs.bond.maturity);
    if let Some(valuation_date) = bond_case.valuation_date.as_deref() {
        let _ = parse_date(valuation_date);
    }

    let swap_case = root
        .test_cases
        .get("irs_valuation")
        .expect("irs_valuation case missing");
    let swap_case: SwapCase = serde_json::from_value(swap_case.clone()).expect("swap case parse");
    let _ = build_discount_curve(&swap_case.inputs.discount_curve);
    let _ = build_forward_curve(&swap_case.inputs.forward_curve);
    let _ = parse_date(&swap_case.inputs.swap.start_date);
    let _ = parse_date(&swap_case.inputs.swap.maturity_date);
    if let Some(valuation_date) = swap_case.valuation_date.as_deref() {
        let _ = parse_date(valuation_date);
    }

    let _ = load_credit_root();
    let _ = load_fx_root();
    let _ = load_equity_root();
}

#[test]
fn test_bond_golden_parity() {
    let root = load_rates_root();
    if !fixture_ready(&root, "rates") {
        return;
    }
    let case = root
        .test_cases
        .get("bond_pricing_treasury")
        .expect("bond_pricing_treasury case missing");
    let case: BondCase = serde_json::from_value(case.clone()).expect("bond case parse");

    let bond_input = case.inputs.bond;
    let issue = parse_date(&bond_input.issue_date);
    let maturity = parse_date(&bond_input.maturity);
    let notional = Money::new(bond_input.notional, bond_input.currency);

    let bond = Bond::fixed(
        bond_input.id,
        notional,
        bond_input.coupon_rate,
        issue,
        maturity,
        case.inputs.discount_curve.id.as_str(),
    )
    .expect("bond construction should succeed");

    let discount_curve = build_discount_curve(&case.inputs.discount_curve);
    let as_of = case
        .valuation_date
        .as_deref()
        .map(parse_date)
        .unwrap_or(issue);
    let market = MarketContext::new().insert_discount(discount_curve);

    let pv = bond
        .value(&market, as_of)
        .expect("bond pricing should succeed");
    assert_eq!(pv.currency(), bond_input.currency);
    assert_expected_pv(pv, &case.expected, "bond PV");
}

#[test]
fn test_irs_golden_parity() {
    let root = load_rates_root();
    if !fixture_ready(&root, "rates") {
        return;
    }
    let case = root
        .test_cases
        .get("irs_valuation")
        .expect("irs_valuation case missing");
    let case: SwapCase = serde_json::from_value(case.clone()).expect("swap case parse");

    let swap_input = case.inputs.swap;
    let start = parse_date(&swap_input.start_date);
    let end = parse_date(&swap_input.maturity_date);
    let notional = Money::new(swap_input.notional, Currency::USD);

    let swap = InterestRateSwap::create_term_swap_with_conventions(
        InstrumentId::new(swap_input.id),
        notional,
        swap_input.fixed_rate,
        start,
        end,
        swap_input.pay_receive,
        CurveId::new(case.inputs.discount_curve.id.as_str()),
        CurveId::new(case.inputs.forward_curve.id.as_str()),
        usd_term_swap_conventions(),
    )
    .expect("swap construction should succeed");

    let discount_curve = build_discount_curve(&case.inputs.discount_curve);
    let forward_curve = build_forward_curve(&case.inputs.forward_curve);
    let as_of = case
        .valuation_date
        .as_deref()
        .map(parse_date)
        .unwrap_or_else(|| parse_date(&case.inputs.discount_curve.reference_date));

    let market = MarketContext::new()
        .insert_discount(discount_curve)
        .insert_forward(forward_curve);

    let pv = swap
        .value(&market, as_of)
        .expect("swap pricing should succeed");
    assert_expected_pv(pv, &case.expected, "swap PV");
}

#[test]
fn test_credit_golden_parity() {
    let root = load_credit_root();
    if !fixture_ready(&root, "credit") {
        return;
    }
    run_generic_parity_cases(root, "credit");
}

#[test]
fn test_fx_golden_parity() {
    let root = load_fx_root();
    if !fixture_ready(&root, "fx") {
        return;
    }
    run_generic_parity_cases(root, "fx");
}

#[test]
fn test_equity_golden_parity() {
    let root = load_equity_root();
    if !fixture_ready(&root, "equity") {
        return;
    }
    run_generic_parity_cases(root, "equity");
}
