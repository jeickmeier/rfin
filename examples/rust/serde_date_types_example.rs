//! Example demonstrating serialization of date and schedule types

use finstack_core::dates::{
    build_periods, BusinessDayConvention, Date, FiscalConfig, Frequency, Month,
    ScheduleBuilder, StubKind,
};
use finstack_core::dates::periods::{Period, PeriodId, PeriodPlan};
use finstack_core::dates::schedule_iter::Schedule;
use finstack_core::dates::calendar::composite::CompositeMode;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Date Types Serialization Example ===\n");

    // 1. PeriodId serialization
    println!("1. PeriodId Serialization:");
    let period_id = PeriodId::quarter(2025, 2);
    let json = serde_json::to_string_pretty(&period_id)?;
    println!("   PeriodId: 2025Q2");
    println!("   JSON: {}", json);
    let deserialized: PeriodId = serde_json::from_str(&json)?;
    println!("   Deserialized: {}\n", deserialized);

    // 2. FiscalConfig serialization
    println!("2. FiscalConfig Serialization:");
    let fiscal_config = FiscalConfig::us_federal();
    let json = serde_json::to_string_pretty(&fiscal_config)?;
    println!("   US Federal Fiscal Year (starts Oct 1)");
    println!("   JSON: {}", json);
    println!();

    // 3. Period serialization
    println!("3. Period Serialization:");
    let period = Period {
        id: PeriodId::month(2025, 3),
        start: Date::from_calendar_date(2025, Month::March, 1)?,
        end: Date::from_calendar_date(2025, Month::April, 1)?,
        is_actual: true,
    };
    let json = serde_json::to_string_pretty(&period)?;
    println!("   Period: March 2025 (actual)");
    println!("   JSON: {}", json);
    println!();

    // 4. PeriodPlan serialization
    println!("4. PeriodPlan Serialization:");
    let plan = build_periods("2025Q1..Q2", Some("2025Q1"))?;
    let json = serde_json::to_string_pretty(&plan)?;
    println!("   Period Plan: 2025Q1-Q2 with Q1 as actual");
    println!("   JSON (truncated): {}", &json[..200.min(json.len())]);
    println!("   ...");
    println!();

    // 5. Frequency serialization
    println!("5. Frequency Serialization:");
    let frequencies = vec![
        ("Annual", Frequency::annual()),
        ("Quarterly", Frequency::quarterly()),
        ("Monthly", Frequency::monthly()),
        ("Biweekly", Frequency::biweekly()),
    ];
    for (name, freq) in frequencies {
        let json = serde_json::to_string(&freq)?;
        println!("   {}: {}", name, json);
    }
    println!();

    // 6. StubKind serialization
    println!("6. StubKind Serialization:");
    let stub_kinds = vec![
        ("None", StubKind::None),
        ("ShortFront", StubKind::ShortFront),
        ("ShortBack", StubKind::ShortBack),
        ("LongFront", StubKind::LongFront),
        ("LongBack", StubKind::LongBack),
    ];
    for (name, stub) in stub_kinds {
        let json = serde_json::to_string(&stub)?;
        println!("   {}: {}", name, json);
    }
    println!();

    // 7. Schedule serialization
    println!("7. Schedule Serialization:");
    let start = Date::from_calendar_date(2025, Month::January, 15)?;
    let end = Date::from_calendar_date(2025, Month::April, 15)?;
    let schedule = ScheduleBuilder::new(start, end)
        .frequency(Frequency::monthly())
        .stub_rule(StubKind::None)
        .build()?;
    let json = serde_json::to_string_pretty(&schedule)?;
    println!("   Monthly schedule from Jan 15 to Apr 15, 2025");
    println!("   JSON: {}", json);
    println!();

    // 8. BusinessDayConvention serialization (with snake_case)
    println!("8. BusinessDayConvention Serialization (snake_case):");
    let conventions = vec![
        ("Unadjusted", BusinessDayConvention::Unadjusted),
        ("Following", BusinessDayConvention::Following),
        ("ModifiedFollowing", BusinessDayConvention::ModifiedFollowing),
        ("Preceding", BusinessDayConvention::Preceding),
        ("ModifiedPreceding", BusinessDayConvention::ModifiedPreceding),
    ];
    for (name, conv) in conventions {
        let json = serde_json::to_string(&conv)?;
        println!("   {}: {}", name, json);
    }
    println!();

    // 9. CompositeMode serialization
    println!("9. CompositeMode Serialization:");
    let modes = vec![
        ("Union", CompositeMode::Union),
        ("Intersection", CompositeMode::Intersection),
    ];
    for (name, mode) in modes {
        let json = serde_json::to_string(&mode)?;
        println!("   {}: {}", name, json);
    }
    println!();

    // 10. Round-trip test with complex structure
    println!("10. Complex Round-trip Test:");
    let complex_plan = build_periods("2025Q1..Q4", Some("2025Q2"))?;
    let json = serde_json::to_string(&complex_plan)?;
    let deserialized: PeriodPlan = serde_json::from_str(&json)?;
    println!("   Original periods: {}", complex_plan.periods.len());
    println!("   Deserialized periods: {}", deserialized.periods.len());
    println!("   Round-trip successful: {}", 
             complex_plan.periods.len() == deserialized.periods.len());

    println!("\n=== All serialization tests completed successfully! ===");

    Ok(())
}
