"""Focused tests for the high-level price_portfolio helper."""

from datetime import date

from finstack.valuations.pricer import standard_registry
from tests.fixtures.strategies import create_flat_market_context, create_test_bond

from finstack.valuations import price_portfolio


def test_price_portfolio_returns_valuation_results() -> None:
    """The helper should return ordered ValuationResult objects by default."""
    registry = standard_registry()
    market = create_flat_market_context(discount_rate=0.05)
    instruments = [
        create_test_bond(bond_id="PORT-BOND-A", coupon_rate=0.04),
        create_test_bond(bond_id="PORT-BOND-B", coupon_rate=0.06),
    ]

    results = price_portfolio(
        instruments,
        market,
        date(2024, 1, 1),
        metrics=["clean_price", "accrued"],
        registry=registry,
    )

    assert [result.instrument_id for result in results] == ["PORT-BOND-A", "PORT-BOND-B"]
    for result in results:
        assert "clean_price" in result.measures
        assert "accrued" in result.measures


def test_price_portfolio_can_return_json_ready_dicts() -> None:
    """The helper should support dict output for service layers."""
    market = create_flat_market_context(discount_rate=0.05)
    instruments = [
        create_test_bond(bond_id="PORT-DICT-A", coupon_rate=0.04),
        create_test_bond(bond_id="PORT-DICT-B", coupon_rate=0.06),
    ]

    results = price_portfolio(
        instruments,
        market,
        date(2024, 1, 1),
        metrics=["clean_price"],
        return_dicts=True,
    )

    assert [result["instrument_id"] for result in results] == ["PORT-DICT-A", "PORT-DICT-B"]
    for result in results:
        assert "value" in result
        assert "measures" in result
        assert "clean_price" in result["measures"]


def test_price_portfolio_defaults_to_discounting_without_metrics() -> None:
    """Omitting model and metrics should still produce valuation results."""
    market = create_flat_market_context(discount_rate=0.05)
    instruments = [create_test_bond(bond_id="PORT-DEFAULTS")]

    results = price_portfolio(instruments, market, date(2024, 1, 1))

    assert len(results) == 1
    assert results[0].instrument_id == "PORT-DEFAULTS"
    assert results[0].measures == {}
