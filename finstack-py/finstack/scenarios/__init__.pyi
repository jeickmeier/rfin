"""Scenario specification, validation, composition, application, and built-in templates."""

from __future__ import annotations

from typing import Any

from finstack.valuations import PnlAttribution

__all__ = [
    "parse_scenario_spec",
    "build_scenario_spec",
    "compose_scenarios",
    "validate_scenario_spec",
    "list_builtin_templates",
    "list_builtin_template_metadata",
    "build_from_template",
    "list_template_components",
    "build_template_component",
    "apply_scenario",
    "apply_scenario_to_market",
    "compute_horizon_return",
    "HorizonResult",
]

def parse_scenario_spec(json_str: str) -> str:
    """Parse, validate, and re-serialize a ``ScenarioSpec`` from JSON.

    Args:
        json_str: JSON-serialized ``ScenarioSpec``.

    Returns:
        Validated canonical JSON string.

    Example:
        >>> from finstack.scenarios import parse_scenario_spec
        >>> parse_scenario_spec(spec_json)  # doctest: +SKIP
        ''
    """
    ...

def build_scenario_spec(
    id: str,
    operations_json: str,
    name: str | None = None,
    description: str | None = None,
    priority: int = 0,
) -> str:
    """Construct a ``ScenarioSpec`` from fields plus a JSON operations list.

    Args:
        id: Stable scenario identifier.
        operations_json: JSON list of ``OperationSpec``.
        name: Optional display name.
        description: Optional long description.
        priority: Composition priority (lower runs first). Defaults to ``0``.

    Returns:
        Validated JSON ``ScenarioSpec``.

    Example:
        >>> from finstack.scenarios import build_scenario_spec
        >>> build_scenario_spec("s1", "[]")  # doctest: +SKIP
        ''
    """
    ...

def compose_scenarios(specs_json: str) -> str:
    """Merge multiple scenario specs using the scenario engine composer.

    Args:
        specs_json: JSON list of ``ScenarioSpec``.

    Returns:
        JSON-serialized composed ``ScenarioSpec``.

    Example:
        >>> from finstack.scenarios import compose_scenarios
        >>> compose_scenarios("[]")  # doctest: +SKIP
        ''
    """
    ...

def validate_scenario_spec(json_str: str) -> bool:
    """Return ``True`` after successfully parsing and validating JSON.

    Args:
        json_str: JSON-serialized ``ScenarioSpec``.

    Returns:
        Always ``True`` on success.

    Example:
        >>> from finstack.scenarios import validate_scenario_spec
        >>> validate_scenario_spec(spec_json)  # doctest: +SKIP
        True
    """
    ...

def list_builtin_templates() -> list[str]:
    """List template IDs from the embedded built-in registry.

    Args:
        (none)

    Returns:
        Template identifier strings.

    Example:
        >>> from finstack.scenarios import list_builtin_templates
        >>> isinstance(list_builtin_templates(), list)
        True
    """
    ...

def list_builtin_template_metadata() -> str:
    """Serialize metadata for all built-in templates to JSON.

    Args:
        (none)

    Returns:
        JSON list of ``TemplateMetadata`` objects.

    Example:
        >>> from finstack.scenarios import list_builtin_template_metadata
        >>> meta_json = list_builtin_template_metadata()
    """
    ...

def build_from_template(template_id: str) -> str:
    """Instantiate a ``ScenarioSpec`` from a built-in template.

    Args:
        template_id: Registry key for the template.

    Returns:
        JSON-serialized ``ScenarioSpec``.

    Example:
        >>> from finstack.scenarios import build_from_template
        >>> build_from_template("unknown")  # doctest: +SKIP
        ''
    """
    ...

def list_template_components(template_id: str) -> list[str]:
    """List sub-component IDs for composite templates.

    Args:
        template_id: Parent template identifier.

    Returns:
        Component identifiers.

    Example:
        >>> from finstack.scenarios import list_template_components
        >>> list_template_components("t")  # doctest: +SKIP
        []
    """
    ...

def build_template_component(template_id: str, component_id: str) -> str:
    """Build a single component spec from a composite template.

    Args:
        template_id: Parent template identifier.
        component_id: Component key inside the template.

    Returns:
        JSON-serialized component ``ScenarioSpec``.

    Example:
        >>> from finstack.scenarios import build_template_component
        >>> build_template_component("t", "c")  # doctest: +SKIP
        ''
    """
    ...

def apply_scenario(
    scenario_json: str,
    market: Any,
    model: Any,
    as_of: str,
) -> dict[str, Any]:
    """Apply a scenario to both market data and a financial model.

    Args:
        scenario_json: JSON ``ScenarioSpec``.
        market: ``MarketContext`` object or JSON ``MarketContext`` string.
        model: ``FinancialModelSpec`` object or JSON ``FinancialModelSpec`` string.
        as_of: ISO 8601 valuation date.

    Returns:
        Dict with ``market_json``, ``model_json``, ``operations_applied`` (``int``),
        ``user_operations`` (``int``), ``expanded_operations`` (``int``),
        ``warnings`` (``list[str]``, rendered Display form), and
        ``warnings_json`` (``str``, JSON-encoded list of structured ``Warning``
        records — parse with ``json.loads(...)`` for programmatic
        ``kind``-based dispatch).

    Example:
        >>> from finstack.scenarios import apply_scenario
        >>> apply_scenario(sj, mj, fj, "2025-01-15")  # doctest: +SKIP
        {}
    """
    ...

def apply_scenario_to_market(
    scenario_json: str,
    market: Any,
    as_of: str,
) -> dict[str, Any]:
    """Apply a scenario to market data only (no model mutations returned).

    Args:
        scenario_json: JSON ``ScenarioSpec``.
        market: ``MarketContext`` object or JSON ``MarketContext`` string.
        as_of: ISO 8601 valuation date.

    Returns:
        Dict with ``market_json``, ``operations_applied``, ``user_operations``,
        ``expanded_operations``, ``warnings`` (``list[str]``), and
        ``warnings_json`` (``str``, JSON-encoded list of structured warnings).

    Example:
        >>> from finstack.scenarios import apply_scenario_to_market
        >>> apply_scenario_to_market(sj, mj, "2025-01-15")  # doctest: +SKIP
        {}
    """
    ...

class HorizonResult:
    """Horizon total return result with full P&L attribution."""

    @property
    def attribution(self) -> PnlAttribution:
        """Full P&L attribution breakdown."""
        ...

    @property
    def initial_value(self) -> float:
        """Initial instrument value."""
        ...

    @property
    def terminal_value(self) -> float:
        """Final instrument value after the scenario is applied."""
        ...

    @property
    def horizon_days(self) -> int | None:
        """Horizon in calendar days (``None`` if no time-roll)."""
        ...

    @property
    def total_return_pct(self) -> float:
        """Total return as decimal fraction (0.05 = 5%)."""
        ...

    @property
    def annualized_return(self) -> float | None:
        """Annualized return (``None`` if no time-roll)."""
        ...

    @property
    def operations_applied(self) -> int:
        """Number of scenario operations applied."""
        ...

    @property
    def user_operations(self) -> int:
        """Number of user-provided scenario operations before hierarchy expansion."""
        ...

    @property
    def expanded_operations(self) -> int:
        """Number of direct operations after hierarchy expansion and deduplication."""
        ...

    @property
    def warnings(self) -> list[str]:
        """Warnings emitted during scenario application (rendered Display form)."""
        ...

    @property
    def warnings_json(self) -> str:
        """JSON-encoded structured warnings.

        Each entry is a `Warning` record with a ``kind`` discriminator plus
        variant-specific fields, mirroring the WASM binding. Parse with
        ``json.loads(...)`` to dispatch on ``kind`` programmatically.
        """
        ...

    def factor_contribution(self, factor: str) -> float:
        """Factor contribution as decimal fraction of initial value.

        Args:
            factor: One of ``"carry"``, ``"rates"``/``"rates_curves"``,
                ``"credit"``/``"credit_curves"``, ``"inflation"``/``"inflation_curves"``,
                ``"correlations"``, ``"fx"``, ``"volatility"``/``"vol"``,
                ``"model_parameters"``/``"model_params"``, or
                ``"market_scalars"``/``"scalars"``.

        Returns:
            Contribution of the given factor as a decimal fraction.
        """
        ...

    def to_json(self) -> str:
        """Serialize the result to JSON."""
        ...

    def explain(self) -> str:
        """Human-readable summary of horizon return and attribution."""
        ...

def compute_horizon_return(
    instrument_json: str,
    market: Any,
    as_of: str,
    scenario_json: str,
    method: str = "parallel",
    config: str | None = None,
) -> HorizonResult:
    """Compute horizon total return under a scenario.

    Args:
        instrument_json: JSON-serialized instrument (tagged ``{"type": ..., "spec": {...}}``).
        market: ``MarketContext`` object or JSON string.
        as_of: Valuation date in ISO 8601 format.
        scenario_json: JSON-serialized ``ScenarioSpec``.
        method: Attribution method — ``"parallel"`` (default), ``"waterfall"``,
            ``"metrics_based"``, or ``"taylor"``.
        config: Optional JSON-serialized ``FinstackConfig``.

    Returns:
        ``HorizonResult`` with decomposed total return and factor attribution.
    """
    ...
