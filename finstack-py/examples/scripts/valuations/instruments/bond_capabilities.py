#!/usr/bin/env python3
from datetime import date

from finstack.core.currency import USD
from finstack.core.dates import BusinessDayConvention
from finstack.core.dates.daycount import DayCount
from finstack.core.dates.schedule import Frequency, StubKind
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve
from finstack.valuations.cashflow import CashflowBuilder, CouponType, FixedCouponSpec, ScheduleParams
from finstack.valuations.instruments import Bond
from finstack.valuations.metrics import MetricId, MetricRegistry
from finstack.valuations.pricer import create_standard_registry

from finstack import Money


def build_market(as_of: date) -> MarketContext:
    disc = DiscountCurve(
        "USD-OIS",
        as_of,
        [(0.0, 1.0), (2.0, 0.97), (5.0, 0.90)],
    )
    # Treasury curve used by Bond.treasury helper
    disc_tsy = DiscountCurve(
        "USD-TREASURY",
        as_of,
        [(0.0, 1.0), (5.0, 0.98)],
    )
    fwd = ForwardCurve(
        "USD-SOFR-3M",
        0.25,
        [(0.0, 0.02), (5.0, 0.02)],
        base_date=as_of,
    )
    market = MarketContext()
    market.insert_discount(disc)
    market.insert_discount(disc_tsy)
    market.insert_forward(fwd)
    return market


def build_custom_schedule(issue: date, maturity: date, notional: Money):
    schedule_params = ScheduleParams.new(
        Frequency.SEMI_ANNUAL,
        DayCount.THIRTY_360,
        BusinessDayConvention.MODIFIED_FOLLOWING,
        calendar_id="usny",
        stub=StubKind.NONE,
    )
    fixed_5pct = FixedCouponSpec.new(
        rate=0.05,
        schedule=schedule_params,
        coupon_type=CouponType.split(0.7, 0.3),  # 70% cash, 30% PIK
    )
    cfb = (
        CashflowBuilder.new()
        .principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        .fixed_cf(fixed_5pct)
    )
    return cfb.build_with_curves(None)


def main() -> None:
    # Always pass an explicit as_of to pricing calls; PricerRegistry otherwise defaults
    # to "today", which can accidentally price the instrument after some cashflows.
    as_of = date(2025, 1, 16)
    market = build_market(as_of)

    issue = date(2025, 1, 15)
    maturity = date(2030, 1, 15)
    notional = Money(1_000_000, USD)

    # A) Custom schedule with PIK + amortization; call schedule defined in instrument
    schedule = build_custom_schedule(issue, maturity, notional)
    [(date(2028, 1, 15), 102.0), (date(2029, 1, 15), 101.0)]
    bond_custom = Bond.fixed_semiannual(
        "BOND-CUSTOM-PIK-AMORT-CALL",
        notional,
        0.05,
        issue,
        maturity,
        "USD-OIS",
    )

    # Or create directly from schedule
    bond_from_sched = Bond.from_cashflows(
        instrument_id="BOND-CUSTOM-SCHED",
        schedule=schedule,
        discount_curve="USD-OIS",
        quoted_clean=99.2,
    )

    # B) FRN via helper
    bond_frn = Bond.floating(
        "BOND-FRN",
        notional,
        issue,
        maturity,
        "USD-OIS",
        "USD-SOFR-3M",
        150.0,
    )

    # Price examples
    reg = create_standard_registry()
    reg.price(bond_custom, "discounting", market, as_of=as_of)
    reg.price(bond_from_sched, "discounting", market, as_of=as_of)
    reg.price(bond_frn, "discounting", market, as_of=as_of)

    # Show first few flows from schedule
    flows = schedule.flows()
    [f.to_tuple()[:3] for f in flows[:3]]

    # C) Zero-coupon bond
    zcb = Bond.zero_coupon(
        instrument_id="BOND-ZERO",
        notional=notional,
        issue=issue,
        maturity=maturity,
        discount_curve="USD-OIS",
    )
    reg.price(zcb, "discounting", market, as_of=as_of)

    # D) Fixed bond helper priced off USD-OIS
    fixed = Bond.fixed_semiannual(
        "BOND-FIXED",
        notional,
        0.03,
        issue,
        maturity,
        "USD-OIS",
    )
    reg.price(fixed, "discounting", market, as_of=as_of)

    # E) Payment split program: switch 100% PIK for the first year, then 100% cash
    cfb2 = (
        CashflowBuilder.new()
        .principal(amount=notional.amount, currency=USD, issue=issue, maturity=maturity)
        .fixed_cf(
            FixedCouponSpec.new(rate=0.055, schedule=ScheduleParams.semiannual_30360(), coupon_type=CouponType.CASH)
        )
        .payment_split_program([
            (date(2026, 1, 15), CouponType.PIK),  # up to this date: PIK
            (maturity, CouponType.CASH),  # remainder: cash
        ])
    )
    sched2 = cfb2.build_with_curves(None)
    bond_split = Bond.from_cashflows(
        instrument_id="BOND-SPLIT-PIK-CASH",
        schedule=sched2,
        discount_curve="USD-OIS",
        quoted_clean=100.25,
    )
    reg.price(bond_split, "discounting", market, as_of=as_of)

    # F) Bond metrics examples — request standard metrics from engine (standard fixed-rate bond)
    metrics = [
        MetricId.from_name("accrued"),
        MetricId.from_name("clean_price"),
        MetricId.from_name("dirty_price"),
        MetricId.from_name("ytm"),
        MetricId.from_name("duration_mac"),
        MetricId.from_name("duration_mod"),
        MetricId.from_name("convexity"),
        MetricId.from_name("z_spread"),
        MetricId.from_name("i_spread"),
        MetricId.from_name("asw_par"),
        MetricId.from_name("asw_market"),
    ]

    # Build a registry of metrics (engine uses standard registry internally)
    MetricRegistry.standard()

    # Build a standard fixed bond with a quoted clean price for metrics
    fixed_for_metrics = Bond.builder(
        instrument_id="BOND-FIXED-METRICS",
        notional=notional,
        issue=issue,
        maturity=maturity,
        discount_curve="USD-OIS",
        coupon_rate=0.05,
        frequency=Frequency.SEMI_ANNUAL,
        day_count=DayCount.THIRTY_360,
        bdc=BusinessDayConvention.FOLLOWING,
        stub=StubKind.NONE,
        quoted_clean_price=100.5,
    )

    # Price with metrics for the standard fixed-rate bond
    metrics_core = [m for m in metrics if m.name]
    res_custom_metrics = reg.price_with_metrics(
        fixed_for_metrics,
        "discounting",
        market,
        [m.name for m in metrics_core],
        as_of=as_of,
    )
    measures = res_custom_metrics.measures
    for m in metrics_core:
        name = m.name
        if isinstance(name, str) and name in measures:
            pass


if __name__ == "__main__":
    main()
