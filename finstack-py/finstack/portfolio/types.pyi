"""Portfolio core types."""

from __future__ import annotations
from typing import Dict, Any, Mapping, Iterable, Tuple, List
from finstack.core.currency import Currency
from finstack.core.money import Money

DUMMY_ENTITY_ID: str
"""Constant for the dummy entity used for standalone instruments ('_standalone')."""

class BookId:
    """Book identifier."""

    def __init__(self, id: str) -> None: ...
    @property
    def id(self) -> str:
        """Get the identifier as a string."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class Book:
    """A book in the portfolio hierarchy."""

    def __init__(self, id: str | BookId, name: str | None = None, parent_id: str | BookId | None = None) -> None: ...
    @property
    def id(self) -> str: ...
    @property
    def name(self) -> str | None: ...
    @property
    def parent_id(self) -> str | None: ...
    @property
    def position_ids(self) -> List[str]: ...
    @property
    def child_book_ids(self) -> List[str]: ...
    @property
    def tags(self) -> Dict[str, str]: ...
    @property
    def meta(self) -> Dict[str, Any]: ...
    def is_root(self) -> bool:
        """Check if this is a root book (no parent)."""
        ...

    def contains_position(self, position_id: str) -> bool:
        """Check if this book directly contains a position."""
        ...

    def contains_child(self, child_id: str) -> bool:
        """Check if this book contains a specific child book."""
        ...

    def add_position(self, position_id: str) -> None:
        """Add a position to this book."""
        ...

    def add_child(self, child_id: str) -> None:
        """Add a child book to this book."""
        ...

    def remove_position(self, position_id: str) -> None:
        """Remove a position from this book."""
        ...

    def remove_child(self, child_id: str) -> None:
        """Remove a child book from this book."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class Entity:
    """An entity that can hold positions.

    Entities represent companies, funds, or other legal entities that own instruments.
    For standalone instruments, use the dummy entity via Entity.dummy().

    Examples
    --------
    Create and tag an entity:

        >>> from finstack.portfolio import Entity
        >>> entity = Entity("ACME_CORP").with_name("Acme Corporation")
        >>> tagged = entity.with_tag("sector", "Technology")
        >>> print(tagged.id, tagged.name, tagged.tags["sector"])
        ACME_CORP Acme Corporation Technology
    """

    def __init__(self, id: str) -> None:
        """Create a new entity with the given ID.

        Args:
            id: Unique entity identifier.

        Returns:
            Entity: New entity instance.

        Examples
        --------
            >>> from finstack.portfolio import Entity
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

        Examples
        --------
            >>> from finstack.portfolio import Entity
            >>> Entity("ACME").with_name("Acme Corporation").name
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

        Examples
        --------
            >>> from finstack.portfolio import Entity
            >>> tagged = Entity("ACME").with_tag("sector", "Technology")
            >>> tagged.tags["sector"]
            'Technology'
        """
        ...

    def with_tags(self, tags: Mapping[str, str] | Iterable[Tuple[str, str]]) -> "Entity":
        """Add multiple tags to the entity from a mapping or iterable of pairs."""
        ...

    @staticmethod
    def dummy() -> "Entity":
        """Create the dummy entity for standalone instruments.

        Returns:
            Entity: Dummy entity with special identifier.

        Examples
        --------
            >>> from finstack.portfolio import Entity
            >>> Entity.dummy().id
            '_standalone'
        """
        ...

    @property
    def id(self) -> str:
        """Get the entity identifier."""
        ...

    @property
    def name(self) -> str | None:
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

    Examples
    --------
        >>> from finstack.core.currency import Currency
        >>> from finstack.portfolio import PositionUnit
        >>> (str(PositionUnit.UNITS), str(PositionUnit.notional_with_ccy(Currency("USD"))))
        ('units', 'notional(USD)')
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

    Examples
    --------
        >>> from finstack.core.currency import Currency
        >>> from finstack.valuations.instruments import Equity
        >>> from finstack.portfolio import Position, PositionUnit
        >>> equity = Equity.builder("EQ-ACME").ticker("ACME").currency(Currency("USD")).price(120.0).build()
        >>> position = Position("POS-1", "ENTITY_A", equity.instrument_id, equity, 100.0, PositionUnit.UNITS)
        >>> (position.is_long(), position.instrument_id)
        (True, 'EQ-ACME')
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

    def with_tag(self, key: str, value: str) -> "Position":
        """Add a tag to the position."""
        ...

    def with_book(self, book_id: str | BookId) -> "Position":
        """Assign this position to a book (builder pattern)."""
        ...

    def with_tags(self, tags: Mapping[str, str] | Iterable[Tuple[str, str]]) -> "Position":
        """Add multiple tags to the position."""
        ...

    def with_meta(self, key: str, value: Any) -> "Position":
        """Attach JSON-serializable metadata to the position."""
        ...

    def scale_value(self, money: Money) -> Money:
        """Scale a Money amount by position quantity and unit."""
        ...

    @property
    def position_id(self) -> str:
        """Position identifier."""
        ...

    @property
    def entity_id(self) -> str:
        """Owning entity identifier."""
        ...

    @property
    def instrument_id(self) -> str:
        """Instrument identifier."""
        ...

    @property
    def book_id(self) -> str | None:
        """Book identifier (None if unassigned)."""
        ...

    @property
    def quantity(self) -> float:
        """Signed quantity (positive=long, negative=short)."""
        ...

    @property
    def unit(self) -> PositionUnit:
        """Unit describing how to interpret quantity."""
        ...

    @property
    def tags(self) -> Dict[str, str]:
        """Position tags used for grouping."""
        ...

    @property
    def meta(self) -> Dict[str, Any]:
        """Position metadata."""
        ...

    @property
    def instrument(self) -> Any:
        """The instrument held by this position."""
        ...

    def to_spec(self) -> "PositionSpec":
        """Convert to a serializable specification."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class PositionSpec:
    """A serializable position specification."""

    @property
    def position_id(self) -> str:
        """Position identifier."""
        ...

    @property
    def entity_id(self) -> str:
        """Entity identifier."""
        ...

    @property
    def instrument_id(self) -> str:
        """Instrument identifier."""
        ...

    @property
    def quantity(self) -> float:
        """Signed quantity."""
        ...

    @property
    def unit(self) -> PositionUnit:
        """Unit of measurement."""
        ...

    @property
    def book_id(self) -> str | None:
        """Book identifier (None if unassigned)."""
        ...

    @property
    def tags(self) -> Dict[str, str]:
        """Position tags."""
        ...

    @property
    def meta(self) -> Dict[str, Any]:
        """Position metadata."""
        ...

    def to_json(self) -> str:
        """Serialize to JSON string."""
        ...

    @staticmethod
    def from_json(json_str: str) -> "PositionSpec":
        """Deserialize from JSON string."""
        ...

    def __repr__(self) -> str: ...
