"""Evaluator for financial models."""

from typing import Optional, Dict, Any, List, Tuple
from datetime import date
from ..types.model import FinancialModelSpec
from ...core.dates.periods import PeriodId
from ...core.market_data.context import MarketContext

class ResultsMeta:
    """Metadata about evaluation results."""

    @property
    def eval_time_ms(self) -> Optional[int]:
        """Evaluation time in milliseconds.

        Returns:
            int | None: Evaluation time if available
        """
        ...

    @property
    def num_nodes(self) -> int:
        """Number of nodes evaluated.

        Returns:
            int: Number of nodes
        """
        ...

    @property
    def num_periods(self) -> int:
        """Number of periods evaluated.

        Returns:
            int: Number of periods
        """
        ...

    def __repr__(self) -> str: ...

class Results:
    """Results from evaluating a financial model."""

    def get(self, node_id: str, period_id: PeriodId) -> Optional[float]:
        """Get the value for a node at a specific period.

        Args:
            node_id: Node identifier
            period_id: Period identifier

        Returns:
            float | None: Value if found, None otherwise
        """
        ...

    def get_node(self, node_id: str) -> Optional[Dict[PeriodId, float]]:
        """Get all period values for a specific node.

        Args:
            node_id: Node identifier

        Returns:
            dict[PeriodId, float] | None: Period values if node exists
        """
        ...

    def get_or(self, node_id: str, period_id: PeriodId, default: float) -> float:
        """Get value or default.

        Args:
            node_id: Node identifier
            period_id: Period identifier
            default: Default value if not found

        Returns:
            float: Value or default
        """
        ...

    def all_periods(self, node_id: str) -> List[Tuple[PeriodId, float]]:
        """Get an iterator over all periods for a node.

        Args:
            node_id: Node identifier

        Returns:
            list[tuple[PeriodId, float]]: List of period-value pairs
        """
        ...

    @property
    def nodes(self) -> Dict[str, Dict[PeriodId, float]]:
        """Get all node results.

        Returns:
            dict[str, dict[PeriodId, float]]: Map of node_id to period values
        """
        ...

    @property
    def meta(self) -> ResultsMeta:
        """Get evaluation metadata.

        Returns:
            ResultsMeta: Evaluation metadata
        """
        ...

    def to_json(self) -> str:
        """Convert to JSON string.

        Returns:
            str: JSON representation
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> Results:
        """Create from JSON string.

        Args:
            json_str: JSON string

        Returns:
            Results: Deserialized results
        """
        ...

    def __repr__(self) -> str: ...

class Evaluator:
    """Evaluator for financial models.

    The evaluator compiles formulas, resolves dependencies, and evaluates
    nodes period-by-period according to precedence rules.
    """

    @classmethod
    def new(cls) -> Evaluator:
        """Create a new evaluator.

        Returns:
            Evaluator: Evaluator instance
        """
        ...

    def evaluate(self, model: FinancialModelSpec) -> Results:
        """Evaluate a financial model over all periods.

        This is a convenience method that calls `evaluate_with_market_context`
        with no market context. If your model uses capital structure with cs.*
        references, use `evaluate_with_market_context` and provide market data.

        Args:
            model: Financial model specification

        Returns:
            Results: Evaluation results
        """
        ...

    def evaluate_with_market_context(
        self,
        model: FinancialModelSpec,
        market_ctx: MarketContext,
        as_of: date
    ) -> Results:
        """Evaluate a financial model with market context for pricing.

        This method allows you to provide market context for pricing capital
        structure instruments. If capital structure is defined but market context
        is not provided, capital structure cashflows will not be computed (cs.*
        references will fail at runtime).

        Args:
            model: Financial model specification
            market_ctx: Market context for pricing instruments
            as_of: Valuation date for pricing

        Returns:
            Results: Evaluation results
        """
        ...

    def __repr__(self) -> str: ...
