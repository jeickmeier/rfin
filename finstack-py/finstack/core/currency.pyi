"""Currency bindings: ISO-4217 metadata and helpers for Python.

Exposes Currency as a Python class with convenient constructors and
properties. All arithmetic using Money requires matching currencies; this
module provides the canonical way to identify currencies via codes or
numeric identifiers.
"""

from typing import List, Tuple

class Currency:
    """Wrap ISO-4217 currency metadata for Python usage.

    Parameters
    ----------
    code : str
        Three-letter ISO code such as "USD" or "eur".

    Returns
    -------
    Currency
        Strongly typed currency object used throughout the bindings.
    """

    def __init__(self, code: str) -> None: ...
    @classmethod
    def from_numeric(cls, numeric: int) -> Currency: ...
    """Construct from an ISO numeric currency code (e.g. 840 → USD).
    
    Parameters
    ----------
    numeric : int
        ISO-4217 numeric currency code.
        
    Returns
    -------
    Currency
        Currency instance associated with numeric.
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
    """Get all available currencies.
    
    Returns
    -------
    List[Currency]
        All supported ISO-4217 currencies.
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
