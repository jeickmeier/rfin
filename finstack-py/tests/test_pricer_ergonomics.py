"""Focused tests for Python pricing ergonomics."""

from datetime import date

from finstack.valuations.pricer import standard_registry
import pytest
from tests.fixtures.strategies import (
    TOLERANCE_DETERMINISTIC,
    create_flat_market_context,
    create_test_bond,
)


def test_price_with_metrics_accepts_documented_argument_order() -> None:
    """The as_of-first call shape should work with keyword metrics."""
    registry = standard_registry()
    market = create_flat_market_context(discount_rate=0.05)
    bond = create_test_bond()
    as_of = date(2024, 1, 1)

    result = registry.price_with_metrics(
        bond,
        "discounting",
        market,
        as_of,
        metrics=["clean_price", "accrued"],
    )

    assert result.instrument_id == "TEST-BOND"
    assert "clean_price" in result.measures
    assert "accrued" in result.measures


def test_price_with_metrics_keeps_legacy_positional_order() -> None:
    """Older positional callers should continue to work unchanged."""
    registry = standard_registry()
    market = create_flat_market_context(discount_rate=0.05)
    bond = create_test_bond()
    as_of = date(2024, 1, 1)
    metric_names = ["clean_price", "accrued"]

    legacy = registry.price_with_metrics(
        bond,
        "discounting",
        market,
        metric_names,
        as_of,
    )
    documented = registry.price_with_metrics(
        bond,
        "discounting",
        market,
        as_of,
        metrics=metric_names,
    )

    assert legacy.instrument_id == documented.instrument_id
    assert legacy.value.amount == pytest.approx(documented.value.amount, abs=TOLERANCE_DETERMINISTIC)
    assert legacy.measures.keys() == documented.measures.keys()
    for metric_name in metric_names:
        assert legacy.measures[metric_name] == pytest.approx(
            documented.measures[metric_name],
            abs=TOLERANCE_DETERMINISTIC,
        )


def test_price_batch_with_metrics_preserves_input_order() -> None:
    """Batch metrics pricing should return results aligned to input order."""
    registry = standard_registry()
    market = create_flat_market_context(discount_rate=0.05)
    instruments = [
        create_test_bond(bond_id="BOND-A", coupon_rate=0.04),
        create_test_bond(bond_id="BOND-B", coupon_rate=0.06),
    ]

    results = registry.price_batch_with_metrics(
        instruments,
        "discounting",
        market,
        date(2024, 1, 1),
        metrics=["clean_price", "accrued"],
    )

    assert [result.instrument_id for result in results] == ["BOND-A", "BOND-B"]
    for result in results:
        assert "clean_price" in result.measures
        assert "accrued" in result.measures
