"""Tests for P&L attribution bindings."""

from datetime import date

from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.attribution import AttributionMethod, attribute_pnl
import pytest

from finstack import Money
from finstack.valuations import Bond


def test_attribution_method_parallel() -> None:
    """Test parallel attribution method creation."""
    method = AttributionMethod.parallel()
    assert method is not None
    assert "Parallel" in str(method)


def test_attribution_method_waterfall() -> None:
    """Test waterfall attribution method with custom order."""
    method = AttributionMethod.waterfall([
        "carry",
        "rates_curves",
        "credit_curves",
        "fx",
    ])
    assert method is not None
    assert "Waterfall" in str(method)


def test_attribution_method_waterfall_invalid_factor() -> None:
    """Test that invalid factor names raise ValueError."""
    with pytest.raises(ValueError, match="Unknown attribution factor"):
        AttributionMethod.waterfall(["invalid_factor"])


def test_attribution_method_metrics_based() -> None:
    """Test metrics-based attribution method creation."""
    method = AttributionMethod.metrics_based()
    assert method is not None
    assert "MetricsBased" in str(method)


def test_bond_attribution_parallel() -> None:
    """Test parallel attribution for a simple bond."""
    # Create bond
    bond = Bond.fixed_semiannual(
        "TEST-BOND",
        Money(1_000_000, "USD"),
        0.05,  # 5% coupon
        date(2025, 1, 1),
        date(2030, 1, 1),
        "USD-OIS",
    )

    # Create discount curve at T₀
    curve_t0 = DiscountCurve("USD-OIS", date(2025, 1, 15), [(0.0, 1.0), (5.0, 0.82)])

    market_t0 = MarketContext()
    market_t0.insert_discount(curve_t0)

    # Create discount curve at T₁ (rates increased)
    curve_t1 = DiscountCurve("USD-OIS", date(2025, 1, 16), [(0.0, 1.0), (5.0, 0.78)])

    market_t1 = MarketContext()
    market_t1.insert_discount(curve_t1)

    # Run attribution
    attr = attribute_pnl(bond, market_t0, market_t1, date(2025, 1, 15), date(2025, 1, 16))

    # Verify structure
    assert attr is not None
    assert attr.total_pnl is not None
    assert attr.carry is not None
    assert attr.rates_curves_pnl is not None
    assert attr.residual is not None

    # Verify currency
    assert attr.total_pnl.currency == "USD"

    # Verify metadata
    assert attr.meta is not None
    assert attr.meta.instrument_id == "TEST-BOND"
    assert attr.meta.num_repricings > 0
    assert "Parallel" in str(attr.meta.method)


def test_bond_attribution_waterfall() -> None:
    """Test waterfall attribution for a bond."""
    # Create bond and markets (reusing from above)
    bond = Bond.fixed_semiannual(
        "TEST-BOND", Money(1_000_000, "USD"), 0.05, date(2025, 1, 1), date(2030, 1, 1), "USD-OIS"
    )

    curve = DiscountCurve("USD-OIS", date(2025, 1, 15), [(0.0, 1.0), (5.0, 0.82)])

    market = MarketContext()
    market.insert_discount(curve)

    # Waterfall attribution
    method = AttributionMethod.waterfall(["carry", "rates_curves", "fx"])

    attr = attribute_pnl(
        bond,
        market,
        market,  # Same market for this test
        date(2025, 1, 15),
        date(2025, 1, 16),
        method=method,
    )

    # Waterfall should have minimal residual
    assert attr.meta.residual_pct < 1.0  # Less than 1%


def test_attribution_exports() -> None:
    """Test CSV and explain exports."""
    bond = Bond.fixed_semiannual(
        "TEST-BOND", Money(1_000_000, "USD"), 0.05, date(2025, 1, 1), date(2030, 1, 1), "USD-OIS"
    )

    curve = DiscountCurve("USD-OIS", date(2025, 1, 15), [(0.0, 1.0), (5.0, 0.82)])

    market = MarketContext()
    market.insert_discount(curve)

    attr = attribute_pnl(bond, market, market, date(2025, 1, 15), date(2025, 1, 16))

    # Test CSV export
    csv = attr.to_csv()
    assert "instrument_id" in csv
    assert "TEST-BOND" in csv
    assert "total" in csv

    # Test explain
    explanation = attr.explain()
    assert "Total P&L" in explanation
    assert "Residual" in explanation


def test_attribution_tolerance_check() -> None:
    """Test residual tolerance checking."""
    bond = Bond.fixed_semiannual(
        "TEST-BOND", Money(1_000_000, "USD"), 0.05, date(2025, 1, 1), date(2030, 1, 1), "USD-OIS"
    )

    curve = DiscountCurve("USD-OIS", date(2025, 1, 15), [(0.0, 1.0), (5.0, 0.82)])

    market = MarketContext()
    market.insert_discount(curve)

    attr = attribute_pnl(bond, market, market, date(2025, 1, 15), date(2025, 1, 16))

    # Test tolerance checking
    # 1% tolerance should pass for most cases
    assert attr.residual_within_tolerance(1.0, 1000.0)


def test_attribution_detail_access() -> None:
    """Test accessing detailed attribution breakdowns."""
    bond = Bond.fixed_semiannual(
        "TEST-BOND", Money(1_000_000, "USD"), 0.05, date(2025, 1, 1), date(2030, 1, 1), "USD-OIS"
    )

    curve = DiscountCurve("USD-OIS", date(2025, 1, 15), [(0.0, 1.0), (5.0, 0.82)])

    market = MarketContext()
    market.insert_discount(curve)

    attr = attribute_pnl(bond, market, market, date(2025, 1, 15), date(2025, 1, 16))

    # Check if rates detail is available
    if attr.rates_detail:
        curve_dict = attr.rates_detail.by_curve_to_dict()
        assert isinstance(curve_dict, dict)

        # Should have discount and forward totals
        assert attr.rates_detail.discount_total is not None
        assert attr.rates_detail.forward_total is not None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
