"""Convenience reporting for financial statements."""

from typing import List
from ..evaluator import Results
from ..types import FinancialModelSpec
from ...core.dates.periods import PeriodId

class Alignment:
    """Alignment options for table columns."""

    LEFT: Alignment
    RIGHT: Alignment
    CENTER: Alignment

    def __repr__(self) -> str: ...

class TableBuilder:
    """Builder for ASCII and Markdown tables."""

    def __init__(self) -> None:
        """Create a new table builder."""
        ...

    def add_header(self, name: str) -> None:
        """Add a column header.

        Args:
            name: Column header text
        """
        ...

    def add_header_with_alignment(self, name: str, alignment: Alignment) -> None:
        """Add a column header with specific alignment.

        Args:
            name: Column header text
            alignment: Column alignment
        """
        ...

    def add_row(self, cells: List[str]) -> None:
        """Add a data row.

        Args:
            cells: List of cell values
        """
        ...

    def build(self) -> str:
        """Build ASCII table.

        Returns:
            str: Formatted ASCII table with box-drawing characters
        """
        ...

    def build_markdown(self) -> str:
        """Build Markdown table.

        Returns:
            str: Formatted Markdown table
        """
        ...

    def __repr__(self) -> str: ...

class PLSummaryReport:
    """P&L summary report."""

    def __init__(self, results: Results, line_items: List[str], periods: List[PeriodId]) -> None:
        """Create a new P&L summary report.

        Args:
            results: Evaluation results
            line_items: Node IDs to include
            periods: Periods to display
        """
        ...

    def to_string(self) -> str:
        """Convert report to string format.

        Returns:
            str: Formatted report
        """
        ...

    def to_markdown(self) -> str:
        """Convert report to Markdown format.

        Returns:
            str: Markdown formatted report
        """
        ...

    def print(self) -> None:
        """Print report to stdout."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class CreditAssessmentReport:
    """Credit assessment report."""

    def __init__(self, results: Results, as_of: PeriodId) -> None:
        """Create a new credit assessment report.

        Args:
            results: Evaluation results
            as_of: Period for assessment
        """
        ...

    def to_string(self) -> str:
        """Convert report to string format.

        Returns:
            str: Formatted report
        """
        ...

    def to_markdown(self) -> str:
        """Convert report to Markdown format.

        Returns:
            str: Markdown formatted report
        """
        ...

    def print(self) -> None:
        """Print report to stdout."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class DebtSummaryReport:
    """Debt summary report."""

    def __init__(self, model: FinancialModelSpec, results: Results, as_of: PeriodId) -> None:
        """Create a new debt summary report.

        Args:
            model: Financial model
            results: Evaluation results
            as_of: Period for report
        """
        ...

    def to_string(self) -> str:
        """Convert report to string format.

        Returns:
            str: Formatted report
        """
        ...

    def to_markdown(self) -> str:
        """Convert report to Markdown format.

        Returns:
            str: Markdown formatted report
        """
        ...

    def print(self) -> None:
        """Print report to stdout."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

def print_debt_summary(model: FinancialModelSpec, results: Results, as_of: PeriodId) -> None:
    """Convenience function to print debt summary.

    Args:
        model: Financial model
        results: Evaluation results
        as_of: Period for report
    """
    ...

__all__ = [
    "Alignment",
    "TableBuilder",
    "PLSummaryReport",
    "CreditAssessmentReport",
    "DebtSummaryReport",
    "print_debt_summary",
]


