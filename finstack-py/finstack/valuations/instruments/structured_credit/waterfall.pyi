"""Structured credit waterfall engine bindings."""

from enum import IntEnum
from typing import Any, Optional

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
        >>> tier = WaterfallTier("fees", 1, PaymentType.Fee)
        >>> tier.add_fixed_fee("trustee", "Trustee", 50000.0, "USD")
        >>> tier.set_allocation_mode(AllocationMode.Sequential)
    """
    
    def __init__(self, tier_id: str, priority: int, payment_type: PaymentType) -> None: ...
    
    def add_recipient(
        self, recipient_id: str, recipient_type: str, calculation: str
    ) -> "WaterfallTier":
        """Add a recipient to this tier.
        
        Args:
            recipient_id: Unique recipient identifier
            recipient_type: Type of recipient (JSON format)
            calculation: Payment calculation (JSON format)
            
        Returns:
            Self for method chaining
        """
        ...
    
    def add_fixed_fee(
        self, recipient_id: str, provider_name: str, amount: float, currency: str
    ) -> "WaterfallTier":
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
    
    def add_tranche_interest(
        self, recipient_id: str, tranche_id: str
    ) -> "WaterfallTier":
        """Add a tranche interest recipient.
        
        Args:
            recipient_id: Unique recipient identifier
            tranche_id: Tranche identifier
            
        Returns:
            Self for method chaining
        """
        ...
    
    def add_tranche_principal(
        self, recipient_id: str, tranche_id: str
    ) -> "WaterfallTier":
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

def clo_2_0_template(currency: str) -> dict[str, Any]:
    """Create a CLO 2.0 waterfall template.
    
    Args:
        currency: Currency code (e.g., "USD")
        
    Returns:
        Waterfall configuration as JSON-serializable dict
        
    Examples:
        >>> waterfall = clo_2_0_template("USD")
        >>> print(waterfall["tiers"])
    """
    ...

def cmbs_standard_template(currency: str) -> dict[str, Any]:
    """Create a CMBS standard waterfall template.
    
    Args:
        currency: Currency code (e.g., "USD")
        
    Returns:
        Waterfall configuration as JSON-serializable dict
    """
    ...

def cre_operating_company_template(currency: str) -> dict[str, Any]:
    """Create a CRE operating company waterfall template.
    
    Args:
        currency: Currency code (e.g., "USD")
        
    Returns:
        Waterfall configuration as JSON-serializable dict
    """
    ...

def get_waterfall_template(template_name: str, currency: str) -> dict[str, Any]:
    """Get a waterfall template by name.
    
    Args:
        template_name: Template name ("clo_2.0", "cmbs_standard", "cre_operating")
        currency: Currency code (e.g., "USD")
        
    Returns:
        Waterfall configuration as JSON-serializable dict
        
    Examples:
        >>> waterfall = get_waterfall_template("clo_2.0", "USD")
    """
    ...

def available_waterfall_templates() -> list[dict[str, str]]:
    """List available waterfall templates.
    
    Returns:
        List of template metadata with name, description, and deal_type
        
    Examples:
        >>> templates = available_waterfall_templates()
        >>> for t in templates:
        ...     print(f"{t['name']}: {t['description']}")
    """
    ...

