"""Shared test fixtures and strategies for cross-language parity testing."""

from .strategies import (
    TOLERANCE_DETERMINISTIC,
    TOLERANCE_FLOATING_POINT,
    TOLERANCE_MONTE_CARLO,
    bond_strategy,
    deposit_strategy,
    discount_curve_strategy,
    forward_curve_strategy,
    major_currencies,
    market_context_strategy,
    positive_amounts,
    positive_notionals,
    positive_rates,
    seeds,
    swap_strategy,
    tenors_in_years,
)

__all__ = [
    "TOLERANCE_DETERMINISTIC",
    "TOLERANCE_FLOATING_POINT",
    "TOLERANCE_MONTE_CARLO",
    "bond_strategy",
    "deposit_strategy",
    "discount_curve_strategy",
    "forward_curve_strategy",
    "major_currencies",
    "market_context_strategy",
    "positive_amounts",
    "positive_notionals",
    "positive_rates",
    "seeds",
    "swap_strategy",
    "tenors_in_years",
]
