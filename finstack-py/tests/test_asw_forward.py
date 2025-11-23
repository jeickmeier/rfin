"""Tests for forward-based asset swap helpers exposed via PyPricerRegistry.asw_forward."""

from __future__ import annotations

import datetime as dt
import math

import pytest

from finstack.core.currency import Currency
from finstack.core.dates import DayCount
from finstack.core.market_data import DiscountCurve, ForwardCurve, MarketContext
from finstack.core.money import Money
from finstack.valuations.instruments import Bond
from finstack.valuations.pricer import create_standard_registry


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
    market.insert_discount(disc)
    market.insert_forward(fwd)

    bond = Bond.fixed_semiannual(
        "ASW-PY",
        Money(100.0, Currency("USD")),
        0.05,
        as_of,
        dt.date(2030, 1, 1),
        "USD-OIS",
    )

    return bond, market


class TestAswForward:
    """Python-level tests for the forward-based ASW helper."""

    def test_missing_dirty_price_raises_key_error(self) -> None:
        """Calling asw_forward without dirty_price_ccy must not assume par."""
        bond, market = _build_market_and_bond()
        registry = create_standard_registry()

        with pytest.raises((TypeError, KeyError), match="dirty_price_ccy|missing.*required"):
            # dirty_price_ccy is now required, so omitting it should raise TypeError
            # or if validation happens in Rust, it might raise KeyError
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
