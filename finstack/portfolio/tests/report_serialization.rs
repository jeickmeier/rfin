use finstack_core::cashflow::CFKind;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, Month, PeriodId};
use finstack_core::money::Money;
use finstack_portfolio::cashflows::{
    CashflowExtractionIssue, CashflowExtractionIssueKind, CashflowWarning,
    PortfolioCashflowBuckets, PortfolioCashflowEvent, PortfolioCashflowKindBuckets,
    PortfolioCashflowPositionSummary, PortfolioCashflows, PortfolioFullCashflows,
};
use finstack_portfolio::dependencies::MarketFactorKey;
use finstack_portfolio::types::PositionId;
use finstack_valuations::cashflow::builder::CashflowRepresentation;
use finstack_valuations::cashflow::DatedFlows;
use finstack_valuations::instruments::RatesCurveKind;
use indexmap::IndexMap;

fn roundtrip_json<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let json = serde_json::to_string(value).expect("serialization should succeed");
    serde_json::from_str(&json).expect("deserialization should succeed")
}

fn assert_roundtrip_value<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let restored = roundtrip_json(value);
    assert_eq!(
        serde_json::to_value(value).expect("value serialization should succeed"),
        serde_json::to_value(&restored).expect("value reserialization should succeed")
    );
}

fn make_date(day: u8) -> Date {
    Date::from_calendar_date(2025, Month::January, day).expect("valid date")
}

#[test]
fn test_cashflow_report_types_roundtrip() {
    let position_id = PositionId::new("POS_1");
    let payment_date = make_date(15);

    assert_roundtrip_value(&CashflowWarning {
        position_id: position_id.clone(),
        instrument_id: "BOND_A".to_string(),
        instrument_type: "Bond".to_string(),
        message: "schedule construction failed".to_string(),
    });

    assert_roundtrip_value(&CashflowExtractionIssue {
        position_id: position_id.clone(),
        instrument_id: "LOAN_B".to_string(),
        instrument_type: "Loan".to_string(),
        kind: CashflowExtractionIssueKind::BuildFailed,
        message: "missing forward curve".to_string(),
    });

    let summary = PortfolioCashflowPositionSummary {
        position_id: position_id.clone(),
        instrument_id: "BOND_A".to_string(),
        instrument_type: "Bond".to_string(),
        representation: CashflowRepresentation::Projected,
        event_count: 1,
    };
    assert_roundtrip_value(&summary);

    let event = PortfolioCashflowEvent {
        position_id: position_id.clone(),
        instrument_id: "BOND_A".to_string(),
        instrument_type: "Bond".to_string(),
        date: payment_date,
        amount: Money::new(12_500.0, Currency::USD),
        kind: CFKind::Fixed,
        reset_date: Some(make_date(10)),
        accrual_factor: 0.25,
        rate: Some(0.05),
    };
    assert_roundtrip_value(&event);

    let mut full_by_position = IndexMap::new();
    full_by_position.insert(position_id.clone(), vec![event.clone()]);

    let mut full_by_date = IndexMap::new();
    let mut per_currency = IndexMap::new();
    let mut per_kind = IndexMap::new();
    per_kind.insert(CFKind::Fixed, Money::new(12_500.0, Currency::USD));
    per_currency.insert(Currency::USD, per_kind);
    full_by_date.insert(payment_date, per_currency);

    let mut summaries = IndexMap::new();
    summaries.insert(position_id.clone(), summary.clone());

    assert_roundtrip_value(&PortfolioFullCashflows {
        events: vec![event.clone()],
        by_position: full_by_position,
        by_date: full_by_date,
        position_summaries: summaries.clone(),
        issues: vec![CashflowExtractionIssue {
            position_id: PositionId::new("POS_2"),
            instrument_id: "SWAP_C".to_string(),
            instrument_type: "Swap".to_string(),
            kind: CashflowExtractionIssueKind::Unsupported,
            message: "cashflow provider unavailable".to_string(),
        }],
    });

    let dated_flows: DatedFlows = vec![(payment_date, Money::new(12_500.0, Currency::USD))];
    let mut ladder_by_position = IndexMap::new();
    ladder_by_position.insert(position_id.clone(), dated_flows);

    let mut ladder_by_date = IndexMap::new();
    let mut totals = IndexMap::new();
    totals.insert(Currency::USD, Money::new(12_500.0, Currency::USD));
    ladder_by_date.insert(payment_date, totals);

    assert_roundtrip_value(&PortfolioCashflows {
        by_date: ladder_by_date,
        by_position: ladder_by_position,
        position_summaries: summaries,
        warnings: vec![CashflowWarning {
            position_id,
            instrument_id: "BOND_A".to_string(),
            instrument_type: "Bond".to_string(),
            message: "used placeholder schedule".to_string(),
        }],
    });

    let mut by_period = IndexMap::new();
    by_period.insert(
        PeriodId::month(2025, 1),
        Money::new(50_000.0, Currency::USD),
    );
    assert_roundtrip_value(&PortfolioCashflowBuckets { by_period });

    let mut by_kind_period = IndexMap::new();
    let mut period_totals = IndexMap::new();
    period_totals.insert(CFKind::Fixed, Money::new(40_000.0, Currency::USD));
    period_totals.insert(CFKind::Notional, Money::new(-10_000.0, Currency::USD));
    by_kind_period.insert(PeriodId::month(2025, 1), period_totals);
    assert_roundtrip_value(&PortfolioCashflowKindBuckets {
        by_period: by_kind_period,
    });
}

#[test]
fn test_dependency_key_roundtrip() {
    assert_roundtrip_value(&MarketFactorKey::Curve {
        id: "USD-OIS".into(),
        kind: RatesCurveKind::Discount,
    });
    assert_roundtrip_value(&MarketFactorKey::Fx {
        base: Currency::EUR,
        quote: Currency::USD,
    });
}
