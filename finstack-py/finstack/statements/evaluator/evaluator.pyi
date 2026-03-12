"""Evaluator for financial models."""

from __future__ import annotations
from typing import Dict, Any, List, Tuple
from datetime import date
from ..types.model import FinancialModelSpec
from ..types.node import NodeValueType
from ..capital_structure import CapitalStructureCashflows
from ...core.dates.periods import PeriodId
from ...core.market_data.context import MarketContext
from ...core.money import Money

class ResultsMeta:
    """Metadata about evaluation results."""

    @property
    def eval_time_ms(self) -> int | None:
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

class StatementResult:
    """Result from evaluating a financial model."""

    def get(self, node_id: str, period_id: PeriodId) -> float | None:
        """Get the value for a node at a specific period.

        Args:
            node_id: Node identifier
            period_id: Period identifier

        Returns:
            float | None: Value if found, None otherwise
        """
        ...

    def get_node(self, node_id: str) -> Dict[PeriodId, float] | None:
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

    def get_money(self, node_id: str, period_id: PeriodId) -> Money | None:
        """Get monetary value for a node at a specific period.

        Args:
            node_id: Node identifier
            period_id: Period identifier

        Returns:
            Money | None: Money value if monetary node, None otherwise
        """
        ...

    def get_scalar(self, node_id: str, period_id: PeriodId) -> float | None:
        """Get scalar value for a node at a specific period.

        Args:
            node_id: Node identifier
            period_id: Period identifier

        Returns:
            float | None: Scalar value if scalar node, None otherwise
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

    @property
    def node_value_types(self) -> Dict[str, NodeValueType]:
        """Get node value types (monetary vs scalar).

        Returns:
            dict[str, NodeValueType]: Map of node_id to value type
        """
        ...

    @property
    def cs_cashflows(self) -> CapitalStructureCashflows | None:
        """Get capital structure cashflows if available.

        Returns:
            CapitalStructureCashflows | None: Capital structure cashflows if model has capital structure
        """
        ...

    def to_json(self) -> str:
        """Convert to JSON string.

        Returns:
            str: JSON representation
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> StatementResult:
        """Create from JSON string.

        Args:
            json_str: JSON string

        Returns:
            StatementResult: Deserialized results
        """
        ...

    def to_polars_long(self) -> Any:
        """Export results to long-format Polars DataFrame.

        Schema: (node_id, period_id, value, value_money, currency, value_type)

        Returns:
            polars.DataFrame: Long-format DataFrame with all node-period combinations
        """
        ...

    def to_polars_wide(self) -> Any:
        """Export results to wide-format Polars DataFrame.

        Schema: periods as rows, nodes as columns

        Returns:
            polars.DataFrame: Wide-format DataFrame with periods as rows and nodes as columns
        """
        ...

    def to_polars_long_filtered(self, node_filter: List[str]) -> Any:
        """Export results to long-format Polars DataFrame with node filtering.

        Args:
            node_filter: List of node IDs to include (empty list includes all nodes)

        Returns:
            polars.DataFrame: Filtered long-format DataFrame
        """
        ...

    def __repr__(self) -> str: ...

class EvaluatorWithContext:
    """Evaluator with pre-configured market context.

    This is a convenience wrapper that stores market context and as-of date
    for capital structure evaluation.
    """

    @classmethod
    def new(cls, market_ctx: MarketContext, as_of: date) -> EvaluatorWithContext:
        """Create a new evaluator with pre-configured market context.

        Args:
            market_ctx: Market context with discount/forward curves
            as_of: Valuation date for pricing

        Returns:
            EvaluatorWithContext: Evaluator instance with stored context
        """
        ...

    def evaluate(self, model: FinancialModelSpec) -> StatementResult:
        """Evaluate a financial model using the stored market context.

        Args:
            model: Financial model specification

        Returns:
            StatementResult: Evaluation results
        """
        ...

class DependencyGraph:
    """Dependency graph for financial models.

    Provides DAG construction and topological ordering for model nodes.
    """

    @classmethod
    def from_model(cls, model: FinancialModelSpec) -> DependencyGraph:
        """Construct a dependency graph from a financial model.

        Args:
            model: Financial model specification

        Returns:
            DependencyGraph: Dependency graph instance
        """
        ...

    def topological_order(self) -> List[str]:
        """Get topological ordering of nodes.

        Returns:
            list[str]: Node IDs in evaluation order
        """
        ...

    def dependencies(self, node_id: str) -> List[str]:
        """Get direct dependencies for a node.

        Args:
            node_id: Node identifier

        Returns:
            list[str]: List of node IDs that this node depends on
        """
        ...

    def has_cycle(self) -> bool:
        """Check if the graph has cycles.

        Returns:
            bool: True if there are circular dependencies
        """
        ...

    def __repr__(self) -> str: ...

class MonteCarloResults:
    """Monte Carlo results for statement forecasts.

    This structure stores percentile bands per metric and period for a Monte
    Carlo evaluation of statement forecasts.
    """

    @property
    def percentile_results(self) -> Dict[str, PercentileSeries]:
        """Aggregated percentile results by metric."""
        ...

    @property
    def n_paths(self) -> int:
        """Number of Monte Carlo paths simulated."""
        ...

    @property
    def percentiles(self) -> List[float]:
        """Percentiles computed for each metric/period."""
        ...

    @property
    def forecast_periods(self) -> List[PeriodId]:
        """Forecast periods included in the simulation."""
        ...

    def get_percentile(self, metric: str, percentile: float) -> Dict[PeriodId, float] | None:
        """Get a percentile time series for a metric.

        Args:
            metric: Metric / node identifier (e.g. ``"ebitda"``)
            percentile: Percentile in [0.0, 1.0] (e.g. 0.95 for P95)

        Returns:
            dict[PeriodId, float] | None: Map of period → percentile value.
        """
        ...

    def breach_probability(self, metric: str, threshold: float) -> float | None:
        """Estimate breach probability for a metric crossing a threshold.

        The current implementation returns the probability that
        ``metric > threshold`` in any forecast period across all paths.

        Args:
            metric: Metric / node identifier (e.g. ``"leverage"``)
            threshold: Breach threshold (e.g. 4.5 for leverage)

        Returns:
            float | None: Breach probability in [0.0, 1.0] or ``None``.
        """
        ...

class PercentileSeries:
    """Per-metric percentile time series."""

    @property
    def metric(self) -> str:
        """Metric / node identifier."""
        ...

    @property
    def values(self) -> Dict[PeriodId, List[Tuple[float, float]]]:
        """Period → ordered list of ``(percentile, value)`` pairs."""
        ...

    def __repr__(self) -> str: ...

class Evaluator:
    """Evaluator for financial statement models.

    Evaluator compiles formulas, resolves dependencies, and evaluates nodes
    period-by-period according to precedence rules (Value > Forecast > Formula).
    It supports both standalone evaluation and evaluation with market context
    for capital structure pricing.

    The evaluator performs topological sorting of the dependency graph and
    evaluates nodes in the correct order, ensuring all dependencies are
    computed before dependent nodes.

    Examples
    --------
    Evaluate a basic model:

        >>> from finstack.core.dates.periods import PeriodId
        >>> from finstack.statements.builder import ModelBuilder
        >>> from finstack.statements.evaluator import Evaluator
        >>> from finstack.statements.types import AmountOrScalar
        >>> builder = ModelBuilder.new("DocCo")
        >>> builder.periods("2025Q1..Q2", None)
        >>> builder.value(
        ...     "revenue",
        ...     [
        ...         (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100.0)),
        ...         (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(110.0)),
        ...     ],
        ... )
        >>> builder.compute("gross_profit", "revenue * 0.4")
        >>> model = builder.build()
        >>> evaluator = Evaluator.new()
        >>> results = evaluator.evaluate(model)
        >>> gp_q1 = results.get("gross_profit", PeriodId.quarter(2025, 1))
        >>> print(round(gp_q1, 2), results.meta.num_nodes, results.meta.num_periods)
        40.0 2 2

    Export to DataFrames (requires Polars):

        Use :meth:`StatementResult.to_polars_long`, :meth:`StatementResult.to_polars_wide`,
        or :meth:`StatementResult.to_polars_long_filtered` on the ``results`` object to
        obtain DataFrames for downstream analysis.

    Notes
    -----
    - Evaluation is deterministic and reproducible
    - Formulas are compiled once and reused across periods
    - Capital structure requires market context for pricing
    - Results can be exported to Polars DataFrames for analysis

    See Also
    --------
    :class:`EvaluatorWithContext`: Convenience wrapper with stored context
    :class:`StatementResult`: Evaluation results structure
    :class:`DependencyGraph`: Dependency analysis
    """

    @classmethod
    def new(cls) -> Evaluator:
        """Create a new evaluator.

        Returns:
            Evaluator: Evaluator instance
        """
        ...

    def evaluate(self, model: FinancialModelSpec) -> StatementResult:
        """Evaluate a financial model over all periods.

        This is a convenience method that calls `evaluate_with_market_context`
        with no market context. If your model uses capital structure with cs.*
        references, use `evaluate_with_market_context` and provide market data.

        Args:
            model: Financial model specification

        Returns:
            StatementResult: Evaluation results
        """
        ...

    def evaluate_with_market_context(
        self, model: FinancialModelSpec, market_ctx: MarketContext, as_of: date
    ) -> StatementResult:
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
            StatementResult: Evaluation results
        """
        ...

    def evaluate_monte_carlo(
        self,
        model: FinancialModelSpec,
        n_paths: int,
        seed: int,
        percentiles: List[float] | None = None,
    ) -> MonteCarloResults:
        """Evaluate a financial model using Monte Carlo simulation of forecasts.

        This method replays the model ``n_paths`` times with independent, but
        deterministic, seeds for stochastic forecast methods (Normal, LogNormal)
        and aggregates paths into percentile bands.

        Args:
            model: Financial model specification
            n_paths: Number of Monte Carlo paths to simulate
            seed: Base random seed (same inputs ⇒ same results)
            percentiles: Percentiles in [0.0, 1.0] (default: [0.05, 0.5, 0.95])

        Returns:
            MonteCarloResults: Monte Carlo percentile results.
        """
        ...

    def with_market_context(self, market_ctx: MarketContext, as_of: date) -> EvaluatorWithContext:
        """Create evaluator with pre-configured market context.

        Args:
            market_ctx: Market context with curves
            as_of: Valuation date

        Returns:
            EvaluatorWithContext: Evaluator with stored context
        """
        ...

    def __repr__(self) -> str: ...
