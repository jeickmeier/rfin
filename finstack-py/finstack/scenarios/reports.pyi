"""Report types from scenario execution."""

from __future__ import annotations
from typing import List, Tuple
from datetime import date

class ApplicationReport:
    """Report describing scenario application results.

    ApplicationReport provides a summary of what happened when a scenario
    was applied to an execution context. It includes the number of operations
    applied, any warnings generated, and metadata for reproducibility.

    Examples
    --------
    Inspect application results:

        >>> from datetime import date
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.statements.types import FinancialModelSpec
        >>> from finstack.scenarios import (
        ...     ScenarioSpec,
        ...     ScenarioEngine,
        ...     ExecutionContext,
        ...     OperationSpec,
        ...     CurveKind,
        ... )
        >>> market = MarketContext()
        >>> market.insert(DiscountCurve("USD-SOFR", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.99)]))
        >>> ctx = ExecutionContext(market, FinancialModelSpec("doc_report", []), date(2025, 1, 1))
        >>> ops = [OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD-SOFR", 25.0)]
        >>> scenario = ScenarioSpec("doc_report", ops, name="Doc Report")
        >>> report = ScenarioEngine().apply(scenario, ctx)
        >>> print(report.operations_applied, report.warnings, report.rounding_context)
        1 [] default

    Notes
    -----
    - operations_applied counts successfully applied operations
    - warnings are non-fatal issues (e.g., missing optional data)
    - rounding_context provides reproducibility metadata
    - Use to verify scenario was applied correctly

    See Also
    --------
    :class:`ScenarioEngine`: Scenario execution engine
    :class:`RollForwardReport`: Time roll-forward report
    """

    @property
    def operations_applied(self) -> int:
        """Number of operations successfully applied.

        Returns:
            int: Count of applied operations
        """
        ...

    @property
    def warnings(self) -> List[str]:
        """Warnings generated during application (non-fatal).

        Returns:
            list[str]: List of warning messages
        """
        ...

    @property
    def rounding_context(self) -> str | None:
        """Rounding context stamp (for reproducibility metadata).

        Returns:
            str | None: Rounding context identifier if available
        """
        ...

    def __repr__(self) -> str: ...

class RollForwardReport:
    """Report from time roll-forward operation.

    RollForwardReport provides detailed P&L attribution from rolling the
    valuation date forward. It breaks down P&L into carry (time decay,
    interest accrual) and market value changes (price movements).

    This report is generated when a time roll-forward operation is applied
    in a scenario. It provides transparency into how portfolio value changes
    over time.

    Examples
    --------
    Inspect roll-forward results:

        >>> from finstack.scenarios import RollForwardReport
        >>> report = RollForwardReport.example()
        >>> print(
        ...     report.old_date,
        ...     report.new_date,
        ...     report.days,
        ...     report.total_carry["USD"],
        ...     report.total_mv_change["USD"],
        ... )
        2025-01-01 2025-02-01 31 25000.0 -10000.0

    Notes
    -----
    - Carry represents time decay and interest accrual
    - Market value change represents price movements
    - P&L is broken down by instrument and currency
    - Useful for P&L attribution and risk reporting

    See Also
    --------
    :class:`OperationSpec.time_roll_forward`: Time roll-forward operation
    :class:`ApplicationReport`: General scenario application report
    """

    @property
    def old_date(self) -> date:
        """Original as-of date.

        Returns:
            date: Date before roll
        """
        ...

    @property
    def new_date(self) -> date:
        """New as-of date after roll.

        Returns:
            date: Date after roll
        """
        ...

    @property
    def days(self) -> int:
        """Number of days rolled forward.

        Returns:
            int: Day count
        """
        ...

    @property
    def instrument_carry(self) -> List[Tuple[str, List[Tuple[str, float]]]]:
        """Per-instrument carry accrual by currency.

        Returns:
            list[tuple[str, list[tuple[str, float]]]]:
                List of (instrument_id, [(currency_code, amount)]) pairs
        """
        ...

    @property
    def instrument_mv_change(self) -> List[Tuple[str, List[Tuple[str, float]]]]:
        """Per-instrument market value change by currency.

        Returns:
            list[tuple[str, list[tuple[str, float]]]]:
                List of (instrument_id, [(currency_code, amount)]) pairs
        """
        ...

    @property
    def total_carry(self) -> dict[str, float]:
        """Total P&L from carry by currency.

        Returns:
            dict[str, float]: Mapping from currency code to total carry
        """
        ...

    @property
    def total_mv_change(self) -> dict[str, float]:
        """Total P&L from market value changes by currency.

        Returns:
            dict[str, float]: Mapping from currency code to total market value change
        """
        ...

    @classmethod
    def example(cls) -> RollForwardReport:
        """Return a deterministic sample report for documentation/testing.

        Returns:
            RollForwardReport: Synthetic report with USD carry and MV change.
        """
        ...

    def __repr__(self) -> str: ...
