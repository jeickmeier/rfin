"""Shared enums, keys, and error helpers used across finstack valuations bindings."""

from typing import Union

# Monte Carlo submodule
from . import mc

class InstrumentType:
    """Enumerates instrument families supported by the valuation engines.

    Examples:
        >>> from finstack.valuations.common import InstrumentType
        >>> InstrumentType.BOND.name
        'bond'
    """

    # Class attributes
    BOND: InstrumentType
    LOAN: InstrumentType
    CDS: InstrumentType
    CDS_INDEX: InstrumentType
    CDS_TRANCHE: InstrumentType
    CDS_OPTION: InstrumentType
    IRS: InstrumentType
    CAP_FLOOR: InstrumentType
    SWAPTION: InstrumentType
    TRS: InstrumentType
    BASIS_SWAP: InstrumentType
    BASKET: InstrumentType
    CONVERTIBLE: InstrumentType
    DEPOSIT: InstrumentType
    EQUITY_OPTION: InstrumentType
    FX_OPTION: InstrumentType
    FX_SPOT: InstrumentType
    FX_SWAP: InstrumentType
    INFLATION_LINKED_BOND: InstrumentType
    INFLATION_SWAP: InstrumentType
    INTEREST_RATE_FUTURE: InstrumentType
    VARIANCE_SWAP: InstrumentType
    EQUITY: InstrumentType
    REPO: InstrumentType
    FRA: InstrumentType
    STRUCTURED_CREDIT: InstrumentType
    PRIVATE_MARKETS_FUND: InstrumentType

    @classmethod
    def from_name(cls, name: str) -> InstrumentType:
        """Convert a snake-case label into an instrument family.

        Args:
            name: Instrument family label such as "bond".

        Returns:
            InstrumentType: Enumeration value that matches name.

        Raises:
            ValueError: If the label is unknown.

        Examples:
            >>> from finstack.valuations.common import InstrumentType
            >>> InstrumentType.from_name("bond")
            InstrumentType.BOND
        """
        ...

    @property
    def name(self) -> str:
        """Snake-case identifier for the instrument family.

        Returns:
            str: Normalized instrument label such as "bond".
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __richcmp__(self, other: object, op: int) -> object: ...

class ModelKey:
    """Enumerates pricing model categories recognized by the registry.

    Examples:
        >>> from finstack.valuations.common import ModelKey
        >>> ModelKey.DISCOUNTING.name
        'discounting'
    """

    # Class attributes
    DISCOUNTING: ModelKey
    TREE: ModelKey
    BLACK76: ModelKey
    HULL_WHITE_1F: ModelKey
    HAZARD_RATE: ModelKey

    @classmethod
    def from_name(cls, name: str) -> ModelKey:
        """Convert a snake-case label into a pricing model key.

        Args:
            name: Pricing model label such as "discounting".

        Returns:
            ModelKey: Enumeration value that corresponds to name.

        Raises:
            ValueError: If the label is not supported.

        Examples:
            >>> from finstack.valuations.common import ModelKey
            >>> ModelKey.from_name("discounting")
            ModelKey.DISCOUNTING
        """
        ...

    @property
    def name(self) -> str:
        """Snake-case identifier for this pricing model.

        Returns:
            str: Normalized model label such as "discounting".
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __richcmp__(self, other: object, op: int) -> object: ...

class PricerKey:
    """Composite key identifying a specific instrument/model pairing.

    Examples:
        >>> from finstack.valuations.common import InstrumentType, ModelKey, PricerKey
        >>> PricerKey(InstrumentType.BOND, ModelKey.DISCOUNTING)
        PricerKey(instrument='bond', model='discounting')
    """

    def __init__(self, instrument: Union[InstrumentType, str], model: Union[ModelKey, str]) -> None:
        """Build a key that refers to a (instrument, model) pair.

        Args:
            instrument: Instrument type or snake-case label.
            model: Model key or snake-case label.

        Returns:
            PricerKey: Identifier usable with PricerRegistry.

        Raises:
            ValueError: If either identifier is not recognized.
        """
        ...

    @property
    def instrument(self) -> InstrumentType:
        """Instrument type component of the key.

        Returns:
            InstrumentType: Instrument portion of the key.
        """
        ...

    @property
    def model(self) -> ModelKey:
        """Model key component of the key.

        Returns:
            ModelKey: Model portion of the key.
        """
        ...

    def __repr__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __richcmp__(self, other: object, op: int) -> object: ...
