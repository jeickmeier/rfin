"""AmountOrScalar binding."""

from __future__ import annotations
from ...core.currency import Currency
from ...core.money import Money

class AmountOrScalar:
    """Union type for scalar values or currency amounts.

    Used in statement models to represent values that can be either:
    - Scalar: Dimensionless numbers (ratios, percentages, counts)
    - Amount: Currency-denominated values (Money)
    """

    @classmethod
    def scalar(cls, value: float) -> AmountOrScalar:
        """Create a scalar (dimensionless) value.

        Args:
            value: Numeric value

        Returns:
            AmountOrScalar: Scalar value
        """
        ...

    @classmethod
    def amount(cls, value: float, currency: Currency) -> AmountOrScalar:
        """Create a currency-denominated amount.

        Args:
            value: Numeric value
            currency: Currency code

        Returns:
            AmountOrScalar: Currency amount
        """
        ...

    @property
    def is_scalar(self) -> bool:
        """Check if this is a scalar value.

        Returns:
            bool: True if scalar, False if amount
        """
        ...

    @property
    def is_amount(self) -> bool:
        """Check if this is a currency amount.

        Returns:
            bool: True if amount, False if scalar
        """
        ...

    def as_money(self) -> Money | None:
        """Get the value as a Money object.

        Returns:
            Money | None: Money if this is an amount, None if scalar
        """
        ...

    @property
    def value(self) -> float:
        """Get the numeric value.

        Returns:
            float: Numeric value
        """
        ...

    @property
    def currency(self) -> Currency | None:
        """Get the currency if this is an amount.

        Returns:
            Currency | None: Currency if amount, None if scalar
        """
        ...

    def to_json(self) -> str:
        """Convert to JSON string.

        Returns:
            str: JSON representation
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> AmountOrScalar:
        """Create from JSON string.

        Args:
            json_str: JSON string

        Returns:
            AmountOrScalar: Deserialized value
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
