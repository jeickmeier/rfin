"""Portfolio grouping utilities."""

from __future__ import annotations
from typing import Dict, List, Tuple
from finstack.core.money import Money
from .portfolio import Portfolio
from .valuation import PortfolioValuation
from .types import Position

def group_by_attribute(portfolio: Portfolio, attribute_key: str) -> Dict[str, List[Position]]:
    """Group portfolio positions by an attribute tag.

    Groups all positions in a portfolio by the value of a specified attribute
    tag. Positions without the tag are placed in the special ``"_untagged"`` bucket.
    This is useful
    for organizing positions by sector, rating, currency, or any custom attribute.

    Parameters
    ----------
    portfolio : Portfolio
        Portfolio containing positions to group.
    attribute_key : str
        Tag key to group by (e.g., "sector", "rating", "currency", "strategy").
        Positions without the tag are placed in ``"_untagged"``.

    Returns
    -------
    Dict[str, List[Position]]
        Dictionary mapping attribute values to lists of positions. Keys are
        the tag values, values are lists of positions with that tag value.

    Raises
    ------
    RuntimeError
        If grouping fails (internal error).

    Examples
    --------
    Group by sector:

        >>> from finstack.portfolio import group_by_attribute
        >>> by_sector = group_by_attribute(portfolio, "sector")
        >>> print(by_sector.keys())
        dict_keys(['Technology', 'Healthcare', 'Financials'])
        >>> tech_positions = by_sector["Technology"]
        >>> print(len(tech_positions))
        15

    Group by rating:

        >>> by_rating = group_by_attribute(portfolio, "rating")
        >>> print(by_rating.keys())
        dict_keys(['AAA', 'AA', 'A', 'BBB'])

    Notes
    -----
    - Positions without the tag are placed in ``"_untagged"`` (no error)
    - Empty groups are not included in the result
    - Useful for filtering and organizing positions

    See Also
    --------
    :func:`aggregate_by_attribute`: Aggregate values by attribute
    :class:`Position`: Position structure with tags
    """
    ...

def aggregate_by_attribute(
    valuation: PortfolioValuation,
    portfolio: Portfolio,
    attribute_key: str,
) -> Dict[str, Money]:
    """Aggregate portfolio valuation by an attribute tag.

    Sums position values within each attribute group. This is useful for
    reporting portfolio value by sector, rating, currency, or any custom
    attribute. Values are converted to the portfolio base currency before
    aggregation.

    Parameters
    ----------
    valuation : PortfolioValuation
        Portfolio valuation results from value_portfolio(). Must contain
        valuations for all positions in the portfolio.
    portfolio : Portfolio
        Portfolio containing positions. Used to access position tags.
    attribute_key : str
        Tag key to group by (e.g., "sector", "rating", "currency").
        Positions without the tag are placed in ``"_untagged"``.

    Returns
    -------
    Dict[str, Money]
        Dictionary mapping attribute values to aggregated amounts in the
        portfolio base currency. Keys are tag values, values are summed
        position values.

    Raises
    ------
    RuntimeError
        If aggregation fails (missing valuations, FX conversion errors).

    Examples
    --------
    Aggregate by sector:

        >>> from finstack.portfolio import aggregate_by_attribute, value_portfolio
        >>> valuation = value_portfolio(portfolio, market_ctx)
        >>> by_sector = aggregate_by_attribute(valuation, portfolio, "sector")
        >>> print(f"Technology: {by_sector['Technology']}")
        Technology: Money(5000000.0, Currency("USD"))
        >>> print(f"Healthcare: {by_sector['Healthcare']}")
        Healthcare: Money(3000000.0, Currency("USD"))

    Aggregate by rating:

        >>> by_rating = aggregate_by_attribute(valuation, portfolio, "rating")
        >>> print(f"AAA: {by_rating['AAA']}")
        AAA: Money(2000000.0, Currency("USD"))

    Notes
    -----
    - Positions without the tag are placed in ``"_untagged"`` (no error)
    - Values are converted to portfolio base currency
    - Empty groups are not included in the result
    - Useful for risk reporting and portfolio analysis

    See Also
    --------
    :func:`group_by_attribute`: Group positions by attribute
    :func:`value_portfolio`: Portfolio valuation
    :class:`PortfolioValuation`: Valuation results
    """
    ...

def aggregate_by_book(
    valuation: PortfolioValuation,
    portfolio: Portfolio,
) -> Dict[str, Money]:
    """Aggregate portfolio valuation by book hierarchy.

    Computes total value for each book by summing direct position values plus
    recursively aggregated values from child books.
    """
    ...

def aggregate_by_multiple_attributes(
    valuation: PortfolioValuation,
    portfolio: Portfolio,
    attribute_keys: list[str],
) -> Dict[Tuple[str, ...], Money]:
    """Aggregate portfolio valuation by multiple attributes simultaneously.

    Parameters
    ----------
    valuation : PortfolioValuation
        Portfolio valuation results.
    portfolio : Portfolio
        Portfolio containing positions.
    attribute_keys : list[str]
        Tag keys to group by (e.g., ["sector", "rating"]).

    Returns
    -------
    Dict[tuple[str, ...], Money]
        Dictionary mapping attribute value tuples to aggregated amounts.
    """
    ...
