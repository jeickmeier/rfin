use finstack_core::currency::Currency;
use finstack_core::dates::{Date, Tenor};
use finstack_valuations::instruments::deposit::Deposit;
use finstack_valuations::instruments::fra::ForwardRateAgreement;
use finstack_valuations::instruments::irs::InterestRateSwap;
use finstack_valuations::market::build::context::BuildCtx;
use finstack_valuations::market::build::rates::build_rate_instrument;

use finstack_valuations::market::quotes::ids::Pillar;
use finstack_valuations::market::quotes::rates::RateQuote; // Used in code if needed, or string into

#[test]
fn test_build_deposit() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 10).unwrap();
    let ctx = BuildCtx {
        as_of,
        curve_ids: Default::default(),
        notional: 1_000_000.0,
        attributes: Default::default(),
    };

    let quote = RateQuote::Deposit {
        id: "USD-DEP-3M".into(),
        index: "USD-LIBOR-3M".into(),
        pillar: Pillar::Tenor(Tenor::parse("3M").unwrap()),
        rate: 0.035,
    };

    let instrument = build_rate_instrument(&quote, &ctx).expect("build deposit");
    assert_eq!(instrument.id(), "USD-DEP-3M");

    if let Some(dep) = instrument.as_any().downcast_ref::<Deposit>() {
        assert_eq!(dep.notional.currency(), Currency::USD);
        assert_eq!(dep.quote_rate, Some(0.035));
    } else {
        panic!("Expected Deposit");
    }
}

#[test]
fn test_build_fra() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 10).unwrap();
    let ctx = BuildCtx {
        as_of,
        curve_ids: Default::default(),
        notional: 1_000_000.0,
        attributes: Default::default(),
    };

    let quote = RateQuote::Fra {
        id: "USD-FRA-3x6".into(),
        index: "USD-LIBOR-3M".into(), // Use 3M conventions for FRA
        start: Pillar::Tenor(Tenor::parse("3M").unwrap()),
        end: Pillar::Tenor(Tenor::parse("6M").unwrap()),
        rate: 0.032,
    };

    let instrument = build_rate_instrument(&quote, &ctx).expect("build fra");
    assert_eq!(instrument.id(), "USD-FRA-3x6");

    if let Some(fra) = instrument.as_any().downcast_ref::<ForwardRateAgreement>() {
        assert_eq!(fra.notional.currency(), Currency::USD);
        assert_eq!(fra.fixed_rate, 0.032);
    } else {
        panic!("Expected ForwardRateAgreement");
    }
}

#[test]
fn test_build_swap() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 10).unwrap();
    let ctx = BuildCtx {
        as_of,
        curve_ids: Default::default(),
        notional: 1_000_000.0,
        attributes: Default::default(),
    };

    // Use USD-SOFR-3M (Term SOFR style or OIS compounded, RateIndexKind=Term in registry will determine builder path)
    // If it's OIS (OvernightRfr), builder uses create_ois_swap_with_conventions.
    // If Term, create_term_swap_with_conventions.
    // Let's assume correct behavior.
    let quote = RateQuote::Swap {
        id: "USD-SWAP-5Y".into(),
        index: "USD-SOFR-3M".into(),
        pillar: Pillar::Tenor(Tenor::parse("5Y").unwrap()),
        rate: 0.030,
        spread: None,
    };

    let instrument = build_rate_instrument(&quote, &ctx).expect("build swap");
    assert_eq!(instrument.id(), "USD-SWAP-5Y");

    if let Some(swap) = instrument.as_any().downcast_ref::<InterestRateSwap>() {
        assert_eq!(swap.notional.currency(), Currency::USD);
        assert_eq!(swap.fixed.rate, 0.030);
    } else {
        panic!("Expected InterestRateSwap");
    }
}

#[test]
fn test_build_futures() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 10).unwrap();
    let ctx = BuildCtx {
        as_of,
        curve_ids: Default::default(),
        notional: 1_000_000.0,
        attributes: Default::default(),
    };

    let quote = RateQuote::Futures {
        id: "USD-FUT-SEP25".into(),
        contract: "SR3".into(),
        expiry: Date::from_calendar_date(2025, time::Month::September, 15).unwrap(),
        price: 96.50,
        convexity_adjustment: None,
    };

    let instrument = build_rate_instrument(&quote, &ctx).expect("build futures");
    assert_eq!(instrument.id(), "USD-FUT-SEP25");

    if instrument
        .as_any()
        .downcast_ref::<finstack_valuations::instruments::ir_future::InterestRateFuture>()
        .is_none()
    {
        panic!("Expected InterestRateFuture");
    }
}
