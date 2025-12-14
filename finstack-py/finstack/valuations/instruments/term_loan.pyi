"""Term loan instrument."""

from typing import Optional, Dict, Any
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ..common import InstrumentType

class TermLoan:
    """Term loan instrument with DDTL (Delayed Draw Term Loan) support.

    TermLoan represents a corporate loan with a fixed maturity and optional
    delayed draw features. Term loans are used for corporate financing and
    require discount curves and optionally credit curves for pricing.

    Term loans can include features like delayed drawdowns, amortization
    schedules, and prepayment options. They are typically defined via JSON
    specifications.

    Examples
    --------
    Create a term loan from JSON:

        >>> from finstack.valuations.instruments import TermLoan
        >>> json_str = '''
        ... {
        ...     "id": "TERM-LOAN-001",
        ...     "currency": "USD",
        ...     "notional_limit": {"amount": 10000000.0, "currency": "USD"},
        ...     "issue": "2024-01-01",
        ...     "maturity": "2029-01-01",
        ...     "rate": {"Fixed": {"rate_bp": 500}},
        ...     "bdc": "modified_following",
        ...     "day_count": "Act360",
        ...     "pay_freq": {"count": 3, "unit": "months"},
        ...     "stub": "None",
        ...     "discount_curve_id": "USD-OIS",
        ...     "pricing_overrides": {"adaptive_bumps": false},
        ...     "attributes": {"meta": {}, "tags": []}
        ... }
        ... '''
        >>> term_loan = TermLoan.from_json(json_str)

    Notes
    -----
    - Term loans require discount curve and optionally credit curve
    - Can include delayed draw term loan (DDTL) features
    - Amortization schedules can be specified
    - Prepayment options affect cashflow timing
    - Typically defined via JSON for complex structures

    See Also
    --------
    :class:`RevolvingCredit`: Revolving credit facilities
    :class:`Bond`: Bonds
    :class:`PricerRegistry`: Pricing entry point
    """

    @classmethod
    def from_json(cls, json_str: str) -> "TermLoan": ...
    """Create a term loan from a JSON string specification.

    Parameters
    ----------
    json_str : str
        JSON string containing term loan specification. Must include
        instrument_id, notional_limit, issue, maturity, and discount_curve.

    Returns
    -------
    TermLoan
        Configured term loan ready for pricing.

    Raises
    ------
    ValueError
        If JSON is invalid or required fields are missing.

    Examples
    --------
        >>> term_loan = TermLoan.from_json(json_str)
        >>> term_loan.notional_limit
        Money(10000000, Currency("USD"))
    """

    def to_json(self) -> str:
        """Serialize the term loan to a JSON string."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def notional_limit(self) -> Money: ...
    @property
    def issue(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
