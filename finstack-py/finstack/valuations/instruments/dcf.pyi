"""DCF valuation helpers."""

from __future__ import annotations
from typing import Dict, Any
from ...statements.types import FinancialModelSpec
from ...core.money import Money

def evaluate_dcf(
    model: FinancialModelSpec,
    wacc: float = 0.10,
    terminal_growth: float = 0.02,
    ufcf_node: str = "ufcf",
    net_debt_override: float | None = None,
    *,
    mid_year_convention: bool = False,
    terminal_type: str = "gordon_growth",
    terminal_metric: float | None = None,
    terminal_multiple: float | None = None,
    high_growth_rate: float | None = None,
    stable_growth_rate: float | None = None,
    half_life_years: float | None = None,
    shares_outstanding: float | None = None,
    dlom: float | None = None,
    dloc: float | None = None,
) -> Dict[str, Money | float]:
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
        Used in perpetuity formula for terminal value. Ignored when
        ``terminal_type`` is ``"exit_multiple"`` or ``"h_model"``.
    ufcf_node : str, optional
        Node name in the model for unlevered free cashflow (default: "ufcf").
    net_debt_override : float, optional
        Net debt override value. If None, calculated from the model.
    mid_year_convention : bool, optional
        Enable mid-year discounting convention (default: False). When True,
        cash flows are discounted at (t - 0.5) instead of t.
    terminal_type : str, optional
        Terminal value method: "gordon_growth" (default), "exit_multiple", or "h_model".
    terminal_metric : float, optional
        Terminal metric value (e.g., EBITDA) for exit multiple method.
        Required when ``terminal_type="exit_multiple"``.
    terminal_multiple : float, optional
        Exit multiple (e.g., 10.0 for 10x EBITDA).
        Required when ``terminal_type="exit_multiple"``.
    high_growth_rate : float, optional
        Initial high growth rate for H-model.
        Required when ``terminal_type="h_model"``.
    stable_growth_rate : float, optional
        Stable growth rate for H-model.
        Required when ``terminal_type="h_model"``.
    half_life_years : float, optional
        Half-life of growth fade for H-model.
        Required when ``terminal_type="h_model"``.
    shares_outstanding : float, optional
        Basic shares outstanding for per-share value calculation.
    dlom : float, optional
        Discount for Lack of Marketability (0.0-1.0, e.g., 0.25 for 25%).
    dloc : float, optional
        Discount for Lack of Control (0.0-1.0, e.g., 0.20 for 20%).

    Returns
    -------
    Dict[str, Money | float]
        Dictionary containing:
        - "equity_value": Money - Equity value (EV - net debt, after discounts)
        - "enterprise_value": Money - Total enterprise value
        - "net_debt": Money - Net debt used
        - "terminal_value_pv": Money - Terminal value (present value)
        - "equity_value_per_share": float (optional) - Per-share value if shares_outstanding set
        - "diluted_shares": float (optional) - Diluted share count if shares_outstanding set

    Raises
    ------
    ValueError
        If model is invalid, if ufcf_node is not found, or if wacc/growth rates
        are invalid.

    Examples
    --------
        >>> from finstack.statements import FinancialModelSpec
        >>> from finstack.valuations.instruments import evaluate_dcf
        >>> result = evaluate_dcf(
        ...     model,
        ...     wacc=0.12,
        ...     terminal_growth=0.03,
        ...     mid_year_convention=True,
        ...     shares_outstanding=1_000_000.0,
        ...     dlom=0.25,
        ... )
        >>> print(result["equity_value_per_share"])

    Sources
    -------
    - Damodaran (DCF valuation): see ``docs/REFERENCES.md#damodaranInvestmentValuation``.
    """
    ...
