"""DCF valuation helpers."""

from typing import Optional, Dict, Any
from ...statements.types import FinancialModelSpec
from ...core.money import Money

def evaluate_dcf(
    model: FinancialModelSpec,
    wacc: float = 0.10,
    terminal_growth: float = 0.02,
    ufcf_node: str = "ufcf",
    net_debt_override: Optional[float] = None,
) -> Dict[str, Money]:
    """Evaluate a corporate DCF using a statements FinancialModelSpec."""
    ...
