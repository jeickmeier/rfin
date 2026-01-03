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
    """Evaluate a corporate DCF (Discounted Cash Flow) valuation.

    This function performs a DCF valuation using a financial model specification
    from the statements module. It calculates enterprise value and equity value
    by discounting free cashflows and applying a terminal value.

    Parameters
    ----------
    model : FinancialModelSpec
        Financial model specification containing cashflow projections.
        Must include a node for unlevered free cashflow (UFCF).
    wacc : float, optional
        Weighted average cost of capital as a decimal (default: 0.10 for 10%).
        Used to discount free cashflows.
    terminal_growth : float, optional
        Terminal growth rate as a decimal (default: 0.02 for 2%).
        Used in perpetuity formula for terminal value.
    ufcf_node : str, optional
        Node name in the model for unlevered free cashflow (default: "ufcf").
    net_debt_override : float, optional
        Net debt override value. If None, calculated from the model.

    Returns
    -------
    Dict[str, Money]
        Dictionary containing:
        - "enterprise_value": Total enterprise value
        - "equity_value": Equity value (EV - net debt)
        - "terminal_value": Terminal value component
        - "present_value_cf": Present value of explicit cashflows

    Raises
    ------
    ValueError
        If model is invalid, if ufcf_node is not found, or if wacc/terminal_growth
        are invalid.

    Examples
    --------
        >>> from finstack.statements import FinancialModelSpec
        >>> from finstack.valuations.instruments import evaluate_dcf
        >>> # ... setup: build/load a FinancialModelSpec with a valid UFCF node
        >>> result = evaluate_dcf(
        ...     model,
        ...     wacc=0.12,  # 12% WACC
        ...     terminal_growth=0.03,  # 3% terminal growth
        ... )
        >>> print(result["equity_value"].amount)

    MarketContext Requirements
    -------------------------
    - None. This helper evaluates a DCF from the provided ``FinancialModelSpec`` and scalar inputs.

    Sources
    -------
    - Damodaran (DCF valuation): see ``docs/REFERENCES.md#damodaranInvestmentValuation``.
    """
    ...
