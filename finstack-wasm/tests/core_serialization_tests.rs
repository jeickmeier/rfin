use finstack_wasm::{
    BusinessDayConvention, DayCount, DayCountContext, DayCountContextState, Frequency, FsDate,
    ScheduleSpec, StubKind, Tenor,
};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn daycount_context_state_roundtrip() {
    let mut ctx = DayCountContext::new();
    ctx.set_calendar_code("target2");
    ctx.set_frequency(&Frequency::quarterly());
    ctx.set_bus_basis(260);

    let state = ctx.to_state();
    let json = state.to_json().unwrap();
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

    let json = spec.to_json().unwrap();
    let restored = ScheduleSpec::from_json(&json).unwrap();
    let schedule = restored.build().unwrap();
    assert_eq!(schedule.length(), 4);
}
