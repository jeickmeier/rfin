use finstack_wasm::{
    Bond, BusinessDayConvention, CreditDefaultSwap, CreditDefaultSwapBuilder, Currency, DayCount,
    DayCountContext, DayCountContextState, Deposit, DepositBuilder, Equity, EquityBuilder,
    EquityOption, EquityOptionBuilder, Frequency, FsDate, FxOption, FxOptionBuilder, FxSpot,
    FxSpotBuilder, InterestRateSwap, InterestRateSwapBuilder, Money, ScheduleSpec, StubKind,
    Swaption, SwaptionBuilder, Tenor,
};
use js_sys::JSON;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn daycount_context_state_roundtrip() {
    let mut ctx = DayCountContext::new();
    ctx.set_calendar_code("target2");
    ctx.set_frequency(&Frequency::quarterly());
    ctx.set_bus_basis(260);

    let state = ctx.to_state();
    let json = state.to_json_string().unwrap();
    let restored = DayCountContextState::from_json(&json).unwrap();
    let restored_ctx = restored.to_context();

    let start = FsDate::new(2025, 1, 2).unwrap();
    let end = FsDate::new(2025, 1, 12).unwrap();

    let day_count = DayCount::bus_252();
    let fraction = day_count
        .year_fraction(&start, &end, Some(restored_ctx))
        .unwrap();
    assert!(fraction > 0.0);
}

#[wasm_bindgen_test]
fn schedule_spec_json_roundtrip() {
    let start = FsDate::new(2025, 1, 15).unwrap();
    let end = FsDate::new(2025, 4, 15).unwrap();

    let spec = ScheduleSpec::new(
        &start,
        &end,
        &Tenor::monthly(),
        Some(StubKind::none()),
        Some(BusinessDayConvention::Following),
        Some("target2".to_string()),
        false,
        false,
        false,
    );

    let json = spec.to_json_string().unwrap();
    let restored = ScheduleSpec::from_json(&json).unwrap();
    let schedule = restored.build().unwrap();
    assert_eq!(schedule.length(), 4);
}

fn js_stringify(value: &wasm_bindgen::JsValue) -> String {
    JSON::stringify(value)
        .unwrap()
        .as_string()
        .unwrap_or_default()
}

#[wasm_bindgen_test]
fn instrument_tojson_fromjson_roundtrips_smoke() {
    let usd = Currency::new("USD").unwrap();
    let eur = Currency::new("EUR").unwrap();

    let issue = FsDate::new(2024, 1, 2).unwrap();
    let as_of = FsDate::new(2024, 1, 2).unwrap();
    let maturity = FsDate::new(2029, 1, 2).unwrap();

    // ---- Bond
    let notional = Money::from_code(1_000_000.0, "USD").unwrap();
    let bond = Bond::new(
        "bond_1",
        &notional,
        &issue,
        &maturity,
        "USD-OIS",
        Some(0.05),
        Some(Frequency::semi_annual()),
        Some(DayCount::thirty_360()),
        Some(BusinessDayConvention::ModifiedFollowing),
        Some("usny".to_string()),
        Some(StubKind::none()),
        None,
        None,
        None,
        Some(99.25),
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    let bond_json = bond.to_json().unwrap();
    let bond2 = Bond::from_json(bond_json.clone()).unwrap();
    assert_eq!(
        js_stringify(&bond_json),
        js_stringify(&bond2.to_json().unwrap())
    );

    // ---- Deposit
    let dep = DepositBuilder::new("dep_1")
        .money(&notional)
        .start(&issue)
        .maturity(&FsDate::new(2024, 7, 2).unwrap())
        .day_count(&DayCount::act_360())
        .discount_curve("USD-OIS")
        .quote_rate(0.05)
        .build()
        .unwrap();
    let dep_json = dep.to_json().unwrap();
    let dep2 = Deposit::from_json(dep_json.clone()).unwrap();
    assert_eq!(
        js_stringify(&dep_json),
        js_stringify(&dep2.to_json().unwrap())
    );

    // ---- Equity
    let eq = EquityBuilder::new("eq_1")
        .ticker("AAPL".to_string())
        .currency(&usd)
        .shares(100.0)
        .price(200.0)
        .build()
        .unwrap();
    let eq_json = eq.to_json().unwrap();
    let eq2 = Equity::from_json(eq_json.clone()).unwrap();
    assert_eq!(
        js_stringify(&eq_json),
        js_stringify(&eq2.to_json().unwrap())
    );

    // ---- EquityOption
    let eq_opt = EquityOptionBuilder::new("eqopt_1")
        .ticker("AAPL".to_string())
        .strike(200.0)
        .option_type("call".to_string())
        .expiry(&FsDate::new(2025, 1, 2).unwrap())
        .notional_amount(1.0)
        .build()
        .unwrap();
    let eq_opt_json = eq_opt.to_json().unwrap();
    let eq_opt2 = EquityOption::from_json(eq_opt_json.clone()).unwrap();
    assert_eq!(
        js_stringify(&eq_opt_json),
        js_stringify(&eq_opt2.to_json().unwrap())
    );

    // ---- FX spot / option / swap
    let fx_spot = FxSpotBuilder::new("fxspot_1")
        .base_currency(&eur)
        .quote_currency(&usd)
        .spot_rate(1.10)
        .build()
        .unwrap();
    let fx_spot_json = fx_spot.to_json().unwrap();
    let fx_spot2 = FxSpot::from_json(fx_spot_json.clone()).unwrap();
    assert_eq!(
        js_stringify(&fx_spot_json),
        js_stringify(&fx_spot2.to_json().unwrap())
    );

    let fx_opt = FxOptionBuilder::new("fxopt_1")
        .base_currency(&eur)
        .quote_currency(&usd)
        .strike(1.10)
        .option_type("call".to_string())
        .expiry(&FsDate::new(2024, 7, 2).unwrap())
        .money(&notional)
        .domestic_curve("USD-OIS")
        .foreign_curve("EUR-OIS")
        .vol_surface("EURUSD-VOL")
        .build()
        .unwrap();
    let fx_opt_json = fx_opt.to_json().unwrap();
    let fx_opt2 = FxOption::from_json(fx_opt_json.clone()).unwrap();
    assert_eq!(
        js_stringify(&fx_opt_json),
        js_stringify(&fx_opt2.to_json().unwrap())
    );

    // ---- CDS
    let cds = CreditDefaultSwapBuilder::new("cds_1")
        .money(&notional)
        .spread_bp(100.0)
        .start_date(&as_of)
        .maturity(&maturity)
        .discount_curve("USD-OIS")
        .credit_curve("ACME-HAZARD")
        .side("buy_protection".to_string())
        .recovery_rate(0.4)
        .build()
        .unwrap();
    let cds_json = cds.to_json().unwrap();
    let cds2 = CreditDefaultSwap::from_json(cds_json.clone()).unwrap();
    assert_eq!(
        js_stringify(&cds_json),
        js_stringify(&cds2.to_json().unwrap())
    );

    // ---- IRS (minimal fixed/float)
    let irs = InterestRateSwapBuilder::new("irs_1")
        .money(&notional)
        .fixed_rate(0.05)
        .start(&issue)
        .end(&maturity)
        .discount_curve("USD-OIS")
        .forward_curve("USD-SOFR-3M")
        .side("pay_fixed")
        .business_day_convention(BusinessDayConvention::ModifiedFollowing)
        .calendar_id("usny".to_string())
        .stub_kind(StubKind::none())
        .reset_lag_days(2)
        .build()
        .unwrap();
    let irs_json = irs.to_json().unwrap();
    let irs2 = InterestRateSwap::from_json(irs_json.clone()).unwrap();
    assert_eq!(
        js_stringify(&irs_json),
        js_stringify(&irs2.to_json().unwrap())
    );

    // ---- Swaption
    let swpt = SwaptionBuilder::new("swpt_1")
        .money(&notional)
        .strike(0.05)
        .swaption_type("payer".to_string())
        .expiry(&FsDate::new(2025, 1, 2).unwrap())
        .swap_start(&FsDate::new(2025, 1, 2).unwrap())
        .swap_end(&FsDate::new(2030, 1, 2).unwrap())
        .discount_curve("USD-OIS")
        .forward_curve("USD-SOFR-3M")
        .vol_surface("USD-SWAPTION-VOL")
        .build()
        .unwrap();
    let swpt_json = swpt.to_json().unwrap();
    let swpt2 = Swaption::from_json(swpt_json.clone()).unwrap();
    assert_eq!(
        js_stringify(&swpt_json),
        js_stringify(&swpt2.to_json().unwrap())
    );
}
