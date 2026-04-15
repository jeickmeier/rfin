//! Cross-asset portfolio integration test.
//!
//! Prices a small multi-asset portfolio (bond + swap + CDS + FX forward) through
//! the `finstack` umbrella crate end-to-end. The point of the test is to catch
//! bugs at instrument-type boundaries (FX conversion, curve mapping, netting,
//! aggregation) that per-instrument unit tests don't exercise.
//!
//! # Feature requirements
//!
//! This test requires the `valuations` feature of the `finstack` umbrella
//! crate (which transitively pulls in `core`). Run with:
//!
//! ```bash
//! cargo test --test cross_asset_portfolio_integration -p finstack --features all
//! ```
//!
//! or
//!
//! ```bash
//! cargo test --test cross_asset_portfolio_integration -p finstack --features valuations
//! ```
//!
//! The entire file is gated on `cfg(feature = "valuations")`, so it is a
//! silent no-op when the feature is disabled. This keeps the default-feature
//! build green without polluting the test matrix with compile errors.

#![cfg(feature = "valuations")]

use finstack::core::currency::Currency;
use finstack::core::dates::{Date, DayCount};
use finstack::core::market_data::context::MarketContext;
use finstack::core::market_data::term_structures::{DiscountCurve, ForwardCurve, HazardCurve};
use finstack::core::money::fx::{FxMatrix, FxQuery, SimpleFxProvider};
use finstack::core::money::Money;
use finstack::core::types::{CurveId, InstrumentId};

use finstack::valuations::instruments::credit_derivatives::cds::{
    CDSConvention, CreditDefaultSwap, PayReceive as CdsPayReceive, PremiumLegSpec,
    ProtectionLegSpec, RECOVERY_SENIOR_UNSECURED,
};
use finstack::valuations::instruments::fixed_income::bond::Bond;
use finstack::valuations::instruments::fx::fx_forward::FxForward;
use finstack::valuations::instruments::internal::InstrumentExt as Instrument;
use finstack::valuations::instruments::rates::irs::{
    FloatingLegCompounding, InterestRateSwap, PayReceive as IrsPayReceive,
};
use finstack::valuations::instruments::{
    Attributes, FixedLegSpec, FloatLegSpec, PricingOptions, PricingOverrides,
};
use finstack::valuations::metrics::MetricId;

use finstack::core::dates::{BusinessDayConvention, StubKind, Tenor};
use rust_decimal::Decimal;
use std::sync::Arc;
use time::macros::date;

// ---------------------------------------------------------------------------
// Market construction
// ---------------------------------------------------------------------------

const USD_DISC_ID: &str = "USD-OIS";
const EUR_DISC_ID: &str = "EUR-OIS";
const USD_FWD_ID: &str = "USD-SOFR-3M";
const CREDIT_ID: &str = "CORP-SENIOR";

/// Build a USD OIS discount curve with a realistic term structure.
fn usd_discount_curve(as_of: Date) -> DiscountCurve {
    DiscountCurve::builder(USD_DISC_ID)
        .base_date(as_of)
        .day_count(DayCount::Act365F)
        .knots([
            (0.0_f64, 1.0_f64),
            (0.25_f64, (-0.05_f64 * 0.25).exp()),
            (0.5_f64, (-0.05_f64 * 0.5).exp()),
            (1.0_f64, (-0.05_f64).exp()),
            (2.0_f64, (-0.05_f64 * 2.0).exp()),
            (5.0_f64, (-0.05_f64 * 5.0).exp()),
            (10.0_f64, (-0.05_f64 * 10.0).exp()),
        ])
        .build()
        .expect("USD discount curve should build")
}

/// Build a EUR OIS discount curve.
fn eur_discount_curve(as_of: Date) -> DiscountCurve {
    DiscountCurve::builder(EUR_DISC_ID)
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([
            (0.0_f64, 1.0_f64),
            (0.25_f64, (-0.03_f64 * 0.25).exp()),
            (0.5_f64, (-0.03_f64 * 0.5).exp()),
            (1.0_f64, (-0.03_f64).exp()),
            (2.0_f64, (-0.03_f64 * 2.0).exp()),
            (5.0_f64, (-0.03_f64 * 5.0).exp()),
        ])
        .build()
        .expect("EUR discount curve should build")
}

/// Build a flat SOFR-3M forward curve.
fn usd_sofr_forward_curve(as_of: Date) -> ForwardCurve {
    ForwardCurve::builder(USD_FWD_ID, 0.25_f64)
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0_f64, 0.05_f64), (10.0_f64, 0.05_f64)])
        .build()
        .expect("SOFR forward curve should build")
}

/// Build a flat hazard curve for the corporate issuer (~2% hazard, 40% recovery).
fn corp_hazard_curve(as_of: Date) -> HazardCurve {
    HazardCurve::builder(CREDIT_ID)
        .base_date(as_of)
        .recovery_rate(RECOVERY_SENIOR_UNSECURED)
        .knots([
            (0.0_f64, 0.02_f64),
            (1.0_f64, 0.02_f64),
            (5.0_f64, 0.02_f64),
        ])
        .build()
        .expect("credit hazard curve should build")
}

/// Assemble a market context with USD/EUR curves, a SOFR forward curve, a
/// corporate credit hazard curve, and a USD/EUR FX matrix (EUR→USD = 1.10).
fn build_market(as_of: Date) -> MarketContext {
    let fx_provider = Arc::new(SimpleFxProvider::new());
    fx_provider
        .set_quote(Currency::EUR, Currency::USD, 1.10)
        .expect("seed EUR/USD rate");
    let fx_matrix = FxMatrix::new(fx_provider);

    MarketContext::new()
        .insert(usd_discount_curve(as_of))
        .insert(eur_discount_curve(as_of))
        .insert(usd_sofr_forward_curve(as_of))
        .insert(corp_hazard_curve(as_of))
        .insert_fx(fx_matrix)
}

// ---------------------------------------------------------------------------
// Instrument constructors
// ---------------------------------------------------------------------------

/// 5y USD fixed-rate corporate bond, 4% semi-annual, $10M notional.
fn usd_corp_bond(as_of: Date) -> Bond {
    let notional = Money::new(10_000_000.0, Currency::USD);
    let maturity = as_of + time::Duration::days(5 * 365);
    Bond::fixed(
        "USD-CORP-5Y",
        notional,
        0.04_f64,
        as_of,
        maturity,
        USD_DISC_ID,
    )
    .expect("USD corporate bond should build")
}

/// 5y vanilla USD IRS: receive fixed 3%, pay 3M SOFR, $10M notional.
///
/// Explicitly constructed to match the discount (USD-OIS) and forward
/// (USD-SOFR-3M) curves already present in the market context.
fn usd_irs(as_of: Date) -> InterestRateSwap {
    let start = as_of;
    let end = as_of + time::Duration::days(5 * 365);

    let fixed = FixedLegSpec {
        discount_curve_id: CurveId::new(USD_DISC_ID),
        rate: Decimal::try_from(0.03_f64).expect("rate fits in Decimal"),
        frequency: Tenor::semi_annual(),
        day_count: DayCount::Thirty360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: None,
        stub: StubKind::None,
        start,
        end,
        par_method: None,
        compounding_simple: true,
        payment_lag_days: 0,
        end_of_month: false,
    };

    let float = FloatLegSpec {
        discount_curve_id: CurveId::new(USD_DISC_ID),
        forward_curve_id: CurveId::new(USD_FWD_ID),
        spread_bp: Decimal::ZERO,
        frequency: Tenor::quarterly(),
        day_count: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: None,
        fixing_calendar_id: None,
        stub: StubKind::None,
        reset_lag_days: 0,
        compounding: FloatingLegCompounding::Simple,
        payment_lag_days: 0,
        end_of_month: false,
        start,
        end,
    };

    InterestRateSwap::builder()
        .id(InstrumentId::new("USD-IRS-5Y"))
        .notional(Money::new(10_000_000.0, Currency::USD))
        .side(IrsPayReceive::ReceiveFixed)
        .fixed(fixed)
        .float(float)
        .build()
        .expect("USD IRS should build")
}

/// 5y single-name CDS at 100bp, $10M notional — buy protection on the same
/// corporate issuer referenced by the bond.
fn usd_corp_cds(as_of: Date) -> CreditDefaultSwap {
    let convention = CDSConvention::IsdaNa;
    let maturity = as_of + time::Duration::days(5 * 365);

    CreditDefaultSwap::builder()
        .id(InstrumentId::new("USD-CDS-5Y"))
        .notional(Money::new(10_000_000.0, Currency::USD))
        .side(CdsPayReceive::PayFixed) // Buy protection
        .convention(convention)
        .premium(PremiumLegSpec {
            start: as_of,
            end: maturity,
            frequency: convention.frequency(),
            stub: convention.stub_convention(),
            bdc: convention.business_day_convention(),
            calendar_id: Some(convention.default_calendar().to_string()),
            day_count: convention.day_count(),
            spread_bp: Decimal::try_from(100.0_f64).expect("100bp fits in Decimal"),
            discount_curve_id: CurveId::new(USD_DISC_ID),
        })
        .protection(ProtectionLegSpec {
            credit_curve_id: CurveId::new(CREDIT_ID),
            recovery_rate: RECOVERY_SENIOR_UNSECURED,
            settlement_delay: convention.settlement_delay(),
        })
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build()
        .expect("CDS should build")
}

/// 1y EUR/USD FX forward on 5M EUR notional, contract rate 1.12 (slightly
/// off-market so PV is non-zero).
fn eurusd_fx_forward(as_of: Date) -> FxForward {
    let maturity = as_of + time::Duration::days(365);
    FxForward::builder()
        .id(InstrumentId::new("EURUSD-1Y-FWD"))
        .base_currency(Currency::EUR)
        .quote_currency(Currency::USD)
        .maturity(maturity)
        .notional(Money::new(5_000_000.0, Currency::EUR))
        .contract_rate_opt(Some(1.12_f64))
        .domestic_discount_curve_id(CurveId::new(USD_DISC_ID))
        .foreign_discount_curve_id(CurveId::new(EUR_DISC_ID))
        .attributes(Attributes::new())
        .build()
        .expect("FX forward should build")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a `Money` value into USD using the market's FX matrix.
fn to_usd(market: &MarketContext, native: Money, as_of: Date) -> f64 {
    if native.currency() == Currency::USD {
        return native.amount();
    }
    let fx = market
        .fx()
        .expect("market context must have an FX matrix for cross-currency aggregation");
    let rate = fx
        .rate(FxQuery::new(native.currency(), Currency::USD, as_of))
        .expect("FX lookup should succeed")
        .rate;
    native.amount() * rate
}

// ---------------------------------------------------------------------------
// The cross-asset integration test
// ---------------------------------------------------------------------------

/// Prices a mini cross-asset portfolio (bond + swap + CDS + FX forward),
/// aggregates into a single USD total, and sanity-checks per-instrument PVs
/// plus a rates DV01 sum.
///
/// The goal is to exercise the boundaries between instrument types: FX
/// conversion (EUR leg → USD), curve mapping (IRS using separate discount
/// vs forward curves, bond + CDS sharing USD discount), and portfolio-level
/// aggregation. If any of those pipes break, this test should catch it.
#[test]
fn cross_asset_portfolio_prices_and_aggregates_in_usd() {
    let as_of = date!(2024 - 01 - 15);
    let market = build_market(as_of);

    // --- Build instruments -------------------------------------------------
    let bond = usd_corp_bond(as_of);
    let swap = usd_irs(as_of);
    let cds = usd_corp_cds(as_of);
    let fx_fwd = eurusd_fx_forward(as_of);

    // --- Price individually ------------------------------------------------
    let bond_pv = bond.value(&market, as_of).expect("bond should price");
    let swap_pv = swap.value(&market, as_of).expect("swap should price");
    let cds_pv = cds.value(&market, as_of).expect("CDS should price");
    let fx_pv = fx_fwd
        .value(&market, as_of)
        .expect("FX forward should price");

    // Every PV must be finite.
    assert!(bond_pv.amount().is_finite(), "bond PV must be finite");
    assert!(swap_pv.amount().is_finite(), "swap PV must be finite");
    assert!(cds_pv.amount().is_finite(), "CDS PV must be finite");
    assert!(fx_pv.amount().is_finite(), "FX forward PV must be finite");

    // The bond and FX forward are not at-market, so their PVs must be
    // materially non-zero. (Swap and CDS can be near zero if by luck they're
    // close to par, so we only assert finiteness for those.)
    assert!(
        bond_pv.amount().abs() > 1.0,
        "bond PV should be materially non-zero, got {}",
        bond_pv.amount()
    );
    assert!(
        fx_pv.amount().abs() > 1.0,
        "FX forward PV should be materially non-zero (off-market rate), got {}",
        fx_pv.amount()
    );

    // Native currencies should match expectations.
    assert_eq!(bond_pv.currency(), Currency::USD, "bond PV in USD");
    assert_eq!(swap_pv.currency(), Currency::USD, "swap PV in USD");
    assert_eq!(cds_pv.currency(), Currency::USD, "CDS PV in USD");
    assert_eq!(
        fx_pv.currency(),
        Currency::USD,
        "EUR/USD FX forward PV is quoted in USD"
    );

    // --- Aggregate into USD total -----------------------------------------
    // Exercises the FX conversion path even though all four instruments
    // happen to be reporting USD (the FX forward on a EUR notional already
    // prices in USD). We also spot-check the helper against a true EUR
    // cashflow below.
    let portfolio_usd_total: f64 = [bond_pv, swap_pv, cds_pv, fx_pv]
        .iter()
        .map(|m| to_usd(&market, *m, as_of))
        .sum();

    assert!(
        portfolio_usd_total.is_finite(),
        "portfolio USD total must be finite, got {}",
        portfolio_usd_total
    );

    // Sanity bound: the portfolio is ~$30M of gross notional, so a sane
    // aggregate PV should be well under the notional sum in magnitude.
    let gross_notional_usd = 30_000_000.0 + 5_000_000.0 * 1.10; // 10M+10M+10M USD + 5M EUR→USD
    assert!(
        portfolio_usd_total.abs() < gross_notional_usd,
        "portfolio PV magnitude {} exceeds gross notional {}",
        portfolio_usd_total,
        gross_notional_usd
    );

    // --- FX conversion boundary check -------------------------------------
    // Directly convert a synthetic 1M EUR cashflow using our helper and
    // verify it lines up with the 1.10 spot FX rate we seeded. This locks
    // down the `to_usd` aggregation path even if every instrument above
    // happened to natively report in USD.
    let synthetic_eur = Money::new(1_000_000.0, Currency::EUR);
    let converted = to_usd(&market, synthetic_eur, as_of);
    assert!(
        (converted - 1_100_000.0).abs() < 1e-6,
        "1M EUR should convert to 1.1M USD at rate 1.10, got {}",
        converted
    );

    // --- Optional rates risk metric: DV01 sum -----------------------------
    // Only the bond and swap are (primarily) rates instruments. Compute DV01
    // on each via `price_with_metrics`, and assert the sum is finite and
    // non-zero. This exercises the metrics pipeline on top of raw pricing.
    let bond_metrics = bond
        .price_with_metrics(&market, as_of, &[MetricId::Dv01], PricingOptions::default())
        .expect("bond DV01 should compute");
    let swap_metrics = swap
        .price_with_metrics(&market, as_of, &[MetricId::Dv01], PricingOptions::default())
        .expect("swap DV01 should compute");

    let bond_dv01 = bond_metrics.measures[&MetricId::Dv01];
    let swap_dv01 = swap_metrics.measures[&MetricId::Dv01];
    assert!(bond_dv01.is_finite(), "bond DV01 finite");
    assert!(swap_dv01.is_finite(), "swap DV01 finite");

    let rates_dv01_sum = bond_dv01 + swap_dv01;
    assert!(rates_dv01_sum.is_finite(), "rates DV01 sum finite");
    assert!(
        rates_dv01_sum.abs() > 1e-6,
        "rates DV01 sum should be materially non-zero, got {}",
        rates_dv01_sum
    );
}
