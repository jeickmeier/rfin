//! Parity tests for Calibration v2.

use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_core::prelude::*;
use finstack_core::types::Currency;
use finstack_valuations::calibration::v2::api::engine;
use finstack_valuations::calibration::v2::api::schema::{
    CalibrationEnvelopeV2, CalibrationMethod, CalibrationPlanV2, CalibrationStepV2,
    DiscountCurveParams, ForwardCurveParams, StepParams,
};
use finstack_valuations::calibration::v2::domain::quotes::{
    InstrumentConventions, MarketQuote, RatesQuote,
};
use std::collections::HashMap;
use time::Month;

#[test]
fn test_v2_simple_usd_calibration() {
    let base_date = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let currency = Currency::USD;

    // 1. Create Quotes
    let mut quotes = Vec::new();
    
    // Deposit (Discount)
    quotes.push(MarketQuote::Rates(RatesQuote::Deposit {
        maturity: base_date + time::Duration::days(1), // Overnight
        rate: 0.05,
        conventions: InstrumentConventions::default()
            .with_day_count(DayCount::Act360)
            .with_settlement_days(0),
    }));

    // OIS Swaps (Discount)
    let tenors = vec![
        (1, 0.0505),
        (3, 0.0510),
        (6, 0.0515),
        (12, 0.0525),
        (24, 0.0535),
    ];
    for (months, rate) in tenors {
        quotes.push(MarketQuote::Rates(RatesQuote::Swap {
            maturity: base_date.add_months(months),
            rate,
            is_ois: true,
            conventions: InstrumentConventions::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(finstack_core::dates::Tenor::annual())
                .with_day_count(DayCount::Act360),
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(finstack_core::dates::Tenor::annual())
                .with_day_count(DayCount::Act360)
                .with_index("USD-SOFR"),
        }));
    }

    // Forward Quotes (3M FRAs)
    let fwd_quotes = vec![
        MarketQuote::Rates(RatesQuote::FRA {
            start: base_date,
            end: base_date.add_months(3),
            rate: 0.0530,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        }),
        MarketQuote::Rates(RatesQuote::FRA {
            start: base_date.add_months(3),
            end: base_date.add_months(6),
            rate: 0.0540,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        }),
    ];

    let mut quote_sets = HashMap::new();
    quote_sets.insert("usd_ois".to_string(), quotes);
    quote_sets.insert("usd_3m".to_string(), fwd_quotes);

    // 2. Build Plan
    let plan = CalibrationPlanV2 {
        id: "test_plan".to_string(),
        description: None,
        quote_sets,
        settings: Default::default(),
        steps: vec![
            CalibrationStepV2 {
                id: "step_1".to_string(),
                quote_set: "usd_ois".to_string(),
                params: StepParams::Discount(DiscountCurveParams {
                    curve_id: "USD-OIS".into(),
                    currency,
                    base_date,
                    method: CalibrationMethod::Bootstrap,
                    interpolation: Default::default(),
                    extrapolation: Default::default(),
                    pricing_discount_id: None,
                    pricing_forward_id: None,
                }),
            },
            CalibrationStepV2 {
                id: "step_2".to_string(),
                quote_set: "usd_3m".to_string(),
                params: StepParams::Forward(ForwardCurveParams {
                    curve_id: "USD-3M".into(),
                    currency,
                    base_date,
                    tenor_years: 0.25,
                    discount_curve_id: "USD-OIS".into(),
                    method: CalibrationMethod::Bootstrap,
                    interpolation: Default::default(),
                }),
            },
        ],
    };

    let envelope = CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: Some(MarketContextState::from(&MarketContext::new())),
    };

    // 3. Execute
    let result = engine::execute(&envelope).expect("Calibration failed");

    // Forward rate checks might need adjustment if rate changes due to different date
    // But since market data is synthetic/flat-ish, it should be robust.
    
    // 4. Verify
    assert!(result.result.report.success);
    
    let context = MarketContext::try_from(result.result.final_market).expect("Failed to restore context");

    // Check Discount Curve
    let discount = context.get_discount("USD-OIS").expect("Discount curve missing");
    let df_1y = discount.df(1.0);
    assert!(df_1y < 1.0 && df_1y > 0.9, "Reasonable DF");

    // Check Forward Curve
    let forward = context.get_forward("USD-3M").expect("Forward curve missing");
    let fwd_0 = forward.rate(0.0);
    assert!((fwd_0 - 0.0530).abs() < 1e-4, "Spot forward should match first FRA");
}

