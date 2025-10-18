"""Portfolio core types."""

from typing import Dict, Any, Optional
from ...core.currency import Currency

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

        Returns:
            Entity: New entity instance.

        Examples:
            >>> entity = Entity("ACME_CORP")
            >>> entity.id
            'ACME_CORP'
        """
        ...

    def with_name(self, name: str) -> "Entity":
        """Set the entity name.

        Args:
            name: Human-readable name.

        Returns:
            Entity: Entity with updated name (builder pattern).

        Examples:
            >>> entity = Entity("ACME").with_name("Acme Corporation")
            >>> entity.name
            'Acme Corporation'
        """
        ...

    def with_tag(self, key: str, value: str) -> "Entity":
        """Add a tag to the entity.

        Args:
            key: Tag key.
            value: Tag value.

        Returns:
            Entity: Entity with added tag (builder pattern).

        Examples:
            >>> entity = Entity("ACME").with_tag("sector", "Technology")
            >>> entity.tags["sector"]
            'Technology'
        """
        ...

    @staticmethod
    def dummy() -> "Entity":
        """Create the dummy entity for standalone instruments.

        Returns:
            Entity: Dummy entity with special identifier.

        Examples:
            >>> dummy = Entity.dummy()
            >>> dummy.id
            '_standalone'
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

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class PositionUnit:
    """Unit of position measurement.

    Describes how the quantity on a position should be interpreted.

    Variants:
        UNITS: Number of units/shares (for equities, baskets)
        NOTIONAL: Notional amount, optionally in a specific currency (for derivatives, FX)
        FACE_VALUE: Face value of debt instruments (for bonds, loans)
        PERCENTAGE: Percentage of ownership

    Examples:
        >>> unit = PositionUnit.UNITS
        >>> unit = PositionUnit.notional_with_ccy(Currency.USD)
        >>> unit = PositionUnit.FACE_VALUE
    """

    # Class attributes
    UNITS: PositionUnit
    FACE_VALUE: PositionUnit
    PERCENTAGE: PositionUnit

    @staticmethod
    def notional() -> "PositionUnit":
        """Create a notional position unit without specific currency.

        Returns:
            PositionUnit: Notional unit.
        """
        ...

    @staticmethod
    def notional_with_ccy(currency: Currency) -> "PositionUnit":
        """Create a notional position unit with specific currency.

        Args:
            currency: Currency for the notional amount.

        Returns:
            PositionUnit: Notional unit with currency.
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class Position:
    """A position in an instrument.

    Represents a holding of a specific quantity of an instrument, belonging to an entity.
    Positions track the instrument reference, quantity, unit, and metadata for aggregation.

    Examples:
        >>> from finstack.valuations.instruments import Deposit
        >>> from finstack.core import Money, Currency
        >>> deposit = Deposit.fixed("DEP_1M", Money(Currency.USD, 1_000_000), ...)
        >>> position = Position("POS_001", "ENTITY_A", "DEP_1M", deposit, 1.0, PositionUnit.UNITS)
        >>> position.is_long()
        True
    """

    def __init__(
        self,
        position_id: str,
        entity_id: str,
        instrument_id: str,
        instrument: Any,
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

        Returns:
            Position: New position instance.

        Raises:
            TypeError: If instrument is not a valid instrument type.
        """
        ...

    def is_long(self) -> bool:
        """Check if the position is long (positive quantity).

        Returns:
            bool: True if quantity is positive.
        """
        ...

    def is_short(self) -> bool:
        """Check if the position is short (negative quantity).

        Returns:
            bool: True if quantity is negative.
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

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
