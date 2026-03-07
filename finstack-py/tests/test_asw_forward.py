"""Tests for forward-based asset swap helpers exposed via PyPricerRegistry.asw_forward."""

from __future__ import annotations

import datetime as dt
import math

from finstack.core.currency import Currency
from finstack.core.dates import DayCount
from finstack.core.market_data import DiscountCurve, ForwardCurve, MarketContext
from finstack.core.money import Money
from finstack.valuations.instruments import Bond
from finstack.valuations.pricer import create_standard_registry
import pytest


def _build_market_and_bond() -> tuple[Bond, MarketContext]:
    """Helper to construct a simple fixed-rate bond and matching market context."""
    as_of = dt.date(2025, 1, 1)

    disc = DiscountCurve(
        "USD-OIS",
        as_of,
        [(0.0, 1.0), (5.0, 0.80)],
        day_count=DayCount.ACT_365F,
    )
    fwd = ForwardCurve(
        "USD-SOFR-3M",
        0.25,
        [(0.0, 0.03), (5.0, 0.035)],
        base_date=as_of,
        day_count=DayCount.ACT_360,
    )

    market = MarketContext()
    market.insert(disc)
    market.insert(fwd)

    bond = (
        Bond
        .builder("ASW-PY")
        .money(Money(100.0, Currency("USD")))
        .coupon_rate(0.05)
        .issue(as_of)
        .maturity(dt.date(2030, 1, 1))
        .disc_id("USD-OIS")
        .build()
    )

    return bond, market


class TestAswForward:
    """Python-level tests for the forward-based ASW helper."""

    def test_missing_dirty_price_raises_value_error(self) -> None:
        """Calling asw_forward without dirty_price_ccy must not silently assume par."""
        bond, market = _build_market_and_bond()
        registry = create_standard_registry()

        with pytest.raises(ValueError, match=r"dirty_price_ccy"):
            registry.asw_forward(
                bond,
                market,
                "USD-SOFR-3M",
                25.0,
            )

    def test_asw_forward_returns_finite_spreads_with_dirty_price(self) -> None:
        """With an explicit dirty price, par and market ASW spreads should be finite."""
        bond, market = _build_market_and_bond()
        registry = create_standard_registry()

        # Market dirty price slightly above par (e.g. 101.25%)
        dirty_price_ccy = 1.0125 * bond.notional.amount

        par, mkt = registry.asw_forward(
            bond,
            market,
            "USD-SOFR-3M",
            25.0,
            dirty_price_ccy,
        )

        assert math.isfinite(par)
        assert math.isfinite(mkt)
