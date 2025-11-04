"""P&L Attribution type stubs."""

from datetime import date
from typing import Optional
from finstack.core.money import Money
from finstack.core.market_data import MarketContext
from finstack.core.config import FinstackConfig

class AttributionMethod:
    """Attribution methodology selector."""
    
    @staticmethod
    def parallel() -> AttributionMethod:
        """Independent factor isolation (may not sum due to cross-effects)."""
        ...
    
    @staticmethod
    def waterfall(factors: list[str]) -> AttributionMethod:
        """Sequential waterfall (guarantees sum = total, order matters).
        
        Args:
            factors: Ordered list of factor names:
                - "carry"
                - "rates_curves"
                - "credit_curves"
                - "inflation_curves"
                - "correlations"
                - "fx"
                - "volatility"
                - "model_parameters"
                - "market_scalars"
        """
        ...
    
    @staticmethod
    def metrics_based() -> AttributionMethod:
        """Use existing metrics (Theta, DV01, CS01) for approximation."""
        ...

class PnlAttribution:
    """P&L attribution result for a single instrument."""
    
    @property
    def total_pnl(self) -> Money:
        """Total P&L (val_t1 - val_t0)."""
        ...
    
    @property
    def carry(self) -> Money:
        """Carry P&L (theta + accruals)."""
        ...
    
    @property
    def rates_curves_pnl(self) -> Money:
        """Interest rate curves P&L."""
        ...
    
    @property
    def credit_curves_pnl(self) -> Money:
        """Credit hazard curves P&L."""
        ...
    
    @property
    def inflation_curves_pnl(self) -> Money:
        """Inflation curves P&L."""
        ...
    
    @property
    def correlations_pnl(self) -> Money:
        """Base correlation curves P&L."""
        ...
    
    @property
    def fx_pnl(self) -> Money:
        """FX rate changes P&L."""
        ...
    
    @property
    def vol_pnl(self) -> Money:
        """Implied volatility changes P&L."""
        ...
    
    @property
    def model_params_pnl(self) -> Money:
        """Model parameters P&L."""
        ...
    
    @property
    def market_scalars_pnl(self) -> Money:
        """Market scalars P&L."""
        ...
    
    @property
    def residual(self) -> Money:
        """Residual P&L."""
        ...
    
    def to_csv(self) -> str:
        """Export attribution as CSV string."""
        ...
    
    def explain(self) -> str:
        """Generate structured tree explanation of P&L attribution."""
        ...

def attribute_pnl(
    instrument,
    market_t0: MarketContext,
    market_t1: MarketContext,
    as_of_t0: date,
    as_of_t1: date,
    method: Optional[AttributionMethod] = None,
) -> PnlAttribution:
    """Perform P&L attribution for an instrument.
    
    Args:
        instrument: Instrument to attribute
        market_t0: Market context at T₀
        market_t1: Market context at T₁
        as_of_t0: Valuation date at T₀
        as_of_t1: Valuation date at T₁
        method: Attribution methodology (defaults to Parallel)
    
    Returns:
        P&L attribution with factor breakdown
    
    Example:
        ```python
        attr = attribute_pnl(
            bond,
            market_yesterday,
            market_today,
            date(2025, 1, 15),
            date(2025, 1, 16),
            method=AttributionMethod.parallel()
        )
        
        print(f"Total P&L: {attr.total_pnl}")
        print(f"Carry: {attr.carry}")
        print(attr.explain())
        ```
    """
    ...

