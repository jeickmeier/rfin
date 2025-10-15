"""
Python type stubs for finstack.portfolio module.

This module provides portfolio management and aggregation capabilities,
including entity and position management, valuation, metrics calculation,
and scenario integration.

Note: Import as `import finstack` then access via `finstack.portfolio.*`
rather than `from finstack.portfolio import *`.
"""

from typing import Any, Dict, List, Optional, Tuple
from datetime import date
from finstack.core import Currency, Money, FinstackConfig
from finstack.core.market_data import MarketContext
from finstack.scenarios import ScenarioSpec

# Core Types

class Entity:
    """An entity that can hold positions.
    
    Entities represent companies, funds, or other legal entities that own instruments.
    For standalone instruments, use the dummy entity via Entity.dummy().
    
    Examples:
        >>> entity = Entity("ACME_CORP")
        >>> entity = entity.with_name("Acme Corporation")
        >>> entity = entity.with_tag("sector", "Technology")
    """
    
    def __init__(self, id: str) -> None:
        """Create a new entity with the given ID.
        
        Args:
            id: Unique entity identifier.
        """
        ...
    
    def with_name(self, name: str) -> Entity:
        """Set the entity name.
        
        Args:
            name: Human-readable name.
            
        Returns:
            Entity with updated name (builder pattern).
        """
        ...
    
    def with_tag(self, key: str, value: str) -> Entity:
        """Add a tag to the entity.
        
        Args:
            key: Tag key.
            value: Tag value.
            
        Returns:
            Entity with added tag (builder pattern).
        """
        ...
    
    @staticmethod
    def dummy() -> Entity:
        """Create the dummy entity for standalone instruments.
        
        Returns:
            Dummy entity with special identifier '_standalone'.
        """
        ...
    
    @property
    def id(self) -> str:
        """Get the entity identifier."""
        ...
    
    @property
    def name(self) -> Optional[str]:
        """Get the entity name."""
        ...
    
    @property
    def tags(self) -> Dict[str, str]:
        """Get the entity tags."""
        ...
    
    @property
    def meta(self) -> Dict[str, Any]:
        """Get entity metadata."""
        ...


class PositionUnit:
    """Unit of position measurement.
    
    Describes how the quantity on a position should be interpreted.
    
    Variants:
        UNITS: Number of units/shares (for equities, baskets)
        NOTIONAL: Notional amount, optionally in a specific currency
        FACE_VALUE: Face value of debt instruments (for bonds, loans)
        PERCENTAGE: Percentage of ownership
    
    Examples:
        >>> unit = PositionUnit.UNITS
        >>> unit = PositionUnit.notional_with_ccy(Currency.USD)
        >>> unit = PositionUnit.FACE_VALUE
    """
    
    UNITS: PositionUnit
    FACE_VALUE: PositionUnit
    PERCENTAGE: PositionUnit
    
    @staticmethod
    def notional() -> PositionUnit:
        """Create a notional position unit without specific currency.
        
        Returns:
            Notional unit.
        """
        ...
    
    @staticmethod
    def notional_with_ccy(currency: Currency) -> PositionUnit:
        """Create a notional position unit with specific currency.
        
        Args:
            currency: Currency for the notional amount.
            
        Returns:
            Notional unit with currency.
        """
        ...


class Position:
    """A position in an instrument.
    
    Represents a holding of a specific quantity of an instrument, belonging to an entity.
    Positions track the instrument reference, quantity, unit, and metadata for aggregation.
    
    Examples:
        >>> from finstack.valuations.instruments import Deposit
        >>> deposit = Deposit.fixed(...)
        >>> position = Position("POS_001", "ENTITY_A", "DEP_1M", deposit, 1.0, PositionUnit.UNITS)
        >>> position.is_long()
        True
    """
    
    def __init__(
        self,
        position_id: str,
        entity_id: str,
        instrument_id: str,
        instrument: Any,  # Instrument type
        quantity: float,
        unit: PositionUnit,
    ) -> None:
        """Create a new position.
        
        Args:
            position_id: Unique identifier for the position.
            entity_id: Owning entity identifier.
            instrument_id: Instrument identifier (for reference/lookup).
            instrument: The actual instrument being held.
            quantity: Signed quantity (positive=long, negative=short).
            unit: Unit of measurement for the quantity.
        """
        ...
    
    def is_long(self) -> bool:
        """Check if the position is long (positive quantity).
        
        Returns:
            True if quantity is positive.
        """
        ...
    
    def is_short(self) -> bool:
        """Check if the position is short (negative quantity).
        
        Returns:
            True if quantity is negative.
        """
        ...
    
    @property
    def position_id(self) -> str:
        """Get the position identifier."""
        ...
    
    @property
    def entity_id(self) -> str:
        """Get the entity identifier."""
        ...
    
    @property
    def instrument_id(self) -> str:
        """Get the instrument identifier."""
        ...
    
    @property
    def quantity(self) -> float:
        """Get the quantity."""
        ...
    
    @property
    def unit(self) -> PositionUnit:
        """Get the position unit."""
        ...
    
    @property
    def tags(self) -> Dict[str, str]:
        """Get position tags."""
        ...
    
    @property
    def meta(self) -> Dict[str, Any]:
        """Get position metadata."""
        ...


# Portfolio

class Portfolio:
    """A portfolio of positions across multiple entities.
    
    The portfolio holds a flat list of positions, each referencing an entity and instrument.
    Positions can be grouped and aggregated by entity or by arbitrary attributes (tags).
    
    Examples:
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
        """
        ...
    
    def get_position(self, position_id: str) -> Optional[Position]:
        """Get a position by identifier.
        
        Args:
            position_id: Identifier of the position to locate.
            
        Returns:
            The position if found, None otherwise.
        """
        ...
    
    def positions_for_entity(self, entity_id: str) -> List[Position]:
        """Get all positions for a given entity.
        
        Args:
            entity_id: Entity identifier used for filtering.
            
        Returns:
            List of positions for the entity.
        """
        ...
    
    def positions_with_tag(self, key: str, value: str) -> List[Position]:
        """Get all positions with a specific tag value.
        
        Args:
            key: Tag key to filter by.
            value: Tag value to match.
            
        Returns:
            List of positions with matching tag.
        """
        ...
    
    def validate(self) -> None:
        """Validate the portfolio structure and references.
        
        Checks that all positions reference valid entities and that structural
        invariants are maintained.
        
        Raises:
            ValueError: If validation fails.
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


class PortfolioBuilder:
    """Builder for constructing a Portfolio with validation.
    
    The builder stores all intermediate values needed to construct a portfolio and checks
    invariants such as base currency, valuation date, and entity references before the
    final portfolio is produced.
    
    Examples:
        >>> builder = (PortfolioBuilder("FUND_A")
        ...     .name("Alpha Fund")
        ...     .base_ccy(Currency.USD)
        ...     .as_of(date(2024, 1, 1))
        ...     .entity(Entity("ACME"))
        ...     .build())
    """
    
    def __init__(self, id: str) -> None:
        """Create a new portfolio builder with the given identifier.
        
        Args:
            id: Unique identifier for the portfolio.
        """
        ...
    
    def name(self, name: str) -> PortfolioBuilder:
        """Set the portfolio's human-readable name.
        
        Args:
            name: Display name stored alongside the portfolio identifier.
            
        Returns:
            Self for chaining.
        """
        ...
    
    def base_ccy(self, ccy: Currency) -> PortfolioBuilder:
        """Declare the portfolio's reporting currency.
        
        Args:
            ccy: Currency to use when consolidating values and metrics.
            
        Returns:
            Self for chaining.
        """
        ...
    
    def as_of(self, date: date) -> PortfolioBuilder:
        """Assign the valuation date used for pricing and analytics.
        
        Args:
            date: The as-of date for valuation and risk calculation.
            
        Returns:
            Self for chaining.
        """
        ...
    
    def entity(self, entity_or_entities: Entity | List[Entity]) -> PortfolioBuilder:
        """Register entity or entities with the builder.
        
        Accepts either a single Entity or a list of entities.
        
        Args:
            entity_or_entities: Entity or list of entities to register.
            
        Returns:
            Self for chaining.
        """
        ...
    
    def position(self, position_or_positions: Position | List[Position]) -> PortfolioBuilder:
        """Add position or positions to the portfolio.
        
        Accepts either a single Position or a list of positions.
        
        Args:
            position_or_positions: Position or list of positions to add.
            
        Returns:
            Self for chaining.
        """
        ...
    
    def tag(self, key: str, value: str) -> PortfolioBuilder:
        """Add a portfolio-level tag.
        
        Args:
            key: Tag key.
            value: Tag value.
            
        Returns:
            Self for chaining.
        """
        ...
    
    def meta(self, key: str, value: Any) -> PortfolioBuilder:
        """Add portfolio-level metadata.
        
        Args:
            key: Metadata key.
            value: Metadata value (must be JSON-serializable).
            
        Returns:
            Self for chaining.
        """
        ...
    
    def build(self) -> Portfolio:
        """Build and validate the portfolio.
        
        Returns:
            Validated portfolio instance.
            
        Raises:
            ValueError: If validation fails (missing base_ccy, as_of, or invalid references).
        """
        ...


# Valuation

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
            The position value if found, None otherwise.
        """
        ...
    
    def get_entity_value(self, entity_id: str) -> Optional[Money]:
        """Get the total value for a specific entity.
        
        Args:
            entity_id: Entity identifier to query.
            
        Returns:
            The entity's total value if found, None otherwise.
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
        Complete valuation results.
        
    Raises:
        RuntimeError: If valuation fails.
    
    Examples:
        >>> valuation = value_portfolio(portfolio, market_context, FinstackConfig())
    """
    ...


# Metrics

class AggregatedMetric:
    """Aggregated metric across the portfolio.
    
    Contains portfolio-wide totals as well as breakdowns by entity.
    
    Examples:
        >>> metric = metrics.get_metric("dv01")
        >>> metric.total
        125.0
        >>> metric.by_entity["ENTITY_A"]
        75.0
    """
    
    @property
    def metric_id(self) -> str:
        """Get the metric identifier."""
        ...
    
    @property
    def total(self) -> float:
        """Get the total value across all positions (for summable metrics)."""
        ...
    
    @property
    def by_entity(self) -> Dict[str, float]:
        """Get aggregated values by entity."""
        ...


class PortfolioMetrics:
    """Complete portfolio metrics results.
    
    Holds both aggregated metrics and per-position values.
    
    Examples:
        >>> metrics = aggregate_metrics(valuation)
        >>> dv01 = metrics.get_metric("dv01")
        >>> position_metrics = metrics.get_position_metrics("POS_1")
    """
    
    def get_metric(self, metric_id: str) -> Optional[AggregatedMetric]:
        """Get an aggregated metric by identifier.
        
        Args:
            metric_id: Identifier of the metric to look up.
            
        Returns:
            The metric if found, None otherwise.
        """
        ...
    
    def get_position_metrics(self, position_id: str) -> Optional[Dict[str, float]]:
        """Get metrics for a specific position.
        
        Args:
            position_id: Identifier of the position to query.
            
        Returns:
            Mapping of metric IDs to values for the position, or None if not found.
        """
        ...
    
    def get_total(self, metric_id: str) -> Optional[float]:
        """Get the total value of a specific metric across the portfolio.
        
        Args:
            metric_id: Identifier of the metric.
            
        Returns:
            Total metric value if found, None otherwise.
        """
        ...
    
    @property
    def aggregated(self) -> Dict[str, AggregatedMetric]:
        """Get aggregated metrics (summable only)."""
        ...
    
    @property
    def by_position(self) -> Dict[str, Dict[str, float]]:
        """Get raw metrics by position (all metrics)."""
        ...


def aggregate_metrics(valuation: PortfolioValuation) -> PortfolioMetrics:
    """Aggregate metrics from portfolio valuation.
    
    Computes portfolio-wide metrics by summing position-level results where appropriate.
    Only summable metrics (DV01, CS01, Theta, etc.) are aggregated.
    
    Args:
        valuation: Portfolio valuation results.
        
    Returns:
        Aggregated metrics results.
        
    Raises:
        RuntimeError: If aggregation fails.
    
    Examples:
        >>> metrics = aggregate_metrics(valuation)
        >>> metrics.get_total("dv01")
        125.0
    """
    ...


# Results

class PortfolioResults:
    """Complete results from portfolio evaluation.
    
    Contains valuation, metrics, and metadata about the calculation.
    
    Examples:
        >>> results.total_value()
        Money(USD, 10000000.0)
        >>> results.get_metric("dv01")
        125.0
    """
    
    def __init__(
        self,
        valuation: PortfolioValuation,
        metrics: PortfolioMetrics,
        meta: Dict[str, Any],
    ) -> None:
        """Create a new portfolio results instance.
        
        Args:
            valuation: Portfolio valuation component.
            metrics: Portfolio metrics component.
            meta: Metadata describing calculation context.
        """
        ...
    
    def total_value(self) -> Money:
        """Get the total portfolio value.
        
        Returns:
            Total portfolio value in base currency.
        """
        ...
    
    def get_metric(self, metric_id: str) -> Optional[float]:
        """Get a specific aggregated metric.
        
        Args:
            metric_id: Identifier of the metric to retrieve.
            
        Returns:
            Metric value if found, None otherwise.
        """
        ...
    
    @property
    def valuation(self) -> PortfolioValuation:
        """Get the portfolio valuation results."""
        ...
    
    @property
    def metrics(self) -> PortfolioMetrics:
        """Get the aggregated metrics."""
        ...
    
    @property
    def meta(self) -> Dict[str, Any]:
        """Get metadata about the calculation."""
        ...


# Grouping

def group_by_attribute(portfolio: Portfolio, attribute_key: str) -> Dict[str, List[Position]]:
    """Group portfolio positions by an attribute.
    
    Returns a dictionary mapping attribute values to lists of positions.
    The attribute key must exist in position tags for positions to be included.
    
    Args:
        portfolio: Portfolio to group.
        attribute_key: Tag key to group by (e.g., "sector", "rating").
        
    Returns:
        Mapping of attribute values to position lists.
        
    Raises:
        RuntimeError: If grouping fails.
    
    Examples:
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
        Mapping of attribute values to aggregated amounts.
        
    Raises:
        RuntimeError: If aggregation fails.
    
    Examples:
        >>> by_sector = aggregate_by_attribute(valuation, portfolio, "sector")
        >>> by_sector["Technology"]
        Money(USD, 5000000.0)
    """
    ...


# Scenarios (optional feature)

def apply_scenario(
    portfolio: Portfolio,
    scenario: ScenarioSpec,
    market_context: MarketContext,
) -> Portfolio:
    """Apply a scenario to a portfolio.
    
    Transforms the portfolio by applying scenario operations. The original portfolio
    is not modified; a new portfolio with transformed positions is returned.
    
    Args:
        portfolio: Portfolio to transform.
        scenario: Scenario specification to apply.
        market_context: Market data context.
        
    Returns:
        Transformed portfolio.
        
    Raises:
        RuntimeError: If scenario application fails.
    
    Examples:
        >>> transformed = apply_scenario(portfolio, scenario, market_context)
    """
    ...


def apply_and_revalue(
    portfolio: Portfolio,
    scenario: ScenarioSpec,
    market_context: MarketContext,
    config: Optional[FinstackConfig] = None,
) -> PortfolioValuation:
    """Apply a scenario to a portfolio and revalue it.
    
    Convenience function that applies a scenario and then values the resulting portfolio.
    Equivalent to calling apply_scenario followed by value_portfolio.
    
    Args:
        portfolio: Portfolio to transform and value.
        scenario: Scenario specification to apply.
        market_context: Market data context.
        config: Finstack configuration (optional, uses default if not provided).
        
    Returns:
        Portfolio valuation results.
        
    Raises:
        RuntimeError: If scenario application or valuation fails.
    
    Examples:
        >>> valuation = apply_and_revalue(portfolio, scenario, market_context)
        >>> valuation.total_base_ccy
        Money(USD, 9500000.0)
    """
    ...

