"""Focused tests for Python pricing ergonomics."""

from datetime import date

from finstack.valuations.metrics import MetricId
from finstack.valuations.pricer import get_standard_registry, standard_registry
import pytest
from tests.fixtures.strategies import (
    TOLERANCE_DETERMINISTIC,
    create_flat_market_context,
    create_test_bond,
)

from finstack.valuations import get_standard_registry as top_level_get_standard_registry


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


def test_get_standard_registry_aliases_are_usable() -> None:
    """Singleton-style aliases should expose the standard registry surface."""
    market = create_flat_market_context(discount_rate=0.05)
    bond = create_test_bond(bond_id="ALIAS-BOND")

    from_pricer_module = get_standard_registry()
    from_top_level = top_level_get_standard_registry()
    from_standard_name = standard_registry()

    baseline = from_standard_name.price(bond, "discounting", market, date(2024, 1, 1))
    alias_result = from_pricer_module.price(bond, "discounting", market, date(2024, 1, 1))
    top_level_result = from_top_level.price(bond, "discounting", market, date(2024, 1, 1))

    assert alias_result.instrument_id == baseline.instrument_id
    assert alias_result.value.amount == pytest.approx(baseline.value.amount, abs=TOLERANCE_DETERMINISTIC)
    assert top_level_result.value.amount == pytest.approx(
        baseline.value.amount,
        abs=TOLERANCE_DETERMINISTIC,
    )
