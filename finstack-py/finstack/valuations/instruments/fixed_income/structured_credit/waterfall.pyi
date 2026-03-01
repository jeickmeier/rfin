"""Structured credit waterfall engine bindings."""

from __future__ import annotations
from enum import IntEnum

class AllocationMode(IntEnum):
    """Allocation mode within a tier.

    Attributes:
        Sequential: Pay recipients in order until tier allocation exhausted
        ProRata: Distribute proportionally by weight or equally
    """

    Sequential = 0
    ProRata = 1

class PaymentType(IntEnum):
    """Payment type classification.

    Attributes:
        Fee: Fee payment
        Interest: Interest payment
        Principal: Principal payment
        Residual: Residual/equity distribution
    """

    Fee = 0
    Interest = 1
    Principal = 2
    Residual = 3

class WaterfallTier:
    """Waterfall tier with multiple recipients.

    A tier groups related payments with a priority level and allocation mode.

    Args:
        tier_id: Unique tier identifier
        priority: Priority order (lower = higher priority)
        payment_type: Type of payment (Fee, Interest, Principal, Residual)

    Examples:
        >>> from finstack.valuations.instruments import AllocationMode, PaymentType, WaterfallTier
        >>> tier = WaterfallTier("fees", 1, PaymentType.Fee)
        >>> tier.add_fixed_fee("trustee", "Trustee", 50_000.0, "USD")
        >>> tier.set_allocation_mode(AllocationMode.Sequential)
        >>> tier.recipient_count
        1
    """

    def __init__(self, tier_id: str, priority: int, payment_type: PaymentType) -> None: ...
    def add_recipient(self, recipient_id: str, recipient_type: str, calculation: str) -> "WaterfallTier":
        """Add a recipient to this tier.

        Args:
            recipient_id: Unique recipient identifier
            recipient_type: Type of recipient (JSON format)
            calculation: Payment calculation (JSON format)

        Returns:
            Self for method chaining
        """
        ...

    def add_fixed_fee(self, recipient_id: str, provider_name: str, amount: float, currency: str) -> "WaterfallTier":
        """Add a fixed fee recipient.

        Args:
            recipient_id: Unique recipient identifier
            provider_name: Service provider name
            amount: Fixed fee amount
            currency: Currency code (e.g., "USD")

        Returns:
            Self for method chaining
        """
        ...

    def add_tranche_interest(self, recipient_id: str, tranche_id: str) -> "WaterfallTier":
        """Add a tranche interest recipient.

        Args:
            recipient_id: Unique recipient identifier
            tranche_id: Tranche identifier

        Returns:
            Self for method chaining
        """
        ...

    def add_tranche_principal(self, recipient_id: str, tranche_id: str) -> "WaterfallTier":
        """Add a tranche principal recipient.

        Args:
            recipient_id: Unique recipient identifier
            tranche_id: Tranche identifier

        Returns:
            Self for method chaining
        """
        ...

    def set_allocation_mode(self, mode: AllocationMode) -> "WaterfallTier":
        """Set allocation mode for this tier.

        Args:
            mode: AllocationMode (Sequential or ProRata)

        Returns:
            Self for method chaining
        """
        ...

    def set_divertible(self, divertible: bool) -> "WaterfallTier":
        """Mark tier as divertible.

        Args:
            divertible: Whether this tier can be diverted

        Returns:
            Self for method chaining
        """
        ...

    @property
    def tier_id(self) -> str:
        """Get tier ID."""
        ...

    @property
    def priority(self) -> int:
        """Get priority."""
        ...

    @property
    def recipient_count(self) -> int:
        """Get number of recipients."""
        ...
