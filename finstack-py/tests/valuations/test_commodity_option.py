"""Tests for CommodityOption instrument."""

import datetime as dt

from finstack.valuations.instruments import CommodityOption


def test_commodity_option_create() -> None:
    """Test creating a commodity option."""
    option = (
        CommodityOption.builder("WTI-CALL-75-2025M06")
        .commodity_type("Energy")
        .ticker("CL")
        .strike(75.0)
        .option_type("call")
        .exercise_style("european")
        .expiry(dt.date(2025, 6, 15))
        .quantity(1000.0)
        .unit("BBL")
        .currency("USD")
        .forward_curve_id("WTI-FORWARD")
        .discount_curve_id("USD-OIS")
        .vol_surface_id("WTI-VOL")
        .build()
    )

    assert option.instrument_id == "WTI-CALL-75-2025M06"
    assert option.commodity_type == "Energy"
    assert option.ticker == "CL"
    assert option.strike == 75.0
    assert option.option_type == "call"
    assert option.exercise_style == "european"
    assert option.quantity == 1000.0
    assert option.unit == "BBL"
    assert option.multiplier == 1.0
    assert option.settlement_type == "cash"
    assert option.currency.code == "USD"
    assert option.forward_curve_id == "WTI-FORWARD"
    assert option.discount_curve_id == "USD-OIS"
    assert option.vol_surface_id == "WTI-VOL"


def test_commodity_option_american() -> None:
    """Test creating an American commodity option."""
    option = (
        CommodityOption.builder("WTI-PUT-70-2025M06")
        .commodity_type("Energy")
        .ticker("CL")
        .strike(70.0)
        .option_type("put")
        .exercise_style("american")
        .expiry(dt.date(2025, 6, 15))
        .quantity(500.0)
        .unit("BBL")
        .currency("USD")
        .forward_curve_id("WTI-FORWARD")
        .discount_curve_id("USD-OIS")
        .vol_surface_id("WTI-VOL")
        .tree_steps(201)
        .build()
    )

    assert option.exercise_style == "american"
    assert option.option_type == "put"
    assert option.strike == 70.0


def test_commodity_option_with_overrides() -> None:
    """Test commodity option with pricing overrides."""
    option = (
        CommodityOption.builder("GC-CALL-2000-2025M12")
        .commodity_type("Metal")
        .ticker("GC")
        .strike(2000.0)
        .option_type("call")
        .exercise_style("european")
        .expiry(dt.date(2025, 12, 15))
        .quantity(100.0)
        .unit("OZ")
        .currency("USD")
        .forward_curve_id("GOLD-FORWARD")
        .discount_curve_id("USD-OIS")
        .vol_surface_id("GOLD-VOL")
        .multiplier(100.0)
        .implied_volatility(0.15)
        .quoted_forward(1950.0)
        .spot_id("GOLD-SPOT")
        .build()
    )

    assert option.commodity_type == "Metal"
    assert option.ticker == "GC"
    assert option.multiplier == 100.0
    assert option.quoted_forward == 1950.0
    assert option.spot_id == "GOLD-SPOT"


def test_commodity_option_repr() -> None:
    """Test commodity option repr."""
    option = (
        CommodityOption.builder("WTI-CALL-75-2025M06")
        .commodity_type("Energy")
        .ticker("CL")
        .strike(75.0)
        .option_type("call")
        .exercise_style("european")
        .expiry(dt.date(2025, 6, 15))
        .quantity(1000.0)
        .unit("BBL")
        .currency("USD")
        .forward_curve_id("WTI-FORWARD")
        .discount_curve_id("USD-OIS")
        .vol_surface_id("WTI-VOL")
        .build()
    )

    repr_str = repr(option)
    assert "WTI-CALL-75-2025M06" in repr_str
    assert "CL" in repr_str
    assert "call" in repr_str
    assert "european" in repr_str
