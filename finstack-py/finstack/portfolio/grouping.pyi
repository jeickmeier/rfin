"""Portfolio grouping utilities."""

from typing import Dict, List
from ...core.money import Money
from .portfolio import Portfolio
from .valuation import PortfolioValuation
from .types import Position

def group_by_attribute(portfolio: Portfolio, attribute_key: str) -> Dict[str, List[Position]]:
    """Group portfolio positions by an attribute.

    Returns a dictionary mapping attribute values to lists of positions.
    The attribute key must exist in position tags for positions to be included.

    Args:
        portfolio: Portfolio to group.
        attribute_key: Tag key to group by (e.g., "sector", "rating").

    Returns:
        dict[str, list[Position]]: Mapping of attribute values to position lists.

    Raises:
        RuntimeError: If grouping fails.

    Examples:
        >>> from finstack.portfolio import group_by_attribute
        >>> by_sector = group_by_attribute(portfolio, "sector")
        >>> by_sector["Technology"]
        [Position(...), Position(...)]
    """
    ...

def aggregate_by_attribute(
    valuation: PortfolioValuation,
    portfolio: Portfolio,
    attribute_key: str,
) -> Dict[str, Money]:
    """Aggregate portfolio valuation by an attribute.

    Sums position values within each attribute group. Only positions with the
    specified attribute key in their tags are included. Values are converted
    to the portfolio base currency before aggregation.

    Args:
        valuation: Portfolio valuation results.
        portfolio: Portfolio containing positions.
        attribute_key: Tag key to group by (e.g., "sector", "rating").

    Returns:
        dict[str, Money]: Mapping of attribute values to aggregated amounts.

    Raises:
        RuntimeError: If aggregation fails.

    Examples:
        >>> from finstack.portfolio import aggregate_by_attribute
        >>> by_sector = aggregate_by_attribute(valuation, portfolio, "sector")
        >>> by_sector["Technology"]
        Money(USD, 5000000.0)
    """
    ...
