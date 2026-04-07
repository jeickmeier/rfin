"""Focused tests for Python pricing ergonomics."""

from datetime import date

from finstack.valuations.metrics import MetricId
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


def test_valuation_result_get_metric_accepts_metric_id_and_string() -> None:
    """ValuationResult should support typed and string metric lookups."""
    registry = standard_registry()
    market = create_flat_market_context(discount_rate=0.05)
    bond = create_test_bond()

    result = registry.price_with_metrics(
        bond,
        "discounting",
        market,
        date(2024, 1, 1),
        metrics=["clean_price", "accrued"],
    )

    clean_price = result.get_metric("clean_price")
    typed_clean_price = result.get_metric(MetricId.from_name("clean_price"))

    assert clean_price is not None
    assert typed_clean_price == pytest.approx(clean_price, abs=TOLERANCE_DETERMINISTIC)
    assert result.get_metric("dv01") is None
    assert [metric.name for metric in result.available_metrics()] == ["clean_price", "accrued"]


def test_price_batch_with_metrics_error_includes_instrument_context() -> None:
    """Batch pricing failures should identify the failing instrument."""
    registry = standard_registry()
    market = create_flat_market_context(discount_rate=0.05)
    instruments = [
        create_test_bond(bond_id="BOND-OK"),
        create_test_bond(bond_id="BOND-MISSING", disc_id="USD-MISSING"),
    ]

    with pytest.raises(RuntimeError, match=r"BOND-MISSING") as exc_info:
        registry.price_batch_with_metrics(
            instruments,
            "discounting",
            market,
            date(2024, 1, 1),
            metrics=["clean_price"],
        )

    assert "USD-MISSING" in str(exc_info.value)
    assert "model=Discounting" in str(exc_info.value)


def test_standard_registry_returns_usable_registry() -> None:
    """standard_registry() should expose the standard registry surface."""
    market = create_flat_market_context(discount_rate=0.05)
    bond = create_test_bond(bond_id="ALIAS-BOND")

    registry = standard_registry()
    result = registry.price(bond, "discounting", market, date(2024, 1, 1))

    assert result.instrument_id == "ALIAS-BOND"
    assert result.value.amount != 0.0
