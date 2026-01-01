#!/usr/bin/env python3
"""Total return swap examples for equity and fixed-income index underlyings."""

from datetime import date

from finstack.core.currency import USD
from finstack.core.dates import BusinessDayConvention
from finstack.core.dates.daycount import DayCount
from finstack.core.dates.schedule import Frequency, StubKind
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve
from finstack.valuations.cashflow import ScheduleParams
from finstack.valuations.instruments import (
    EquityTotalReturnSwap,
    EquityUnderlying,
    FiIndexTotalReturnSwap,
    IndexUnderlying,
    TrsFinancingLegSpec,
    TrsScheduleSpec,
    TrsSide,
)
from finstack.valuations.pricer import create_standard_registry

from finstack import Money


def build_market(as_of: date) -> MarketContext:
    market = MarketContext()

    discount_curve = DiscountCurve(
        "USD-OIS",
        as_of,
        [
            (0.0, 1.0),
            (0.5, 0.9975),
            (1.0, 0.9950),
            (3.0, 0.9800),
        ],
    )
    sofr_curve = ForwardCurve(
        "USD-SOFR-3M",
        0.25,
        [
            (0.0, 0.045),
            (1.0, 0.047),
            (3.0, 0.049),
        ],
        base_date=as_of,
    )
    market.insert_discount(discount_curve)
    market.insert_forward(sofr_curve)

    market.insert_price("ACME-SPOT", MarketScalar.price(Money(120.0, USD)))
    market.insert_price("ACME-DIVYIELD", MarketScalar.unitless(0.018))

    market.insert_price("IG-INDEX-LEVEL", MarketScalar.price(Money(100.0, USD)))
    market.insert_price("IG-INDEX-YIELD", MarketScalar.unitless(0.035))

    return market


def build_schedule(start: date, end: date) -> TrsScheduleSpec:
    schedule_params = ScheduleParams.new(
        Frequency.QUARTERLY,
        DayCount.ACT_360,
        BusinessDayConvention.MODIFIED_FOLLOWING,
        calendar_id="usny",
        stub=StubKind.NONE,
    )
    return TrsScheduleSpec.new(start, end, schedule_params)


def main() -> None:
    as_of = date(2024, 1, 2)
    market = build_market(as_of)
    registry = create_standard_registry()

    schedule = build_schedule(as_of, date(as_of.year + 1, as_of.month, as_of.day))
    financing = TrsFinancingLegSpec.new(
        discount_curve="USD-OIS",
        forward_curve="USD-SOFR-3M",
        day_count=DayCount.ACT_360,
        spread_bp=50.0,
    )

    equity_underlying = EquityUnderlying.new(
        ticker="ACME",
        spot_id="ACME-SPOT",
        currency=USD,
        div_yield_id="ACME-DIVYIELD",
        contract_size=1.0,
    )
    equity_trs = EquityTotalReturnSwap.create(
        "ACME-TRS",
        Money(5_000_000, USD),
        equity_underlying,
        financing,
        schedule,
        TrsSide.RECEIVE_TOTAL_RETURN,
        initial_level=120.0,
    )
    registry.price_with_metrics(
        equity_trs,
        "discounting",
        market,
        ["index_delta", "financing_annuity"],
    )

    index_underlying = IndexUnderlying.new(
        index_id="CDX.NA.IG",
        base_currency=USD,
        yield_id="IG-INDEX-YIELD",
        duration_id=None,
        convexity_id=None,
        contract_size=1.0,
    )
    index_trs = FiIndexTotalReturnSwap.create(
        "CDX-TRS",
        Money(8_000_000, USD),
        index_underlying,
        financing,
        schedule,
        TrsSide.PAY_TOTAL_RETURN,
        initial_level=100.0,
    )
    registry.price_with_metrics(
        index_trs,
        "discounting",
        market,
        ["par_spread", "dv01"],
    )


if __name__ == "__main__":
    main()
