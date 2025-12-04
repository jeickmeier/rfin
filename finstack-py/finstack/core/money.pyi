"""Money bindings: currency-tagged amounts with safe arithmetic and FX conversion."""

from typing import Union, Tuple, Optional
from datetime import date
from .currency import Currency
from .market_data.fx import FxMatrix, FxConversionPolicy

class Money:
    """Currency-tagged monetary amount with type-safe arithmetic.

    Money represents a monetary value with an associated currency. All arithmetic
    operations require matching currencies, preventing accidental cross-currency
    calculations. Money instances are immutable and respect currency-specific
    decimal places for formatting.

    Parameters
    ----------
    amount : float
        Scalar monetary value. The interpretation depends on the currency's
        decimal places (e.g., 100.50 for USD represents $100.50, while 100 for
        JPY represents ¥100).
    currency : str or Currency
        Currency identifier. Can be an ISO code string (e.g., "USD") or a
        :class:`Currency` instance.

    Returns
    -------
    Money
        Immutable money object supporting arithmetic, formatting, and
        conversions.

    Raises
    ------
    ValueError
        If the currency code is invalid or if arithmetic operations involve
        mismatched currencies.

    Examples
    --------
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> usd = Money(100.50, "USD")
        >>> eur = Money(50.0, Currency("EUR"))
        >>> total = usd + Money(20.0, "USD")
        >>> print((usd.format(), total.amount, eur.currency.code))
        ('USD 100.50', 120.5, 'EUR')

    Notes
    -----
    - Money objects are immutable and hashable
    - All arithmetic requires matching currencies (enforced at runtime)
    - Formatting respects currency-specific decimal places
    - Use :meth:`from_config` to control ingest rounding behavior
    - Use :meth:`zero` to create zero amounts in a specific currency

    See Also
    --------
    :class:`Currency`: Currency identification and metadata
    :class:`FinstackConfig`: Configuration for rounding and decimal scales
    """

    def __init__(self, amount: float, currency: Union[str, Currency]) -> None: ...
    @classmethod
    def from_config(cls, amount: float, currency: Union[str, Currency], config: "FinstackConfig") -> Money: ...
    """Construct a money value using a configuration for ingest rounding.
    
    Parameters
    ----------
    amount : float
        Raw monetary value.
    currency : Currency or str
        Currency identifier.
    config : FinstackConfig
        Configuration controlling ingest rounding/scale.
        
    Returns
    -------
    Money
        Money instance respecting custom ingest rules.
        
    Examples
    --------
        >>> from finstack.core.config import FinstackConfig
        >>> from finstack.core.money import Money
        >>> cfg = FinstackConfig()
        >>> cfg.set_ingest_scale("JPY", 4)
        >>> Money.from_config(123.4567, "JPY", cfg).amount
        123.4567
    """

    @classmethod
    def zero(cls, currency: Union[str, Currency]) -> Money: ...
    """Create a zero amount in the specified currency.
    
    Parameters
    ----------
    currency : Currency or str
        Currency for the zero amount.
        
    Returns
    -------
    Money
        Zero amount in the specified currency.
    """

    @classmethod
    def from_tuple(cls, value: Tuple[float, Currency]) -> Money: ...
    """Construct from a (amount, currency) tuple.
    
    Parameters
    ----------
    value : Tuple[float, Currency]
        (amount, currency) tuple.
        
    Returns
    -------
    Money
        Money instance from tuple representation.
    """

    @property
    def amount(self) -> float: ...
    """Get the numeric amount.
    
    Returns
    -------
    float
        Scalar value in the currency's minor units.
    """

    @property
    def currency(self) -> Currency: ...
    """Get the currency.
    
    Returns
    -------
    Currency
        Currency instance.
    """

    def to_tuple(self) -> Tuple[float, Currency]: ...
    """Convert to (amount, currency) tuple.
    
    Returns
    -------
    Tuple[float, Currency]
        Tuple representation.
    """

    def format(self) -> str: ...
    """Format as a human-readable string.
    
    Returns
    -------
    str
        Formatted string (e.g. "USD 125.50").
    """

    def format_with_config(self, config: "FinstackConfig") -> str: ...
    """Format using custom configuration.
    
    Parameters
    ----------
    config : FinstackConfig
        Configuration for formatting rules.
        
    Returns
    -------
    str
        Formatted string respecting config rules.
    """

    def format_custom(self, decimals: int, show_currency: bool = True) -> str: ...
    """Format with explicit decimal places and optional currency code."""

    def format_with_separators(self, decimals: int) -> str: ...
    """Format with thousands separators and explicit decimal places."""

    def convert(
        self,
        to_currency: Union[str, Currency],
        on: "date",
        fx_matrix: FxMatrix,
        policy: Optional[Union[str, FxConversionPolicy]] = None,
    ) -> Money: ...
    """Convert this amount into another currency using an FX matrix.

    Parameters
    ----------
    to_currency : str or Currency
        Target currency.
    on : date
        Valuation date for the conversion.
    fx_matrix : FxMatrix
        FX source used to obtain rates.
    policy : FxConversionPolicy or str, optional
        Conversion timing policy (defaults to cashflow_date).
    """

    def checked_add(self, other: Money) -> Money: ...
    """Add another money amount with explicit currency checking.
    
    This method performs addition with explicit error handling. For most use
    cases, the ``+`` operator is preferred, which calls this method internally.
    
    Parameters
    ----------
    other : Money
        Money amount to add. Must have the same currency as this instance.
        
    Returns
    -------
    Money
        Sum of the two amounts in the same currency.
        
    Raises
    ------
    ValueError
        If currencies don't match. The error message will indicate which
        currencies were involved.
    """

    def checked_sub(self, other: Money) -> Money: ...
    """Subtract another money amount with explicit currency checking.
    
    This method performs subtraction with explicit error handling. For most use
    cases, the ``-`` operator is preferred, which calls this method internally.
    
    Parameters
    ----------
    other : Money
        Money amount to subtract. Must have the same currency as this instance.
        
    Returns
    -------
    Money
        Difference of the two amounts in the same currency.
        
    Raises
    ------
    ValueError
        If currencies don't match. The error message will indicate which
        currencies were involved.
    """

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...
    def __add__(self, other: Money) -> Money: ...
    def __radd__(self, other: Money) -> Money: ...
    def __sub__(self, other: Money) -> Money: ...
    def __rsub__(self, other: Money) -> Money: ...
    def __mul__(self, factor: float) -> Money: ...
    def __rmul__(self, factor: float) -> Money: ...
    def __truediv__(self, divisor: float) -> Money: ...
    def __rtruediv__(self, value: float) -> None: ...
    def __getnewargs__(self) -> Tuple[float, Currency]: ...
