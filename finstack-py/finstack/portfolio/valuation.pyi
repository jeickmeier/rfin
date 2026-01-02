"""Portfolio valuation."""

from typing import Optional, Dict, Any
from finstack.core.money import Money
from finstack.core.market_data.context import MarketContext
from finstack.core.config import FinstackConfig
from .portfolio import Portfolio

class PortfolioValuationOptions:
    """Options controlling portfolio valuation behaviour."""

    def __init__(self, *, strict_risk: bool = False) -> None: ...
    @property
    def strict_risk(self) -> bool:
        """When True, fail if requested risk metrics cannot be computed for a position."""
        ...

class PositionValue:
    """Result of valuing a single position.

    Holds both native-currency and base-currency valuations.

    Examples
    --------
        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.portfolio import (
        ...     PortfolioBuilder,
        ...     Entity,
        ...     Position,
        ...     PositionUnit,
        ...     value_portfolio,
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
        >>> valuation = value_portfolio(portfolio, MarketContext())
        >>> pv = valuation.get_position_value("POS-1")
        >>> (pv.position_id, pv.value_native.amount)
        ('POS-1', 12000.0)
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

    Examples
    --------
        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.portfolio import (
        ...     PortfolioBuilder,
        ...     Entity,
        ...     Position,
        ...     PositionUnit,
        ...     value_portfolio,
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
        >>> valuation = value_portfolio(portfolio, MarketContext())
        >>> (valuation.total_base_ccy.amount, sorted(valuation.position_values.keys()))
        (12000.0, ['POS-1'])
    """

    def get_position_value(self, position_id: str) -> Optional[PositionValue]:
        """Get the value for a specific position.

        Args:
            position_id: Identifier to query.

        Returns:
            PositionValue or None: The position value if found.

        Examples
        --------
            >>> from datetime import date
            >>> from finstack.core.currency import Currency
            >>> from finstack.core.market_data.context import MarketContext
            >>> from finstack.portfolio import (
            ...     PortfolioBuilder,
            ...     Entity,
            ...     Position,
            ...     PositionUnit,
            ...     value_portfolio,
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
            >>> valuation = value_portfolio(portfolio, MarketContext())
            >>> valuation.get_position_value("POS-1").value_base.amount
            12000.0
        """
        ...

    def get_entity_value(self, entity_id: str) -> Optional[Money]:
        """Get the total value for a specific entity.

        Args:
            entity_id: Entity identifier to query.

        Returns:
            Money or None: The entity's total value if found.

        Examples
        --------
            >>> from datetime import date
            >>> from finstack.core.currency import Currency
            >>> from finstack.core.market_data.context import MarketContext
            >>> from finstack.portfolio import (
            ...     PortfolioBuilder,
            ...     Entity,
            ...     Position,
            ...     PositionUnit,
            ...     value_portfolio,
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
            >>> valuation = value_portfolio(portfolio, MarketContext())
            >>> valuation.get_entity_value("ACME").amount
            12000.0
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
    """Value a complete portfolio with multi-currency aggregation.

    This function values all positions in a portfolio, aggregates by entity,
    and converts all values to the portfolio's base currency using explicit
    FX conversion policies. It computes both position-level and entity-level
    valuations with risk metrics.

    Parameters
    ----------
    portfolio : Portfolio
        Portfolio containing positions, entities, and base currency.
        All positions must reference valid entities and instruments.
    market_context : MarketContext
        Market data context with curves, surfaces, FX rates, and spot prices.
        Must contain all market data required by portfolio instruments.
    config : FinstackConfig, optional
        Finstack configuration for rounding, numeric mode, and FX policies.
        If None, uses default configuration.

    Returns
    -------
    PortfolioValuation
        Complete valuation results including:
        - position_values: Per-position valuations (native and base currency)
        - by_entity: Aggregated values by entity (base currency)
        - total_base_ccy: Grand total in base currency

    Raises
    ------
    ValueError
        If portfolio structure is invalid (missing entities, invalid references).
    RuntimeError
        If valuation fails (missing market data, pricing errors, FX conversion
        failures).

    Examples
    --------
    Value a simple equity portfolio:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.portfolio import (
        ...     PortfolioBuilder,
        ...     Entity,
        ...     Position,
        ...     PositionUnit,
        ...     value_portfolio,
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
        >>> valuation = value_portfolio(portfolio, MarketContext())
        >>> print(valuation.total_base_ccy.amount)
        12000.0
        >>> valuation.get_entity_value("ACME").amount
        12000.0
        >>> valuation.get_position_value("POS-1").value_native.amount
        12000.0

    Notes
    -----
    - All positions are valued using their native currency
    - Values are converted to base currency using FX rates from MarketContext
    - Entity aggregation sums all position values for that entity
    - Total is the sum of all entity values in base currency
    - Risk metrics are computed per-position and can be aggregated separately

    See Also
    --------
    :class:`Portfolio`: Portfolio structure
    :class:`PortfolioValuation`: Valuation results
    :class:`PositionValue`: Individual position valuation
    :class:`MarketContext`: Market data container
    """
    ...

def value_portfolio_with_options(
    portfolio: Portfolio,
    market_context: MarketContext,
    options: PortfolioValuationOptions,
    config: Optional[FinstackConfig] = None,
) -> PortfolioValuation:
    """Value a portfolio using explicit valuation options (e.g., strict_risk)."""
    ...
