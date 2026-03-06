use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_valuations::instruments::fx::fx_forward::FxForward;
use finstack_valuations::instruments::fx::fx_option::FxOption;
use finstack_valuations::instruments::fx::fx_swap::FxSwap;
use finstack_valuations::instruments::OptionType;
use finstack_valuations::market::build_fx_instrument;
use finstack_valuations::market::conventions::ids::FxOptionConventionId;
use finstack_valuations::market::quotes::fx::FxQuote;
use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_valuations::market::BuildCtx;
use time::Month;

fn fx_build_ctx(as_of: Date) -> BuildCtx {
    let mut curve_ids = finstack_core::HashMap::default();
    curve_ids.insert("domestic_discount".to_string(), "USD-OIS".to_string());
    curve_ids.insert("foreign_discount".to_string(), "EUR-OIS".to_string());
    BuildCtx::new(as_of, 1_000_000.0, curve_ids)
}

#[test]
fn test_build_fx_forward_from_outright_quote() {
    let as_of = Date::from_calendar_date(2025, Month::January, 10).unwrap();
    let ctx = fx_build_ctx(as_of);

    let quote = FxQuote::ForwardOutright {
        id: QuoteId::new("EURUSD-FWD-3M"),
        convention: "EUR/USD".into(),
        pillar: Pillar::Tenor("3M".parse().unwrap()),
        forward_rate: 1.1050,
    };

    let instrument = build_fx_instrument(&quote, &ctx).expect("build fx forward");
    assert_eq!(instrument.id(), "EURUSD-FWD-3M");

    let forward = instrument
        .as_any()
        .downcast_ref::<FxForward>()
        .expect("Expected FxForward");
    assert_eq!(forward.base_currency, Currency::EUR);
    assert_eq!(forward.quote_currency, Currency::USD);
    assert_eq!(forward.notional, Money::new(1_000_000.0, Currency::EUR));
    assert_eq!(forward.contract_rate, Some(1.1050));
    assert_eq!(forward.domestic_discount_curve_id.as_str(), "USD-OIS");
    assert_eq!(forward.foreign_discount_curve_id.as_str(), "EUR-OIS");
}

#[test]
fn test_build_fx_swap_from_spot_start_quote() {
    let as_of = Date::from_calendar_date(2025, Month::January, 10).unwrap();
    let ctx = fx_build_ctx(as_of);

    let quote = FxQuote::SwapOutright {
        id: QuoteId::new("EURUSD-SWAP-3M"),
        convention: "EUR/USD".into(),
        far_pillar: Pillar::Tenor("3M".parse().unwrap()),
        near_rate: 1.1000,
        far_rate: 1.1055,
    };

    let instrument = build_fx_instrument(&quote, &ctx).expect("build fx swap");
    assert_eq!(instrument.id(), "EURUSD-SWAP-3M");

    let swap = instrument
        .as_any()
        .downcast_ref::<FxSwap>()
        .expect("Expected FxSwap");
    assert_eq!(swap.base_currency, Currency::EUR);
    assert_eq!(swap.quote_currency, Currency::USD);
    assert_eq!(swap.base_notional, Money::new(1_000_000.0, Currency::EUR));
    assert_eq!(swap.near_rate, Some(1.1000));
    assert_eq!(swap.far_rate, Some(1.1055));
    assert_eq!(swap.domestic_discount_curve_id.as_str(), "USD-OIS");
    assert_eq!(swap.foreign_discount_curve_id.as_str(), "EUR-OIS");
    assert!(swap.far_date > swap.near_date);
}

#[test]
fn test_build_fx_option_from_vanilla_quote() {
    let as_of = Date::from_calendar_date(2025, Month::January, 10).unwrap();
    let ctx = fx_build_ctx(as_of);

    let quote = FxQuote::OptionVanilla {
        id: QuoteId::new("EURUSD-CALL-6M"),
        convention: FxOptionConventionId::new("EUR/USD-VANILLA"),
        expiry: Date::from_calendar_date(2025, Month::July, 10).unwrap(),
        strike: 1.12,
        option_type: OptionType::Call,
        vol_surface_id: "EURUSD-VOL".into(),
    };

    let instrument = build_fx_instrument(&quote, &ctx).expect("build fx option");
    assert_eq!(instrument.id(), "EURUSD-CALL-6M");

    let option = instrument
        .as_any()
        .downcast_ref::<FxOption>()
        .expect("Expected FxOption");
    assert_eq!(option.base_currency, Currency::EUR);
    assert_eq!(option.quote_currency, Currency::USD);
    assert_eq!(option.notional, Money::new(1_000_000.0, Currency::EUR));
    assert_eq!(option.strike, 1.12);
    assert_eq!(option.option_type, OptionType::Call);
    assert_eq!(option.domestic_discount_curve_id.as_str(), "USD-OIS");
    assert_eq!(option.foreign_discount_curve_id.as_str(), "EUR-OIS");
    assert_eq!(option.vol_surface_id.as_str(), "EURUSD-VOL");
}
