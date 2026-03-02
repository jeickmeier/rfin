"""Shared enums, keys, and error helpers used across finstack valuations bindings."""

from __future__ import annotations

# Monte Carlo submodule
from . import monte_carlo
from . import parse

class InstrumentType:
    """Enumerates instrument families supported by the valuation engines.

    Examples:
        >>> from finstack.valuations.common import InstrumentType
        >>> InstrumentType.BOND.name
        'bond'
    """

    # Fixed Income
    BOND: InstrumentType
    LOAN: InstrumentType
    CONVERTIBLE: InstrumentType
    INFLATION_LINKED_BOND: InstrumentType
    TERM_LOAN: InstrumentType
    BOND_FUTURE: InstrumentType
    STRUCTURED_CREDIT: InstrumentType
    REVOLVING_CREDIT: InstrumentType
    AGENCY_MBS_PASSTHROUGH: InstrumentType
    AGENCY_TBA: InstrumentType
    DOLLAR_ROLL: InstrumentType
    AGENCY_CMO: InstrumentType

    # Interest Rates
    DEPOSIT: InstrumentType
    FRA: InstrumentType
    IRS: InstrumentType
    BASIS_SWAP: InstrumentType
    CAP_FLOOR: InstrumentType
    SWAPTION: InstrumentType
    BERMUDAN_SWAPTION: InstrumentType
    REPO: InstrumentType
    INTEREST_RATE_FUTURE: InstrumentType
    INFLATION_SWAP: InstrumentType
    YOY_INFLATION_SWAP: InstrumentType
    INFLATION_CAP_FLOOR: InstrumentType
    XCCY_SWAP: InstrumentType
    CMS_OPTION: InstrumentType
    RANGE_ACCRUAL: InstrumentType

    # Credit Derivatives
    CDS: InstrumentType
    CDS_INDEX: InstrumentType
    CDS_TRANCHE: InstrumentType
    CDS_OPTION: InstrumentType

    # FX
    FX_SPOT: InstrumentType
    FX_SWAP: InstrumentType
    FX_FORWARD: InstrumentType
    FX_OPTION: InstrumentType
    FX_BARRIER_OPTION: InstrumentType
    FX_DIGITAL_OPTION: InstrumentType
    FX_TOUCH_OPTION: InstrumentType
    FX_VARIANCE_SWAP: InstrumentType
    NDF: InstrumentType

    # Equity
    EQUITY: InstrumentType
    EQUITY_OPTION: InstrumentType
    EQUITY_TOTAL_RETURN_SWAP: InstrumentType
    FI_INDEX_TOTAL_RETURN_SWAP: InstrumentType
    VARIANCE_SWAP: InstrumentType
    EQUITY_INDEX_FUTURE: InstrumentType
    VOLATILITY_INDEX_FUTURE: InstrumentType
    VOLATILITY_INDEX_OPTION: InstrumentType
    PRIVATE_MARKETS_FUND: InstrumentType
    REAL_ESTATE_ASSET: InstrumentType
    LEVERED_REAL_ESTATE_EQUITY: InstrumentType
    DCF: InstrumentType
    AUTOCALLABLE: InstrumentType
    CLIQUET_OPTION: InstrumentType

    # Exotics
    ASIAN_OPTION: InstrumentType
    BARRIER_OPTION: InstrumentType
    LOOKBACK_OPTION: InstrumentType
    QUANTO_OPTION: InstrumentType
    BASKET: InstrumentType

    # Commodity
    COMMODITY_FORWARD: InstrumentType
    COMMODITY_SWAP: InstrumentType
    COMMODITY_OPTION: InstrumentType
    COMMODITY_ASIAN_OPTION: InstrumentType

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
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...

class ModelKey:
    """Enumerates pricing model categories recognized by the registry.

    Examples:
        >>> from finstack.valuations.common import ModelKey
        >>> ModelKey.DISCOUNTING.name
        'discounting'
    """

    # Analytic / closed-form
    DISCOUNTING: ModelKey
    BLACK76: ModelKey
    NORMAL: ModelKey
    HULL_WHITE_1F: ModelKey
    HAZARD_RATE: ModelKey
    HESTON_FOURIER: ModelKey

    # Lattice
    TREE: ModelKey

    # Monte Carlo
    MONTE_CARLO_GBM: ModelKey
    MONTE_CARLO_HESTON: ModelKey
    MONTE_CARLO_HULL_WHITE_1F: ModelKey

    # Exotic closed-form
    BARRIER_BS_CONTINUOUS: ModelKey
    ASIAN_GEOMETRIC_BS: ModelKey
    ASIAN_TURNBULL_WAKEMAN: ModelKey
    LOOKBACK_BS_CONTINUOUS: ModelKey
    QUANTO_BS: ModelKey
    FX_BARRIER_BS_CONTINUOUS: ModelKey

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
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...

class PricerKey:
    """Composite key identifying a specific instrument/model pairing.

    Examples:
        >>> from finstack.valuations.common import InstrumentType, ModelKey, PricerKey
        >>> PricerKey(InstrumentType.BOND, ModelKey.DISCOUNTING)
        PricerKey(instrument='bond', model='discounting')
    """

    def __init__(self, instrument: InstrumentType | str, model: ModelKey | str) -> None:
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
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...

# Exported symbols for IDEs
__all__ = [
    "InstrumentType",
    "ModelKey",
    "PricerKey",
    "monte_carlo",
    "parse",
]
