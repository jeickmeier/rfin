"""Shared pytest configuration and fixtures for finstack tests."""

from datetime import date

from finstack.core.market_data import MarketContext
import pytest
from tests.fixtures.strategies import (
    TOLERANCE_DETERMINISTIC,
    TOLERANCE_FLOATING_POINT,
    TOLERANCE_MONTE_CARLO,
    create_flat_market_context,
)


def pytest_configure(config: pytest.Config) -> None:
    """Register custom markers."""
    config.addinivalue_line("markers", "parity: cross-language parity tests")
    config.addinivalue_line("markers", "bulk: bulk pricing tests")
    config.addinivalue_line("markers", "seeded: seeded stochastic tests")
    config.addinivalue_line("markers", "properties: property-based tests")


@pytest.fixture
def base_date() -> date:
    """Standard base date for tests."""
    return date(2024, 1, 1)


@pytest.fixture
def tolerance_deterministic() -> float:
    """Tolerance for deterministic operations (1e-10)."""
    return TOLERANCE_DETERMINISTIC


@pytest.fixture
def tolerance_floating_point() -> float:
    """Tolerance for floating-point operations (1e-8)."""
    return TOLERANCE_FLOATING_POINT


@pytest.fixture
def tolerance_monte_carlo() -> float:
    """Tolerance for Monte Carlo simulations (1e-6)."""
    return TOLERANCE_MONTE_CARLO


@pytest.fixture
def standard_market(base_date: date) -> MarketContext:
    """Standard market context with 5% flat curves."""
    return create_flat_market_context(
        discount_rate=0.05,
        forward_rate=0.05,
        base_date=base_date,
    )
