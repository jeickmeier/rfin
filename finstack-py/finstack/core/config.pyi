"""Configuration bindings for rounding policies and currency scales.

Provides a Python-facing FinstackConfig to manage global rounding behavior
and per-currency decimal scales for both ingestion and presentation. Use this
to control how Money values are parsed and formatted throughout analyses.
Also exposes a RoundingMode enum with common strategies (bankers, floor,
ceil, toward/away from zero).
"""

from __future__ import annotations
from typing import Dict, Any
from .currency import Currency

class RoundingMode:
    """Rounding strategy for decimal arithmetic.

    Available modes:
    - BANKERS: Round to nearest even (default)
    - FLOOR: Round toward negative infinity
    - CEIL: Round toward positive infinity
    - TOWARD_ZERO: Round toward zero
    - AWAY_FROM_ZERO: Round away from zero

    Examples
    --------
        >>> from finstack.core.config import RoundingMode
        >>> mode = RoundingMode.BANKERS
        >>> mode.name
        'bankers'
    """

    BANKERS: "RoundingMode"
    """Banker's rounding (round to nearest even)."""
    FLOOR: "RoundingMode"
    """Round toward negative infinity."""
    CEIL: "RoundingMode"
    """Round toward positive infinity."""
    TOWARD_ZERO: "RoundingMode"
    """Round toward zero (truncate)."""
    AWAY_FROM_ZERO: "RoundingMode"
    """Round away from zero."""

    @classmethod
    def from_name(cls, name: str) -> "RoundingMode":
        """Create from string name.

        Parameters
        ----------
        name : str
            Rounding mode name (case-insensitive). Valid values:
            "bankers", "floor", "ceil", "toward_zero", "away_from_zero".

        Returns
        -------
        RoundingMode
            Rounding mode instance.

        Raises
        ------
        ValueError
            If name is not recognized.
        """
        ...

    @property
    def name(self) -> str:
        """Get the mode name.

        Returns
        -------
        str
            Human-readable mode name (e.g., "bankers", "floor").
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...

class CurrencyScalePolicy:
    """Policy mapping that determines decimal places for each currency.

    The policy stores currency-specific overrides. Currencies without
    overrides use their ISO-4217 default decimal places.

    Parameters
    ----------
    overrides : dict[str, int], optional
        Mapping from currency code to decimal places.

    Examples
    --------
        >>> from finstack.core.config import CurrencyScalePolicy
        >>> policy = CurrencyScalePolicy({"JPY": 0, "USD": 4})
        >>> policy.overrides
        {'JPY': 0, 'USD': 4}
    """

    def __init__(self, overrides: Dict[str, int] | None = None) -> None: ...
    @property
    def overrides(self) -> Dict[str, int]:
        """Get the currency scale overrides.

        Returns
        -------
        dict[str, int]
            Mapping from currency code to decimal places.
        """
        ...

class RoundingPolicy:
    """Full rounding policy used at IO boundaries and normalization steps.

    Combines the rounding mode with ingest and output scale policies.

    Parameters
    ----------
    mode : RoundingMode or str, optional
        Rounding mode to use.
    ingest_scale : CurrencyScalePolicy or dict, optional
        Scale policy for ingesting values.
    output_scale : CurrencyScalePolicy or dict, optional
        Scale policy for output formatting.

    Examples
    --------
        >>> from finstack.core.config import RoundingPolicy, RoundingMode
        >>> policy = RoundingPolicy(mode=RoundingMode.FLOOR, ingest_scale={"JPY": 0}, output_scale={"USD": 4})
    """

    def __init__(
        self,
        *,
        mode: str | RoundingMode | None = None,
        ingest_scale: CurrencyScalePolicy | Dict[str, int] | None = None,
        output_scale: CurrencyScalePolicy | Dict[str, int] | None = None,
    ) -> None: ...
    @property
    def mode(self) -> RoundingMode:
        """Active rounding mode."""
        ...

    @property
    def ingest_scale(self) -> CurrencyScalePolicy:
        """Scale policy for ingesting values."""
        ...

    @property
    def output_scale(self) -> CurrencyScalePolicy:
        """Scale policy for output formatting."""
        ...

class ZeroKind:
    """Zero-kind classification for tolerance checks.

    Different types of values require different epsilon tolerances when
    checking for effectively-zero values:
    - Money: Use currency-specific scale
    - Rate: Use rate epsilon (1e-12)
    - Generic: Use generic epsilon (1e-10)

    Examples
    --------
        >>> from finstack.core.config import ZeroKind
        >>> ZeroKind.RATE
        ZeroKind.RATE
        >>> money_kind = ZeroKind.money("USD")
        >>> money_kind.kind
        'money'
    """

    GENERIC: "ZeroKind"
    """Generic floating-point comparisons."""
    RATE: "ZeroKind"
    """Interest rates or small numeric ratios."""

    @classmethod
    def money(cls, currency: str | Currency) -> "ZeroKind":
        """Create a money zero-kind for the specified currency.

        Parameters
        ----------
        currency : str or Currency
            Currency for the money tolerance.

        Returns
        -------
        ZeroKind
            Money zero-kind instance.
        """
        ...

    @property
    def kind(self) -> str:
        """Get the kind name ("money", "rate", or "generic")."""
        ...

    def __repr__(self) -> str: ...

class NumericMode:
    """Numeric engine mode compiled into the crate.

    Currently only F64 mode is supported.
    """

    F64: "NumericMode"
    """Floating-point f64 engine."""

    def __repr__(self) -> str: ...

class RoundingContext:
    """Snapshot of active rounding settings for result stamping.

    Contains the rounding configuration at the time of computation,
    used for audit trails and reproducibility.

    Examples
    --------
        >>> from finstack.core.config import FinstackConfig
        >>> cfg = FinstackConfig()
        >>> ctx = cfg.rounding_context()
        >>> ctx.mode.name
        'bankers'
    """

    @property
    def mode(self) -> RoundingMode:
        """Active rounding mode."""
        ...

    @property
    def version(self) -> int:
        """Schema version for forward compatibility."""
        ...

    @property
    def ingest_scale_by_currency(self) -> Dict[str, int]:
        """Ingest scale overrides by currency code."""
        ...

    @property
    def output_scale_by_currency(self) -> Dict[str, int]:
        """Output scale overrides by currency code."""
        ...

    def output_scale(self, currency: str | Currency) -> int:
        """Effective output scale for a currency.

        Parameters
        ----------
        currency : str or Currency
            Currency to query.

        Returns
        -------
        int
            Decimal places for output.
        """
        ...

    def money_epsilon(self, currency: str | Currency) -> float:
        """Money epsilon derived from the currency output scale.

        Half of one unit in the last place at the configured scale.

        Parameters
        ----------
        currency : str or Currency
            Currency to query.

        Returns
        -------
        float
            Epsilon value for the currency.
        """
        ...

    def is_effectively_zero_money(self, amount: float, currency: str | Currency) -> bool:
        """Check if a money amount is effectively zero under this context.

        Parameters
        ----------
        amount : float
            Amount to check.
        currency : str or Currency
            Currency of the amount.

        Returns
        -------
        bool
            True if the amount is within the currency's epsilon.
        """
        ...

    def is_effectively_zero(self, value: float, kind: ZeroKind) -> bool:
        """Check if a value is effectively zero for the specified kind.

        Parameters
        ----------
        value : float
            Value to check.
        kind : ZeroKind
            Type of zero comparison.

        Returns
        -------
        bool
            True if the value is within the kind's tolerance.
        """
        ...

class ResultsMeta:
    """Metadata bundle that accompanies valuation outputs.

    The metadata is intentionally small so it can be attached to reports
    and downstream data stores for reproducibility and audit trails.

    Examples
    --------
        >>> from finstack.core.config import FinstackConfig
        >>> cfg = FinstackConfig()
        >>> meta = cfg.results_meta()
        >>> meta.numeric_mode
        NumericMode.F64
    """

    @property
    def numeric_mode(self) -> NumericMode:
        """Numeric engine mode used to produce the results."""
        ...

    @property
    def rounding(self) -> RoundingContext:
        """Rounding context snapshot applied to IO boundaries."""
        ...

    @property
    def fx_policy_applied(self) -> str | None:
        """Optional FX policy applied by the computing layer."""
        ...

    @property
    def timestamp(self) -> str | None:
        """Timestamp when result was computed (ISO 8601 format)."""
        ...

    @property
    def version(self) -> str | None:
        """Finstack library version used to produce the result."""
        ...

    def __repr__(self) -> str: ...

class FinstackConfig:
    """Global configuration for rounding policies and currency decimal scales.

    FinstackConfig controls how monetary values are rounded during ingestion
    and formatted during output. It also manages per-currency decimal place
    settings, allowing fine-grained control over precision for different
    currencies.

    The configuration is mutable and can be modified after creation.
    Default settings use Bankers rounding and ISO-4217 standard decimal places.

    Parameters
    ----------
    None
        Construct via ``FinstackConfig()`` to use default rounding rules
        (Bankers rounding, ISO-4217 decimal places).

    Returns
    -------
    FinstackConfig
        Configuration instance that can be reused across money operations.

    Examples
    --------
        >>> from finstack.core.config import FinstackConfig
        >>> cfg = FinstackConfig()
        >>> cfg.set_rounding_mode("floor")
        >>> cfg.set_ingest_scale("JPY", 4)
        >>> cfg.set_output_scale("USD", 4)
        >>> print((cfg.rounding_mode.name, cfg.ingest_scale("JPY"), cfg.output_scale("USD")))
        ('floor', 4, 4)

    Notes
    -----
    - Configuration changes affect all subsequent operations using that config
    - Default rounding mode is Bankers (round to nearest even)
    - Default decimal scales follow ISO-4217 standard
    - Use :meth:`copy` to create independent configurations
    - Ingest scale controls precision when creating Money from floats
    - Output scale controls precision when formatting Money to strings

    See Also
    --------
    :class:`RoundingMode`: Available rounding strategies
    :class:`Money`: Money formatting with configuration
    """

    def __init__(self) -> None: ...
    def copy(self) -> "FinstackConfig":
        """Create a copy of this configuration.

        Returns
        -------
        FinstackConfig
            Independent copy of the configuration.
        """
        ...

    @property
    def rounding_mode(self) -> RoundingMode:
        """Get the current rounding mode.

        Returns
        -------
        RoundingMode
            Active rounding strategy.
        """
        ...

    def set_rounding_mode(self, mode: str | RoundingMode) -> None:
        """Set the global rounding mode for decimal arithmetic.

        Parameters
        ----------
        mode : str or RoundingMode
            New rounding mode. Can be a string (case-insensitive) or a
            :class:`RoundingMode` instance. Valid strings: "bankers", "floor",
            "ceil", "toward_zero", "away_from_zero".

        Raises
        ------
        ValueError
            If the mode string is not recognized.

        Examples
        --------
            >>> from finstack.core.config import FinstackConfig
            >>> cfg = FinstackConfig()
            >>> cfg.set_rounding_mode("ceil")
            >>> cfg.rounding_mode.name
            'ceil'
        """
        ...

    def ingest_scale(self, currency: str | Currency) -> int:
        """Get the ingest scale for a currency.

        Parameters
        ----------
        currency : str or Currency
            Currency to query.

        Returns
        -------
        int
            Number of decimal places for ingest.
        """
        ...

    def set_ingest_scale(self, currency: str | Currency, decimals: int) -> None:
        """Set the number of decimal places used when creating Money from floats.

        The ingest scale controls how many decimal places are preserved when
        converting a float to a Money value. This affects precision during
        monetary operations.

        Parameters
        ----------
        currency : str or Currency
            Currency to configure (e.g., "USD", "JPY").
        decimals : int
            Number of decimal places to preserve (must be >= 0).

        Examples
        --------
            >>> from finstack.core.config import FinstackConfig
            >>> cfg = FinstackConfig()
            >>> cfg.set_ingest_scale("JPY", 4)
            >>> cfg.ingest_scale("JPY")
            4
        """
        ...

    def output_scale(self, currency: str | Currency) -> int:
        """Get the output scale for a currency.

        Parameters
        ----------
        currency : str or Currency
            Currency to query.

        Returns
        -------
        int
            Number of decimal places for output.
        """
        ...

    def set_output_scale(self, currency: str | Currency, decimals: int) -> None:
        """Set the number of decimal places used when formatting Money to strings.

        The output scale controls how many decimal places are shown when converting
        a Money value to a string representation. This affects display formatting
        but does not change the underlying precision.

        Parameters
        ----------
        currency : str or Currency
            Currency to configure (e.g., "USD", "JPY").
        decimals : int
            Number of decimal places to display (must be >= 0).

        Examples
        --------
            >>> from finstack.core.config import FinstackConfig
            >>> from finstack.core.money import Money
            >>> cfg = FinstackConfig()
            >>> cfg.set_output_scale("USD", 4)
            >>> Money(100.123456, "USD").format_with_config(cfg)
            'USD 100.1235'
        """
        ...

    @property
    def rounding_policy(self) -> RoundingPolicy:
        """Get the full rounding policy (mode plus ingest/output overrides)."""
        ...

    def rounding_context(self) -> RoundingContext:
        """Build an immutable rounding context snapshot from this configuration.

        Returns
        -------
        RoundingContext
            Snapshot of current rounding settings.
        """
        ...

    def results_meta(self) -> ResultsMeta:
        """Build a results metadata snapshot from this configuration.

        Returns
        -------
        ResultsMeta
            Metadata bundle for result stamping.
        """
        ...

    @property
    def numeric_mode(self) -> NumericMode:
        """Numeric mode compiled into the core crate."""
        ...

    def set_extension(self, key: str, value: Any) -> None:
        """Set an extension section in the configuration.

        Parameters
        ----------
        key : str
            Extension key (e.g., "valuations.calibration.v2")
        value : Any
            Extension configuration value (must be JSON-serializable).
        """
        ...

# Module-level constant
NUMERIC_MODE: NumericMode
"""Active numeric mode used by the engine (always F64)."""

def rounding_context_from(config: FinstackConfig) -> RoundingContext:
    """Build a rounding context snapshot from a configuration.

    Parameters
    ----------
    config : FinstackConfig
        Configuration to snapshot.

    Returns
    -------
    RoundingContext
        Snapshot of the rounding settings.
    """
    ...

def results_meta(config: FinstackConfig) -> ResultsMeta:
    """Build a results metadata snapshot from a configuration.

    Parameters
    ----------
    config : FinstackConfig
        Configuration to snapshot.

    Returns
    -------
    ResultsMeta
        Metadata bundle for result stamping.
    """
    ...

__all__ = [
    "CurrencyScalePolicy",
    "RoundingPolicy",
    "FinstackConfig",
    "RoundingMode",
    "ZeroKind",
    "RoundingContext",
    "NumericMode",
    "NUMERIC_MODE",
    "ResultsMeta",
    "rounding_context_from",
    "results_meta",
]
