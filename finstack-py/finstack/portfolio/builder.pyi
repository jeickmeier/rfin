"""Portfolio builder."""

from typing import Any, Union, List
from .types import Entity, Position
from .portfolio import Portfolio

class PortfolioBuilder:
    """Builder for constructing a Portfolio with validation.

    The builder stores all intermediate values needed to construct a portfolio and checks
    invariants such as base currency, valuation date, and entity references before the
    final portfolio is produced.

    Examples:
        >>> from finstack.portfolio import PortfolioBuilder, Entity
        >>> from finstack.core import Currency
        >>> from datetime import date
        >>> portfolio = (
        ...     PortfolioBuilder("FUND_A")
        ...     .name("Alpha Fund")
        ...     .base_ccy(Currency.USD)
        ...     .as_of(date(2024, 1, 1))
        ...     .entity(Entity("ACME"))
        ...     .build()
        ... )
    """

    def __init__(self, id: str) -> None:
        """Create a new portfolio builder with the given identifier.

        Args:
            id: Unique identifier for the portfolio.

        Returns:
            PortfolioBuilder: New builder instance.

        Examples:
            >>> builder = PortfolioBuilder("FUND_A")
        """
        ...

    def name(self, name: str) -> "PortfolioBuilder":
        """Set the portfolio's human-readable name.

        Args:
            name: Display name stored alongside the portfolio identifier.

        Returns:
            PortfolioBuilder: Self for chaining.

        Examples:
            >>> builder = PortfolioBuilder("FUND_A").name("Alpha Fund")
        """
        ...

    def base_ccy(self, ccy: Any) -> "PortfolioBuilder":
        """Declare the portfolio's reporting currency.

        Args:
            ccy: Currency to use when consolidating values and metrics.

        Returns:
            PortfolioBuilder: Self for chaining.

        Examples:
            >>> from finstack.core import Currency
            >>> builder = PortfolioBuilder("FUND_A").base_ccy(Currency.USD)
        """
        ...

    def as_of(self, date: Any) -> "PortfolioBuilder":
        """Assign the valuation date used for pricing and analytics.

        Args:
            date: The as-of date for valuation and risk calculation.

        Returns:
            PortfolioBuilder: Self for chaining.

        Examples:
            >>> from datetime import date
            >>> builder = PortfolioBuilder("FUND_A").as_of(date(2024, 1, 1))
        """
        ...

    def entity(self, entity_or_entities: Union[Entity, List[Entity]]) -> "PortfolioBuilder":
        """Register entity or entities with the builder.

        Accepts either a single Entity or a list of entities.

        Args:
            entity_or_entities: Entity or list of entities to register.

        Returns:
            PortfolioBuilder: Self for chaining.

        Examples:
            >>> entity = Entity("ACME")
            >>> builder = PortfolioBuilder("FUND_A").entity(entity)
            >>> # Or with multiple entities:
            >>> builder = PortfolioBuilder("FUND_A").entity([entity1, entity2])
        """
        ...

    def position(self, position_or_positions: Union[Position, List[Position]]) -> "PortfolioBuilder":
        """Add position or positions to the portfolio.

        Accepts either a single Position or a list of positions.

        Args:
            position_or_positions: Position or list of positions to add.

        Returns:
            PortfolioBuilder: Self for chaining.

        Examples:
            >>> position = Position("POS_1", "ENTITY_A", "INSTR_1", instrument, 1.0, PositionUnit.UNITS)
            >>> builder = PortfolioBuilder("FUND_A").position(position)
            >>> # Or with multiple positions:
            >>> builder = PortfolioBuilder("FUND_A").position([pos1, pos2])
        """
        ...

    def tag(self, key: str, value: str) -> "PortfolioBuilder":
        """Add a portfolio-level tag.

        Args:
            key: Tag key.
            value: Tag value.

        Returns:
            PortfolioBuilder: Self for chaining.

        Examples:
            >>> builder = PortfolioBuilder("FUND_A").tag("strategy", "long_only")
        """
        ...

    def meta(self, key: str, value: Any) -> "PortfolioBuilder":
        """Add portfolio-level metadata.

        Args:
            key: Metadata key.
            value: Metadata value (must be JSON-serializable).

        Returns:
            PortfolioBuilder: Self for chaining.

        Examples:
            >>> builder = PortfolioBuilder("FUND_A").meta("inception", "2020-01-01")
        """
        ...

    def build(self) -> Portfolio:
        """Build and validate the portfolio.

        Returns:
            Portfolio: Validated portfolio instance.

        Raises:
            ValueError: If validation fails (missing base_ccy, as_of, or invalid references).

        Examples:
            >>> portfolio = builder.build()
        """
        ...

    def __repr__(self) -> str: ...
