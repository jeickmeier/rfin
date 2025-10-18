"""Portfolio class."""

from typing import Dict, List, Any, Optional
from datetime import date
from ...core.currency import Currency
from .types import Entity, Position

class Portfolio:
    """A portfolio of positions across multiple entities.

    The portfolio holds a flat list of positions, each referencing an entity and instrument.
    Positions can be grouped and aggregated by entity or by arbitrary attributes (tags).

    Examples:
        >>> from finstack.portfolio import Portfolio, Entity
        >>> from finstack.core import Currency
        >>> from datetime import date
        >>> portfolio = Portfolio("FUND_A", Currency.USD, date(2024, 1, 1))
        >>> portfolio.entities["ACME"] = Entity("ACME")
        >>> len(portfolio.positions)
        0
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

        Examples:
            >>> position = portfolio.get_position("POS_1")
        """
        ...

    def positions_for_entity(self, entity_id: str) -> List[Position]:
        """Get all positions for a given entity.

        Args:
            entity_id: Entity identifier used for filtering.

        Returns:
            list[Position]: List of positions for the entity.

        Examples:
            >>> positions = portfolio.positions_for_entity("ENTITY_A")
            >>> len(positions)
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

        Examples:
            >>> positions = portfolio.positions_with_tag("sector", "Technology")
        """
        ...

    def validate(self) -> None:
        """Validate the portfolio structure and references.

        Checks that all positions reference valid entities and that structural
        invariants are maintained.

        Raises:
            ValueError: If validation fails.

        Examples:
            >>> portfolio.validate()
        """
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
