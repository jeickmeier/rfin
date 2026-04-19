"""Portfolio construction, valuation, optimization, cashflows, scenarios, and metrics."""

from __future__ import annotations

from finstack.core.market_data import MarketContext

__all__ = [
    "Portfolio",
    "PortfolioResult",
    "PortfolioValuation",
    "aggregate_full_cashflows",
    "aggregate_cashflows",
    "aggregate_metrics",
    "apply_scenario_and_revalue",
    "build_portfolio_from_spec",
    "optimize_portfolio",
    "parse_portfolio_spec",
    "portfolio_result_get_metric",
    "portfolio_result_total_value",
    "value_portfolio",
]

class Portfolio:
    """Built runtime portfolio. Cheap to clone; pass directly to pipeline functions.

    Build once with :meth:`from_spec` and reuse across ``value_portfolio``,
    ``aggregate_cashflows``, ``aggregate_metrics``, and
    ``apply_scenario_and_revalue`` to skip the per-call spec parse + index
    rebuild.
    """

    @staticmethod
    def from_spec(spec_json: str) -> Portfolio:
        """Parse a ``PortfolioSpec`` JSON string into a runtime portfolio."""
        ...

    @property
    def id(self) -> str: ...
    @property
    def as_of(self) -> str: ...
    @property
    def base_ccy(self) -> str: ...
    def __len__(self) -> int: ...
    def to_spec_json(self) -> str: ...
    def __repr__(self) -> str: ...

class PortfolioValuation:
    """Typed wrapper around a ``PortfolioValuation`` result.

    Wrap the JSON returned by :func:`value_portfolio` once and pass the typed
    object to :func:`aggregate_metrics` to skip re-parsing.
    """

    @staticmethod
    def from_json(valuation_json: str) -> PortfolioValuation: ...
    def to_json(self) -> str: ...
    @property
    def total_value(self) -> float: ...
    @property
    def base_ccy(self) -> str: ...
    @property
    def as_of(self) -> str: ...
    def __len__(self) -> int: ...
    def __repr__(self) -> str: ...

class PortfolioResult:
    """Typed wrapper around a ``PortfolioResult`` envelope.

    Use the scalar accessors (``total_value``, ``get_metric``) to read single
    values without re-parsing the JSON envelope.
    """

    @staticmethod
    def from_json(result_json: str) -> PortfolioResult: ...
    def to_json(self) -> str: ...
    @property
    def total_value(self) -> float: ...
    def get_metric(self, metric_id: str) -> float | None: ...
    def require_metric(self, metric_id: str) -> float: ...
    def __repr__(self) -> str: ...

def parse_portfolio_spec(json_str: str) -> str:
    """Parse and canonicalize a ``PortfolioSpec`` from JSON."""
    ...

def build_portfolio_from_spec(spec_json: str) -> str:
    """Build a runtime portfolio from JSON and return the round-tripped spec.

    Prefer :meth:`Portfolio.from_spec` for real work — it returns the typed
    object that pipeline functions reuse without rebuilding.
    """
    ...

def portfolio_result_total_value(result: PortfolioResult | str) -> float:
    """Read total portfolio value from a ``PortfolioResult`` envelope.

    Accepts a typed :class:`PortfolioResult` (O(1)) or a JSON string
    (O(size-of-envelope)).
    """
    ...

def portfolio_result_get_metric(result: PortfolioResult | str, metric_id: str) -> float | None:
    """Read one metric from a ``PortfolioResult``.

    Accepts a typed :class:`PortfolioResult` or a JSON string.
    """
    ...

def aggregate_metrics(
    valuation: PortfolioValuation | str,
    base_ccy: str,
    market: MarketContext | str,
    as_of: str,
) -> str:
    """Aggregate portfolio metrics from a valuation.

    Accepts a typed :class:`PortfolioValuation` (fast path) or a JSON string.
    """
    ...

def value_portfolio(
    portfolio: Portfolio | str,
    market: MarketContext | str,
    strict_risk: bool = False,
) -> str:
    """Value a portfolio.

    Accepts either a typed :class:`Portfolio` (no rebuild) or a JSON
    ``PortfolioSpec`` string, and either a typed ``MarketContext`` or a JSON
    string. Returns JSON for backwards compatibility — wrap with
    :meth:`PortfolioValuation.from_json` once to enable the fast downstream
    path into ``aggregate_metrics``.
    """
    ...

def aggregate_cashflows(portfolio: Portfolio | str, market: MarketContext | str) -> str:
    """Build the simple cashflow ladder for the portfolio."""
    ...

def aggregate_full_cashflows(portfolio: Portfolio | str, market: MarketContext | str) -> str:
    """Build the full classified cashflow ladder for the portfolio."""
    ...

def apply_scenario_and_revalue(
    portfolio: Portfolio | str,
    scenario_json: str,
    market: MarketContext | str,
) -> tuple[str, str]:
    """Apply a scenario and revalue the portfolio.

    Returns ``(valuation_json, report_json)``.
    """
    ...

def optimize_portfolio(spec_json: str, market: MarketContext | str) -> str:
    """Optimize portfolio weights using the LP-based optimizer.

    Accepts a ``PortfolioOptimizationSpec`` JSON that combines the portfolio
    specification with an objective function, constraints, and weighting
    scheme. Returns compact JSON — use :func:`json.dumps(json.loads(...), indent=2)`
    to pretty-print if desired.
    """
    ...
