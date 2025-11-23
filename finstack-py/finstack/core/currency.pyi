"""Currency bindings: ISO-4217 metadata and helpers for Python.

Exposes Currency as a Python class with convenient constructors and
properties. All arithmetic using Money requires matching currencies; this
module provides the canonical way to identify currencies via codes or
numeric identifiers.
"""

from typing import List, Tuple

class Currency:
    """ISO-4217 currency identifier with metadata and validation.

    Currency objects provide type-safe currency identification throughout
    finstack. All monetary arithmetic requires matching currencies, preventing
    accidental cross-currency operations. Currency instances are immutable,
    hashable, and can be used as dictionary keys.

    Parameters
    ----------
    code : str
        Three-letter ISO-4217 currency code (case-insensitive).
        Examples: "USD", "eur", "GBP", "JPY".

    Returns
    -------
    Currency
        Immutable currency object with code, numeric identifier, and decimal
        places metadata.

    Raises
    ------
    ValueError
        If the currency code is not recognized as a valid ISO-4217 code.

    Examples
    --------
    Examples
    --------
        >>> from finstack.core.currency import Currency
        >>> usd = Currency("USD")
        >>> eur = Currency.from_numeric(978)
        >>> print((usd.code, usd.numeric, usd.decimals, usd == Currency("usd"), eur.code))
        ('USD', 840, 2, True, 'EUR')

    Notes
    -----
    - Currency codes are normalized to uppercase internally
    - Currency objects are immutable and can be safely shared
    - All supported currencies follow ISO-4217 standard
    - Use :meth:`from_numeric` to construct from numeric codes
    - Use :meth:`all` to list all available currencies

    See Also
    --------
    :class:`Money`: Currency-tagged monetary amounts
    :class:`finstack.core.money.Money`: Money arithmetic with currency safety
    """

    def __init__(self, code: str) -> None: ...
    @classmethod
    def from_numeric(cls, numeric: int) -> Currency: ...
    """Construct a Currency from an ISO-4217 numeric code.
    
    Parameters
    ----------
    numeric : int
        ISO-4217 numeric currency code (e.g., 840 for USD, 978 for EUR).
        
    Returns
    -------
    Currency
        Currency instance associated with the numeric code.
        
    Raises
    ------
    ValueError
        If the numeric code is not recognized as a valid ISO-4217 code.
        
    Examples
    --------
        >>> Currency.from_numeric(840)  # USD
        Currency("USD")
        >>> Currency.from_numeric(978)  # EUR
        Currency("EUR")
        >>> Currency.from_numeric(392)  # JPY
        Currency("JPY")
    """

    @property
    def code(self) -> str: ...
    """Three-letter currency code (always upper-case).
    
    Returns
    -------
    str
        Upper-case ISO code (e.g. "USD").
    """

    @property
    def numeric(self) -> int: ...
    """ISO numeric currency code.
    
    Returns
    -------
    int
        Numeric ISO identifier (e.g. 840 for USD).
    """

    @property
    def decimals(self) -> int: ...
    """Number of decimal places for this currency.
    
    Returns
    -------
    int
        Decimal places (e.g. 2 for USD, 0 for JPY).
    """

    def to_tuple(self) -> Tuple[str, int, int]: ...
    """Convert to (code, numeric, decimals) tuple.
    
    Returns
    -------
    Tuple[str, int, int]
        (code, numeric, decimals) representation.
    """

    @classmethod
    def all(cls) -> List[Currency]: ...
    """Get all available ISO-4217 currencies.
    
    Returns
    -------
    List[Currency]
        List of all supported currencies, ordered by code.
        
    Examples
    --------
        >>> len(Currency.all()) > 100
        True
    """

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...

# Currency constants
USD: Currency
EUR: Currency
GBP: Currency
JPY: Currency
CHF: Currency
CAD: Currency
AUD: Currency
NZD: Currency
SEK: Currency
NOK: Currency
DKK: Currency
PLN: Currency
CZK: Currency
HUF: Currency
RUB: Currency
BRL: Currency
MXN: Currency
ZAR: Currency
KRW: Currency
SGD: Currency
HKD: Currency
CNY: Currency
INR: Currency
TRY: Currency
ILS: Currency
AED: Currency
SAR: Currency
QAR: Currency
KWD: Currency
BHD: Currency
OMR: Currency
JOD: Currency
LBP: Currency
EGP: Currency
MAD: Currency
TND: Currency
DZD: Currency
LYD: Currency
SDG: Currency
ETB: Currency
KES: Currency
UGX: Currency
TZS: Currency
ZMW: Currency
BWP: Currency
SZL: Currency
LSL: Currency
NAD: Currency
MZN: Currency
AOA: Currency
GHS: Currency
NGN: Currency
XOF: Currency
XAF: Currency
XPF: Currency
