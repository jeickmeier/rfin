"""
Term loan instrument with DDTL (Delayed Draw Term Loan) support.

A term loan is a debt instrument with a defined maturity, optional amortization,
and support for both fixed and floating rates. The DDTL variant allows for
delayed draws during an availability period with commitment fees and usage fees.
"""

from datetime import date
from typing import ClassVar

from finstack.core.currency import Currency
from finstack.core.money import Money

class TermLoan:
    """
    Term loan instrument with DDTL (Delayed Draw Term Loan) support.

    A term loan is a debt instrument with a defined maturity, optional amortization,
    and support for both fixed and floating rates. The DDTL variant allows for
    delayed draws during an availability period with commitment fees and usage fees.

    Examples:
        >>> from finstack.valuations.instruments import TermLoan
        >>> from finstack.core.money import Money
        >>> from datetime import date
        >>> # Create a simple fixed-rate term loan
        >>> loan = TermLoan.from_json('''{
        ...     "id": "loan_001",
        ...     "discount_curve_id": "usd_discount",
        ...     "currency": "USD",
        ...     "notional_limit": {"amount": 100000000.0, "currency": "USD"},
        ...     "issue": "2024-01-01",
        ...     "maturity": "2029-01-01",
        ...     "rate": {"Fixed": {"rate_bp": 500}},
        ...     "pay_freq": {"count": 3, "unit": "months"},
        ...     "day_count": "Act360",
        ...     "bdc": "following",
        ...     "calendar_id": null,
        ...     "stub": "None",
        ...     "amortization": "None",
        ...     "coupon_type": "Cash",
        ...     "upfront_fee": null,
        ...     "ddtl": null,
        ...     "covenants": null,
        ...     "pricing_overrides": {"adaptive_bumps": false, "spot_bump_pct": null, "vol_bump_pct": null, "rate_bump_bp": null},
        ...     "call_schedule": null,
        ...     "settlement_days": 1,
        ...     "attributes": {"meta": {}, "tags": []}
        ... }''')
        >>> loan.instrument_id
        'loan_001'
    """

    @classmethod
    def from_json(cls, json_str: str) -> TermLoan:
        """
        Create a term loan from a JSON string specification.

        The JSON should match the TermLoan schema from finstack-valuations.
        This is the recommended way to create complex term loans with DDTL features,
        covenants, and custom amortization schedules.

        Args:
            json_str: JSON string matching the TermLoan schema.

        Returns:
            Configured term loan instrument.

        Raises:
            ValueError: If JSON cannot be parsed or is invalid.
        """
        ...

    def to_json(self) -> str:
        """
        Serialize the term loan to a JSON string.

        Returns:
            JSON representation of the term loan.
        """
        ...

    @property
    def instrument_id(self) -> str:
        """
        Instrument identifier.

        Returns:
            Unique identifier assigned to the instrument.
        """
        ...

    @property
    def currency(self) -> Currency:
        """
        Currency for all cashflows.

        Returns:
            Currency object.
        """
        ...

    @property
    def notional_limit(self) -> Money:
        """
        Maximum commitment / notional limit.

        Returns:
            Notional limit as Money object.
        """
        ...

    @property
    def issue(self) -> date:
        """
        Issue (effective) date.

        Returns:
            Issue date.
        """
        ...

    @property
    def maturity(self) -> date:
        """
        Maturity date.

        Returns:
            Maturity date.
        """
        ...

    @property
    def discount_curve(self) -> str:
        """
        Discount curve identifier.

        Returns:
            Identifier for the discount curve.
        """
        ...

    @property
    def instrument_type(self) -> int:
        """
        Instrument type enum value.

        Returns:
            Enumeration value identifying the instrument family.
        """
        ...

    def __repr__(self) -> str: ...
