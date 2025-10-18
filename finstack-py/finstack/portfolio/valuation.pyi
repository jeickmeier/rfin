"""Portfolio valuation."""

from typing import Optional, Dict, Any
from ...core.money import Money
from ...core.market_data.context import MarketContext
from ...core.config import FinstackConfig
from .portfolio import Portfolio

class PositionValue:
    """Result of valuing a single position.

    Holds both native-currency and base-currency valuations.

    Examples:
        >>> position_value.position_id
        'POS_1'
        >>> position_value.value_native
        Money(USD, 1000000.0)
    """

    @property
    def position_id(self) -> str:
        """Get the position identifier."""
        ...

    @property
    def entity_id(self) -> str:
        """Get the entity identifier."""
        ...

    @property
    def value_native(self) -> Money:
        """Get the value in the instrument's native currency."""
        ...

    @property
    def value_base(self) -> Money:
        """Get the value converted to portfolio base currency."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class PortfolioValuation:
    """Complete portfolio valuation results.

    Provides per-position valuations, totals by entity, and the grand total.

    Examples:
        >>> valuation = value_portfolio(portfolio, market_context, config)
        >>> valuation.total_base_ccy
        Money(USD, 10000000.0)
        >>> valuation.by_entity["ENTITY_A"]
        Money(USD, 5000000.0)
    """

    def get_position_value(self, position_id: str) -> Optional[PositionValue]:
        """Get the value for a specific position.

        Args:
            position_id: Identifier to query.

        Returns:
            PositionValue or None: The position value if found.

        Examples:
            >>> position_value = valuation.get_position_value("POS_1")
        """
        ...

    def get_entity_value(self, entity_id: str) -> Optional[Money]:
        """Get the total value for a specific entity.

        Args:
            entity_id: Entity identifier to query.

        Returns:
            Money or None: The entity's total value if found.

        Examples:
            >>> entity_value = valuation.get_entity_value("ENTITY_A")
        """
        ...

    @property
    def position_values(self) -> Dict[str, PositionValue]:
        """Get values for each position."""
        ...

    @property
    def total_base_ccy(self) -> Money:
        """Get the total portfolio value in base currency."""
        ...

    @property
    def by_entity(self) -> Dict[str, Money]:
        """Get aggregated values by entity."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

def value_portfolio(
    portfolio: Portfolio,
    market_context: MarketContext,
    config: Optional[FinstackConfig] = None,
) -> PortfolioValuation:
    """Value a complete portfolio.

    Args:
        portfolio: Portfolio to value.
        market_context: Market data context.
        config: Finstack configuration (optional, uses default if not provided).

    Returns:
        PortfolioValuation: Complete valuation results.

    Raises:
        RuntimeError: If valuation fails.

    Examples:
        >>> from finstack.portfolio import value_portfolio
        >>> from finstack.core import FinstackConfig
        >>> valuation = value_portfolio(portfolio, market_context, FinstackConfig())
    """
    ...
