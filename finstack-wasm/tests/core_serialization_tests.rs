use finstack_wasm::{
    Bond, BusinessDayConvention, CreditDefaultSwap, Currency, DayCount, DayCountContext,
    DayCountContextState, Deposit, Equity, EquityOption, Frequency, FsDate, FxOption, FxSpot,
    InterestRateSwap, Money, ScheduleSpec, StubKind, Swaption, Tenor,
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
    let dep = Deposit::new(
        "dep_1",
        &notional,
        &issue,
        &FsDate::new(2024, 7, 2).unwrap(),
        &DayCount::act_360(),
        "USD-OIS",
        Some(0.05),
    )
    .unwrap();
    let dep_json = dep.to_json().unwrap();
    let dep2 = Deposit::from_json(dep_json.clone()).unwrap();
    assert_eq!(
        js_stringify(&dep_json),
        js_stringify(&dep2.to_json().unwrap())
    );

    // ---- Equity
    let eq = Equity::new("eq_1", "AAPL", &usd, Some(100.0), Some(200.0));
    let eq_json = eq.to_json().unwrap();
    let eq2 = Equity::from_json(eq_json.clone()).unwrap();
    assert_eq!(
        js_stringify(&eq_json),
        js_stringify(&eq2.to_json().unwrap())
    );

    // ---- EquityOption
    let eq_opt = EquityOption::new(
        "eqopt_1",
        "AAPL",
        200.0,
        "call",
        &FsDate::new(2025, 1, 2).unwrap(),
        &notional,
        Some(1.0),
    )
    .unwrap();
    let eq_opt_json = eq_opt.to_json().unwrap();
    let eq_opt2 = EquityOption::from_json(eq_opt_json.clone()).unwrap();
    assert_eq!(
        js_stringify(&eq_opt_json),
        js_stringify(&eq_opt2.to_json().unwrap())
    );

    // ---- FX spot / option / swap
    let fx_spot = FxSpot::new("fxspot_1", &eur, &usd, None, Some(1.10), None).unwrap();
    let fx_spot_json = fx_spot.to_json().unwrap();
    let fx_spot2 = FxSpot::from_json(fx_spot_json.clone()).unwrap();
    assert_eq!(
        js_stringify(&fx_spot_json),
        js_stringify(&fx_spot2.to_json().unwrap())
    );

    let fx_opt = FxOption::new(
        "fxopt_1",
        &eur,
        &usd,
        1.10,
        "call",
        &FsDate::new(2024, 7, 2).unwrap(),
        &notional,
        "USD-OIS",
        "EUR-OIS",
        "EURUSD-VOL",
    )
    .unwrap();
    let fx_opt_json = fx_opt.to_json().unwrap();
    let fx_opt2 = FxOption::from_json(fx_opt_json.clone()).unwrap();
    assert_eq!(
        js_stringify(&fx_opt_json),
        js_stringify(&fx_opt2.to_json().unwrap())
    );

    // ---- CDS
    let cds = CreditDefaultSwap::new(
        "cds_1",
        &notional,
        100.0,
        &as_of,
        &maturity,
        "USD-OIS",
        "ACME-HAZARD",
        "buy_protection",
        Some(0.4),
    )
    .unwrap();
    let cds_json = cds.to_json().unwrap();
    let cds2 = CreditDefaultSwap::from_json(cds_json.clone()).unwrap();
    assert_eq!(
        js_stringify(&cds_json),
        js_stringify(&cds2.to_json().unwrap())
    );

    // ---- IRS (minimal fixed/float)
    let irs = InterestRateSwap::new(
        "irs_1",
        &notional,
        0.05,
        &issue,
        &maturity,
        "USD-OIS",
        "USD-SOFR-3M",
        "pay_fixed",
        None,
        None,
        None,
        None,
        Some(BusinessDayConvention::ModifiedFollowing),
        Some("usny".to_string()),
        Some(StubKind::none()),
        Some(2),
    )
    .unwrap();
    let irs_json = irs.to_json().unwrap();
    let irs2 = InterestRateSwap::from_json(irs_json.clone()).unwrap();
    assert_eq!(
        js_stringify(&irs_json),
        js_stringify(&irs2.to_json().unwrap())
    );

    // ---- Swaption
    let swpt = Swaption::new(
        "swpt_1",
        &notional,
        0.05,
        "payer",
        &FsDate::new(2025, 1, 2).unwrap(),
        &FsDate::new(2025, 1, 2).unwrap(),
        &FsDate::new(2030, 1, 2).unwrap(),
        "USD-OIS",
        "USD-SOFR-3M",
        "USD-SWAPTION-VOL",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    let swpt_json = swpt.to_json().unwrap();
    let swpt2 = Swaption::from_json(swpt_json.clone()).unwrap();
    assert_eq!(
        js_stringify(&swpt_json),
        js_stringify(&swpt2.to_json().unwrap())
    );
}
