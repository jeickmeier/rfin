"""Tests for covenant forecasting functionality."""

from finstack.statements_analytics import analysis

from finstack.valuations import covenants


def test_covenant_types_available() -> None:
    """Test that covenant type bindings are available in valuations."""
    assert hasattr(covenants, "CovenantType")
    assert hasattr(covenants, "Covenant")
    assert hasattr(covenants, "CovenantSpec")
    assert hasattr(covenants, "FutureBreach")


def test_forecast_functions_available() -> None:
    """Test that forecast functions are available in statements.analysis."""
    assert hasattr(analysis, "forecast_breaches")
    assert hasattr(analysis, "forecast_covenant")
    assert hasattr(analysis, "forecast_covenants")
