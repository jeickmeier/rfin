"""Smoke tests for builder-only instrument construction."""

from __future__ import annotations

import datetime as dt

from finstack.core.currency import USD, Currency
from finstack.core.dates.daycount import DayCount
from finstack.valuations.cashflow import CouponType, FixedCouponSpec, ScheduleParams
from finstack.valuations.instruments import (
    AgencyCmo,
    AgencyMbsPassthrough,
    AgencyProgram,
    AgencyTba,
    CmoTranche,
    CmoWaterfall,
    ConversionPolicy,
    ConversionSpec,
    ConvertibleBond,
    DollarRoll,
    Deposit,
    EquityTotalReturnSwap,
    EquityUnderlying,
    FiIndexTotalReturnSwap,
    FxOption,
    FxSpot,
    FxSwap,
    InterestRateSwap,
    IndexUnderlying,
    InflationLinkedBond,
    InflationSwap,
    PoolType,
    TbaTerm,
    TrsFinancingLegSpec,
    TrsScheduleSpec,
    TrsSide,
)
from finstack.core.dates.schedule import Frequency

from finstack import Money


def test_fx_builders_smoke() -> None:
    """Smoke test FX instrument builders (spot/swap/option)."""
    spot = (
        FxSpot
        .builder("EURUSD-SPOT")
        .base_currency("EUR")
        .quote_currency("USD")
        .spot_rate(1.10)
        .notional(Money(1_000_000, Currency("EUR")))
        .settlement(dt.date(2024, 1, 4))
        .build()
    )
    assert spot.instrument_id == "EURUSD-SPOT"
    assert spot.pair_name == "EURUSD"
    assert spot.base_currency.code == "EUR"
    assert spot.quote_currency.code == "USD"
    assert spot.spot_rate == 1.10

    swap = (
        FxSwap
        .builder("EURUSD-SWAP")
        .base_currency("EUR")
        .quote_currency("USD")
        .notional(Money(5_000_000, Currency("EUR")))
        .near_date(dt.date(2024, 1, 4))
        .far_date(dt.date(2024, 7, 4))
        .domestic_discount_curve("USD-OIS")
        .foreign_discount_curve("EUR-OIS")
        .near_rate(1.10)
        .far_rate(1.11)
        .build()
    )
    assert swap.instrument_id == "EURUSD-SWAP"
    assert swap.near_rate == 1.10
    assert swap.far_rate == 1.11

    option = (
        FxOption
        .builder("EURUSD-PUT")
        .base_currency("EUR")
        .quote_currency("USD")
        .strike(1.05)
        .expiry(dt.date(2025, 1, 2))
        .notional(Money(1_000_000, Currency("EUR")))
        .domestic_discount_curve("USD-OIS")
        .foreign_discount_curve("EUR-OIS")
        .vol_surface("EURUSD-VOL")
        .option_type("put")
        .build()
    )
    assert option.instrument_id == "EURUSD-PUT"
    assert option.option_type == "put"
    assert option.exercise_style == "european"


def test_inflation_builders_smoke() -> None:
    """Smoke test inflation instrument builders (ILB and swap)."""
    ilb = (
        InflationLinkedBond
        .builder("TIPS-2030")
        .notional(Money(1_000_000, USD))
        .real_coupon(0.02)
        .issue(dt.date(2024, 1, 1))
        .maturity(dt.date(2030, 1, 1))
        .base_index(300.0)
        .discount_curve("USD-OIS")
        .inflation_curve("US-CPI")
        .build()
    )
    assert ilb.instrument_id == "TIPS-2030"
    assert ilb.real_coupon == 0.02
    assert ilb.discount_curve == "USD-OIS"
    assert ilb.inflation_curve == "US-CPI"

    zciis = (
        InflationSwap
        .builder("ZCIIS-5Y")
        .notional(Money(10_000_000, USD))
        .fixed_rate(0.025)
        .start_date(dt.date(2024, 1, 1))
        .maturity(dt.date(2029, 1, 1))
        .discount_curve("USD-OIS")
        .inflation_curve("US-CPI")
        .side("pay_fixed")
        .build()
    )
    assert zciis.instrument_id == "ZCIIS-5Y"
    assert zciis.fixed_rate == 0.025


def test_builder_aliases_remain_available() -> None:
    """Legacy builder aliases should remain available for compatibility."""
    deposit = (
        Deposit
        .builder("DEP-ALIAS")
        .notional(1_000_000.0)
        .currency("USD")
        .start(dt.date(2024, 1, 1))
        .maturity(dt.date(2024, 4, 1))
        .day_count(DayCount.ACT_360)
        .disc_id("USD-OIS")
        .quote_rate(0.045)
        .build()
    )
    assert deposit.discount_curve == "USD-OIS"

    irs = (
        InterestRateSwap
        .builder("IRS-ALIAS")
        .notional(10_000_000.0)
        .currency("USD")
        .fixed_rate(0.05)
        .frequency(Frequency.SEMI_ANNUAL)
        .maturity(dt.date(2029, 1, 1))
        .disc_id("USD-OIS")
        .fwd_id("USD-SOFR")
        .build()
    )
    assert irs.discount_curve == "USD-OIS"
    assert irs.forward_curve == "USD-SOFR"

    zciis = (
        InflationSwap
        .builder("ZCIIS-ALIAS")
        .notional(Money(10_000_000, USD))
        .fixed_rate(0.025)
        .start_date(dt.date(2024, 1, 1))
        .maturity(dt.date(2029, 1, 1))
        .discount_curve("USD-OIS")
        .inflation_curve("US-CPI")
        .side("pay_fixed")
        .build()
    )
    assert zciis.instrument_id == "ZCIIS-ALIAS"


def test_trs_builders_smoke() -> None:
    """Smoke test total return swap builders (equity + index)."""
    underlying = EquityUnderlying.new(
        ticker="ACME",
        spot_id="ACME-SPOT",
        currency=Currency("USD"),
        div_yield_id="ACME-DIVYIELD",
        contract_size=1.0,
    )
    financing = TrsFinancingLegSpec.new(
        discount_curve="USD-OIS",
        forward_curve="USD-SOFR-3M",
        day_count=DayCount.ACT_360,
        spread_bp=25.0,
    )
    schedule = TrsScheduleSpec.new(
        start=dt.date(2024, 1, 1),
        end=dt.date(2025, 1, 1),
        schedule_params=ScheduleParams.quarterly_act360(),
    )

    trs = (
        EquityTotalReturnSwap
        .builder("TRS-ACME")
        .notional(Money(1_000_000, USD))
        .underlying(underlying)
        .financing(financing)
        .schedule(schedule)
        .side(TrsSide.RECEIVE_TOTAL_RETURN)
        .initial_level(120.0)
        .build()
    )
    assert trs.instrument_id == "TRS-ACME"
    assert trs.side == "receive_total_return"

    idx_underlying = IndexUnderlying.new(
        index_id="CDX.NA.IG",
        base_currency=Currency("USD"),
        yield_id="IG-YIELD",
        duration_id=None,
        convexity_id=None,
        contract_size=1.0,
    )
    idx_trs = (
        FiIndexTotalReturnSwap
        .builder("TRS-INDEX")
        .notional(Money(2_000_000, USD))
        .underlying(idx_underlying)
        .financing(financing)
        .schedule(schedule)
        .side(TrsSide.PAY_TOTAL_RETURN)
        .build()
    )
    assert idx_trs.instrument_id == "TRS-INDEX"


def test_convertible_builder_smoke() -> None:
    """Smoke test convertible bond builder."""
    schedule = ScheduleParams.semiannual_30360()
    fixed_coupon = FixedCouponSpec.new(0.035, schedule, CouponType.CASH)
    conversion = ConversionSpec(ConversionPolicy.voluntary(), ratio=20.0)

    cb = (
        ConvertibleBond
        .builder("ACME-CB")
        .notional(Money(1_000_000, USD))
        .issue(dt.date(2024, 1, 1))
        .maturity(dt.date(2029, 1, 1))
        .discount_curve("USD-OIS")
        .conversion(conversion)
        .underlying_equity_id("EQUITY-SPOT")
        .call_schedule([(dt.date(2027, 1, 1), 102.5)])
        .fixed_coupon(fixed_coupon)
        .build()
    )
    assert cb.instrument_id == "ACME-CB"
    assert cb.discount_curve == "USD-OIS"
    assert cb.conversion_ratio == 20.0


def test_agency_mbs_builders_smoke() -> None:
    """Smoke test agency MBS builders (pass-through/TBA/dollar roll/CMO)."""
    mbs = (
        AgencyMbsPassthrough
        .builder("FN-MA1234")
        .pool_id("MA1234")
        .agency(AgencyProgram.Fnma)
        .original_face(1_000_000.0)
        .current_face(950_000.0)
        .currency("USD")
        .wac(0.045)
        .pass_through_rate(0.04)
        .wam(348)
        .issue_date(dt.date(2022, 1, 1))
        .maturity_date(dt.date(2052, 1, 1))
        .discount_curve_id("USD-OIS")
        .pool_type(PoolType.Generic)
        .day_count(DayCount.THIRTY_360)
        .build()
    )
    assert mbs.instrument_id == "FN-MA1234"

    tba = (
        AgencyTba
        .builder("FN30-4.0-202403")
        .agency(AgencyProgram.Fnma)
        .coupon(0.04)
        .term(TbaTerm.ThirtyYear)
        .settlement_year(2024)
        .settlement_month(3)
        .notional(10_000_000.0)
        .currency("USD")
        .trade_price(98.5)
        .discount_curve_id("USD-OIS")
        .build()
    )
    assert tba.instrument_id == "FN30-4.0-202403"

    roll = (
        DollarRoll
        .builder("ROLL-0324-0424")
        .agency(AgencyProgram.Fnma)
        .coupon(0.04)
        .term(TbaTerm.ThirtyYear)
        .notional(10_000_000.0)
        .currency("USD")
        .front_settlement_year(2024)
        .front_settlement_month(3)
        .back_settlement_year(2024)
        .back_settlement_month(4)
        .front_price(98.5)
        .back_price(98.0)
        .discount_curve_id("USD-OIS")
        .build()
    )
    assert roll.instrument_id == "ROLL-0324-0424"

    tranches = [
        CmoTranche.sequential("A", 40_000_000.0, "USD", 0.04, 1),
        CmoTranche.sequential("B", 30_000_000.0, "USD", 0.045, 2),
    ]
    waterfall = CmoWaterfall(tranches)
    cmo = (
        AgencyCmo
        .builder("FNR-2024-1-A")
        .deal_name("FNR 2024-1")
        .agency(AgencyProgram.Fnma)
        .issue_date(dt.date(2024, 1, 1))
        .waterfall(waterfall)
        .reference_tranche_id("A")
        .discount_curve_id("USD-OIS")
        .build()
    )
    assert cmo.instrument_id == "FNR-2024-1-A"
