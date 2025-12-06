"""Portfolio class."""

from typing import Dict, List, Any, Optional
from datetime import date
from finstack.core.currency import Currency
from .types import Entity, Position

class Portfolio:
    """A portfolio of positions across multiple entities with aggregation support.

    Portfolio represents a collection of financial instrument positions organized
    by entities. It supports multi-currency positions, attribute-based grouping,
    and aggregation for reporting and risk analysis.

    Portfolios are the primary structure for portfolio-level valuation, risk
    aggregation, and scenario analysis. Positions reference entities and
    instruments, enabling flexible grouping and reporting.

    Examples
    --------
    Create and query a portfolio:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.portfolio import (
        ...     PortfolioBuilder,
        ...     Portfolio,
        ...     Entity,
        ...     Position,
        ...     PositionUnit,
        ... )
        >>> from finstack.valuations.instruments import Equity
        >>> entity = Entity("ACME")
        >>> equity = Equity.create("EQ-ACME", ticker="ACME", currency=Currency("USD"), price=120.0)
        >>> position = Position("POS-1", entity.id, equity.instrument_id, equity, 100.0, PositionUnit.UNITS)
        >>> portfolio = (
        ...     PortfolioBuilder("FUND_A")
        ...     .base_ccy(Currency("USD"))
        ...     .as_of(date(2025, 1, 1))
        ...     .entity(entity)
        ...     .position(position)
        ...     .build()
        ... )
        >>> portfolio.validate()
        >>> len(portfolio.positions_for_entity("ACME")), portfolio.get_position("POS-1").instrument_id
        (1, 'EQ-ACME')

    Notes
    -----
    - Positions must reference valid entities
    - Base currency is used for aggregation and reporting
    - Positions can have tags for attribute-based grouping
    - Portfolio supports metadata for custom attributes

    See Also
    --------
    :class:`PortfolioBuilder`: Fluent builder for portfolios
    :class:`Entity`: Entity structure
    :class:`Position`: Position structure
    :func:`value_portfolio`: Value a portfolio
    """

    def __init__(self, id: str, base_ccy: Currency, as_of: date) -> None:
        """Create a new empty portfolio.

        Args:
            id: Unique portfolio identifier.
            base_ccy: Reporting currency.
            as_of: Valuation date.

        Returns:
            Portfolio: New portfolio instance.

        Examples:
            >>> from finstack.core import Currency
            >>> from datetime import date
            >>> portfolio = Portfolio("FUND_A", Currency.USD, date(2024, 1, 1))
            >>> portfolio.id
            'FUND_A'
        """
        ...

    def get_position(self, position_id: str) -> Optional[Position]:
        """Get a position by identifier.

        Args:
            position_id: Identifier of the position to locate.

        Returns:
            Position or None: The position if found.

        Examples
        --------
            >>> from datetime import date
            >>> from finstack.core.currency import Currency
            >>> from finstack.portfolio import PortfolioBuilder, Entity, Position, PositionUnit
            >>> from finstack.valuations.instruments import Equity
            >>> entity = Entity("ACME")
            >>> equity = Equity.create("EQ-ACME", ticker="ACME", currency=Currency("USD"), price=120.0)
            >>> position = Position("POS-1", entity.id, equity.instrument_id, equity, 100.0, PositionUnit.UNITS)
            >>> portfolio = (
            ...     PortfolioBuilder("FUND_A")
            ...     .base_ccy(Currency("USD"))
            ...     .as_of(date(2025, 1, 1))
            ...     .entity(entity)
            ...     .position(position)
            ...     .build()
            ... )
            >>> portfolio.get_position("POS-1").instrument_id
            'EQ-ACME'
        """
        ...

    def positions_for_entity(self, entity_id: str) -> List[Position]:
        """Get all positions for a given entity.

        Args:
            entity_id: Entity identifier used for filtering.

        Returns:
            list[Position]: List of positions for the entity.

        Examples
        --------
            >>> from datetime import date
            >>> from finstack.core.currency import Currency
            >>> from finstack.portfolio import PortfolioBuilder, Entity, Position, PositionUnit
            >>> from finstack.valuations.instruments import Equity
            >>> entity = Entity("ACME")
            >>> equity = Equity.create("EQ-ACME", ticker="ACME", currency=Currency("USD"), price=120.0)
            >>> position = Position("POS-1", entity.id, equity.instrument_id, equity, 100.0, PositionUnit.UNITS)
            >>> portfolio = (
            ...     PortfolioBuilder("FUND_A")
            ...     .base_ccy(Currency("USD"))
            ...     .as_of(date(2025, 1, 1))
            ...     .entity(entity)
            ...     .position(position)
            ...     .build()
            ... )
            >>> len(portfolio.positions_for_entity("ACME"))
            1
        """
        ...

    def positions_with_tag(self, key: str, value: str) -> List[Position]:
        """Get all positions with a specific tag value.

        Args:
            key: Tag key to filter by.
            value: Tag value to match.

        Returns:
            list[Position]: List of positions with matching tag.
        """
        ...

    def validate(self) -> None:
        """Validate the portfolio structure and references.

        Checks that all positions reference valid entities and that structural
        invariants are maintained.

        Raises:
            ValueError: If validation fails.

        Examples
        --------
            >>> from datetime import date
            >>> from finstack.core.currency import Currency
            >>> from finstack.portfolio import PortfolioBuilder, Entity, Position, PositionUnit
            >>> from finstack.valuations.instruments import Equity
            >>> entity = Entity("ACME")
            >>> equity = Equity.create("EQ-ACME", ticker="ACME", currency=Currency("USD"), price=120.0)
            >>> position = Position("POS-1", entity.id, equity.instrument_id, equity, 100.0, PositionUnit.UNITS)
            >>> portfolio = (
            ...     PortfolioBuilder("FUND_A")
            ...     .base_ccy(Currency("USD"))
            ...     .as_of(date(2025, 1, 1))
            ...     .entity(entity)
            ...     .position(position)
            ...     .build()
            ... )
            >>> portfolio.validate()
        """
        ...

    def has_dummy_entity(self) -> bool:
        """Return True if the portfolio contains the dummy entity (_standalone)."""
        ...

    @property
    def id(self) -> str:
        """Get the portfolio identifier."""
        ...

    @property
    def name(self) -> Optional[str]:
        """Get the portfolio name."""
        ...

    @name.setter
    def name(self, value: Optional[str]) -> None:
        """Set the portfolio name."""
        ...

    @property
    def base_ccy(self) -> Currency:
        """Get the base currency."""
        ...

    @property
    def as_of(self) -> date:
        """Get the valuation date."""
        ...

    @property
    def entities(self) -> Dict[str, Entity]:
        """Get the portfolio entities."""
        ...

    @property
    def positions(self) -> List[Position]:
        """Get the portfolio positions."""
        ...

    @property
    def tags(self) -> Dict[str, str]:
        """Get portfolio tags."""
        ...

    @property
    def meta(self) -> Dict[str, Any]:
        """Get portfolio metadata."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
