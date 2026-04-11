"""ISO-4217 currency bindings from ``finstack-core``.

Provides the :class:`Currency` type for representing ISO-4217 currencies with
numeric codes and minor-unit precision. Module-level constants are provided for
every supported currency (e.g. ``USD``, ``EUR``, ``GBP``).

Example::

    >>> from finstack.core.currency import Currency, USD
    >>> usd = Currency("USD")
    >>> usd.code
    'USD'
    >>> usd.numeric
    840
    >>> usd == USD
    True
"""

from __future__ import annotations

__all__ = [
    "Currency",
    "AED",
    "AFN",
    "ALL",
    "AMD",
    "ANG",
    "AOA",
    "ARS",
    "AUD",
    "AWG",
    "AZN",
    "BAM",
    "BBD",
    "BDT",
    "BGN",
    "BHD",
    "BIF",
    "BMD",
    "BND",
    "BOB",
    "BRL",
    "BSD",
    "BTN",
    "BWP",
    "BYN",
    "BZD",
    "CAD",
    "CDF",
    "CHF",
    "CLP",
    "CNY",
    "COP",
    "CRC",
    "CUP",
    "CVE",
    "CZK",
    "DJF",
    "DKK",
    "DOP",
    "DZD",
    "EGP",
    "ERN",
    "ETB",
    "EUR",
    "FJD",
    "FKP",
    "GBP",
    "GEL",
    "GHS",
    "GIP",
    "GMD",
    "GNF",
    "GTQ",
    "GYD",
    "HKD",
    "HNL",
    "HRK",
    "HTG",
    "HUF",
    "IDR",
    "ILS",
    "INR",
    "IQD",
    "IRR",
    "ISK",
    "JMD",
    "JOD",
    "JPY",
    "KES",
    "KGS",
    "KHR",
    "KMF",
    "KPW",
    "KRW",
    "KWD",
    "KYD",
    "KZT",
    "LAK",
    "LBP",
    "LKR",
    "LRD",
    "LSL",
    "LYD",
    "MAD",
    "MDL",
    "MGA",
    "MKD",
    "MMK",
    "MNT",
    "MOP",
    "MRU",
    "MUR",
    "MVR",
    "MWK",
    "MXN",
    "MYR",
    "MZN",
    "NAD",
    "NGN",
    "NIO",
    "NOK",
    "NPR",
    "NZD",
    "OMR",
    "PAB",
    "PEN",
    "PGK",
    "PHP",
    "PKR",
    "PLN",
    "PYG",
    "QAR",
    "RON",
    "RSD",
    "RUB",
    "RWF",
    "SAR",
    "SBD",
    "SCR",
    "SDG",
    "SEK",
    "SGD",
    "SHP",
    "SLE",
    "SOS",
    "SRD",
    "SSP",
    "STN",
    "SVC",
    "SYP",
    "SZL",
    "THB",
    "TJS",
    "TMT",
    "TND",
    "TOP",
    "TRY",
    "TTD",
    "TWD",
    "TZS",
    "UAH",
    "UGX",
    "USD",
    "UYU",
    "UZS",
    "VES",
    "VND",
    "VUV",
    "WST",
    "XAF",
    "XCD",
    "XOF",
    "XPF",
    "YER",
    "ZAR",
    "ZMW",
    "ZWL",
]

class Currency:
    """An ISO-4217 currency.

    Immutable, hashable value type representing a single ISO-4217 currency.
    Supports comparison with other ``Currency`` instances and with ISO
    alphabetic code strings.

    Parameters
    ----------
    code : str
        Three-letter ISO-4217 alphabetic code (case-insensitive).

    Raises
    ------
    ValueError
        If *code* is not a recognised ISO-4217 currency.

    Examples
    --------
    >>> from finstack.core.currency import Currency
    >>> eur = Currency("EUR")
    >>> eur.code
    'EUR'
    >>> eur.numeric
    978
    >>> eur.decimals
    2
    >>> eur == "EUR"
    True
    """

    def __init__(self, code: str) -> None:
        """Parse an ISO-4217 alphabetic code (case-insensitive).

        Parameters
        ----------
        code : str
            Three-letter ISO-4217 alphabetic code.

        Raises
        ------
        ValueError
            If *code* is not a recognised currency.
        """
        ...

    @classmethod
    def from_numeric(cls, code: int) -> Currency:
        """Construct from an ISO-4217 numeric code.

        Parameters
        ----------
        code : int
            ISO-4217 numeric identifier (e.g. ``840`` for USD).

        Returns
        -------
        Currency
            The matching currency.

        Raises
        ------
        ValueError
            If *code* does not map to a known currency.
        """
        ...

    @property
    def code(self) -> str:
        """Three-letter ISO-4217 alphabetic code (uppercase).

        Returns
        -------
        str
        """
        ...

    @property
    def numeric(self) -> int:
        """ISO-4217 numeric identifier.

        Returns
        -------
        int
        """
        ...

    @property
    def decimals(self) -> int:
        """Typical number of decimal places (minor units) for this currency.

        Returns
        -------
        int
        """
        ...

    def to_json(self) -> str:
        """Serialize this currency to a JSON string.

        Returns
        -------
        str
            JSON representation.

        Raises
        ------
        ValueError
            If serialization fails.
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> Currency:
        """Deserialize a currency from a JSON string.

        Parameters
        ----------
        json : str
            JSON payload.

        Returns
        -------
        Currency
            The deserialized currency.

        Raises
        ------
        ValueError
            If *json* is not valid or does not represent a known currency.
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...
    def __lt__(self, other: object) -> bool: ...
    def __le__(self, other: object) -> bool: ...
    def __gt__(self, other: object) -> bool: ...
    def __ge__(self, other: object) -> bool: ...

# Module-level currency constants (a representative subset is shown;
# every ISO-4217 currency from finstack-core is available).
USD: Currency
EUR: Currency
GBP: Currency
JPY: Currency
CHF: Currency
CAD: Currency
AUD: Currency
NZD: Currency
CNY: Currency
HKD: Currency
SGD: Currency
SEK: Currency
NOK: Currency
DKK: Currency
INR: Currency
BRL: Currency
MXN: Currency
ZAR: Currency
KRW: Currency
TRY: Currency
PLN: Currency
THB: Currency
IDR: Currency
MYR: Currency
PHP: Currency
TWD: Currency
CZK: Currency
HUF: Currency
ILS: Currency
CLP: Currency
COP: Currency
PEN: Currency
ARS: Currency
RUB: Currency
SAR: Currency
AED: Currency
KWD: Currency
BHD: Currency
OMR: Currency
QAR: Currency
JOD: Currency
EGP: Currency
NGN: Currency
KES: Currency
GHS: Currency
TZS: Currency
UGX: Currency
RON: Currency
BGN: Currency
HRK: Currency
RSD: Currency
ISK: Currency
