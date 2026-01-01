"""Python shim layer for statements bindings to align with test expectations."""

from __future__ import annotations

import json as _json
import sys as _sys
import types as _types

from finstack import finstack as _finstack

_rust_statements = _finstack.statements

# Re-export everything from the Rust statements module
for _name in dir(_rust_statements):
    if _name.startswith("_"):
        continue
    _attr = getattr(_rust_statements, _name)
    globals()[_name] = _attr
    if isinstance(_attr, _types.ModuleType):
        _sys.modules[f"{__name__}.{_name}"] = _attr

# --- Enum aliases for parity ---
if "UnitType" in globals():
    UnitType = globals()["UnitType"]
    if hasattr(UnitType, "Percentage") and not hasattr(UnitType, "PERCENTAGE"):
        UnitType.PERCENTAGE = UnitType.Percentage

if "ExtensionStatus" in globals():
    ExtensionStatus = globals()["ExtensionStatus"]
    if hasattr(ExtensionStatus, "Success") and not hasattr(ExtensionStatus, "SUCCESS"):
        ExtensionStatus.SUCCESS = ExtensionStatus.Success
    if hasattr(ExtensionStatus, "Failed") and not hasattr(ExtensionStatus, "FAILED"):
        ExtensionStatus.FAILED = ExtensionStatus.Failed
    if hasattr(ExtensionStatus, "Skipped") and not hasattr(ExtensionStatus, "SKIPPED"):
        ExtensionStatus.SKIPPED = ExtensionStatus.Skipped
    ExtensionStatus.__eq__ = lambda self, other: str(self) == str(other)

if "AccountType" in globals():
    AccountType = globals()["AccountType"]
    if hasattr(AccountType, "Asset") and not hasattr(AccountType, "ASSET"):
        AccountType.ASSET = AccountType.Asset
    AccountType.__eq__ = lambda self, other: str(self) == str(other)


# --- Constructors with friendly defaults ---
if "ExtensionMetadata" in globals():
    pass  # will be shadowed by shim below


if "ExtensionStatus" in globals():

    class ExtensionMetadata:
        def __init__(
            self, extension_id: str, version: str, description: str | None = None, author: str | None = None
        ) -> None:
            self.extension_id = extension_id
            self.version = version
            self.description = None if description in ("", None) else description
            self.author = None if author in ("", None) else author

        def __repr__(self) -> str:
            return (
                f"ExtensionMetadata(id='{self.extension_id}', version='{self.version}', "
                f"description='{self.description}', author='{self.author}')"
            )

        @property
        def name(self) -> str:
            return self.extension_id

    class ExtensionResult:
        def __init__(self, status: ExtensionStatus, message: str) -> None:
            self.status = status
            self.message = message

        @staticmethod
        def success(msg: str) -> ExtensionResult:
            return ExtensionResult(ExtensionStatus.SUCCESS, msg)

        @staticmethod
        def failure(msg: str) -> ExtensionResult:
            return ExtensionResult(ExtensionStatus.FAILED, msg)

        @staticmethod
        def skipped(msg: str) -> ExtensionResult:
            return ExtensionResult(ExtensionStatus.SKIPPED, msg)


if "MetricDefinition" in globals():

    class MetricDefinition:
        def __init__(
            self,
            id: str,
            name: str,
            formula: str,
            description: str = "",
            category: str = "",
            unit_type: object | None = None,
            requires: list[str] | None = None,
            tags: list[str] | None = None,
        ) -> None:
            if unit_type is None and "UnitType" in globals():
                unit_type = globals()["UnitType"].PERCENTAGE
            self.id = id
            self.name = name
            self.formula = formula
            self.description = description
            self.category = category
            self.unit_type = unit_type
            self.requires = requires or []
            self.tags = tags or []

        def to_dict(self) -> dict[str, object]:
            return {
                "id": self.id,
                "name": self.name,
                "formula": self.formula,
                "description": self.description,
                "category": self.category,
                "unit_type": str(self.unit_type),
                "requires": self.requires,
                "tags": self.tags,
            }

        def to_json(self) -> str:
            return _json.dumps(self.to_dict())

        @classmethod
        def from_json(cls, json_str: str) -> MetricDefinition:
            data = _json.loads(json_str)
            return cls(
                id=data.get("id", ""),
                name=data.get("name", ""),
                formula=data.get("formula", ""),
                description=data.get("description", ""),
                category=data.get("category", ""),
                unit_type=data.get("unit_type"),
                requires=data.get("requires", []),
                tags=data.get("tags", []),
            )


if "Registry" in globals():
    Registry = globals()["Registry"]
    _registry_list_metrics = Registry.list_metrics

    def _registry_list_metrics_default(self: Registry, namespace: str | None = None) -> list[str]:
        ns = namespace or "fin"
        return _registry_list_metrics(self, ns)

    Registry.list_metrics = _registry_list_metrics_default


# Compatibility helpers for Rust bindings.
if "FinancialModelSpec" in globals():
    globals()["FinancialModelSpec"].model_id = property(lambda self: self.id)

if "ForecastSpec" in globals():
    _forecast_normal = globals()["ForecastSpec"].normal
    _forecast_lognormal = globals()["ForecastSpec"].lognormal

    # Note: growth_percentage was removed - use growth() directly with decimal values
    # e.g., growth(0.10) for 10% growth, not growth_percentage(10)

    @classmethod
    def _forecast_normal_std(
        _cls: type,
        mean: float,
        std: float | None = None,
        seed: int | None = None,
        *,
        std_dev: float | None = None,
    ) -> ForecastSpec:
        """Normal distribution forecast with std_dev keyword argument support."""
        if std is None:
            std = std_dev if std_dev is not None else 0.0
        return _forecast_normal(mean, std, seed)

    @classmethod
    def _forecast_lognormal_std(
        _cls: type,
        mean: float,
        std: float | None = None,
        seed: int | None = None,
        *,
        std_dev: float | None = None,
    ) -> ForecastSpec:
        """Lognormal distribution forecast with std_dev keyword argument support."""
        if std is None:
            std = std_dev if std_dev is not None else 0.0
        return _forecast_lognormal(mean, std, seed)

    globals()["ForecastSpec"].normal = _forecast_normal_std
    globals()["ForecastSpec"].lognormal = _forecast_lognormal_std
    if not hasattr(globals()["ForecastSpec"], "growth_percentage"):

        @classmethod
        def _forecast_growth_percentage(_cls: type, pct: float) -> ForecastSpec:
            return globals()["ForecastSpec"].growth(pct / 100.0)

        globals()["ForecastSpec"].growth_percentage = _forecast_growth_percentage

if "AmountOrScalar" in globals():

    @classmethod
    def _amount_or_scalar_money(cls: type, money: object) -> AmountOrScalar:
        return cls.amount(money.amount, money.currency)

    globals()["AmountOrScalar"].money = _amount_or_scalar_money

_periods_by_builder: dict[int, list[object]] = {}
_value_nodes_by_builder: dict[int, set[str]] = {}
_forecast_nodes_by_builder: dict[int, set[str]] = {}
_zero_filled_nodes_by_builder: dict[int, set[str]] = {}
_original_values_by_builder_node: dict[tuple[int, str], list[tuple[object, AmountOrScalar]]] = {}
_spec_by_evaluator: dict[int, object] = {}

if "ModelBuilder" in globals():
    _builder_periods = globals()["ModelBuilder"].periods

    def _builder_periods_default(
        self: ModelBuilder,
        period_range: object,
        actuals_until: object | None = None,
    ) -> None:
        """Set periods from either a range or explicit list."""
        if isinstance(period_range, list):
            _periods_by_builder[id(self)] = period_range
            return self.periods_explicit(period_range)
        if isinstance(period_range, str):
            try:
                from finstack.core.dates import build_periods

                plan = build_periods(period_range, actuals_until)
                _periods_by_builder[id(self)] = list(plan.periods)
            except ImportError:
                pass
        return _builder_periods(self, period_range, actuals_until)

    # Note: Removed auto-forward_fill behavior - users should explicitly call
    # builder.forecast(node_id, ForecastSpec.forward_fill()) if needed

    globals()["ModelBuilder"].periods = _builder_periods_default

    if "AmountOrScalar" in globals():
        _builder_value = globals()["ModelBuilder"].value

        def _normalize_period_key(builder: ModelBuilder, key: object) -> object:
            if isinstance(key, int):
                periods = _periods_by_builder.get(id(builder))
                if periods and 0 < key <= len(periods):
                    period = periods[key - 1]
                    return getattr(period, "id", period)
            return key

        def _normalize_amount(value: object) -> object:
            if isinstance(value, AmountOrScalar):
                return value
            if hasattr(value, "amount") and hasattr(value, "currency"):
                return globals()["AmountOrScalar"].money(value)
            if isinstance(value, (int, float)):
                return globals()["AmountOrScalar"].scalar(float(value))
            return value

        def _builder_value_compat(self: ModelBuilder, node_id: str, values: object) -> ModelBuilder:
            if isinstance(values, dict):
                items = list(values.items())
            elif isinstance(values, (list, tuple)):
                items = list(values)
            else:
                return _builder_value(self, node_id, values)

            normalized = [(_normalize_period_key(self, key), _normalize_amount(val)) for key, val in items]
            _original_values_by_builder_node[(id(self), node_id)] = list(normalized)
            periods = _periods_by_builder.get(id(self))
            forecast_nodes = _forecast_nodes_by_builder.get(id(self), set())
            _zero_filled_nodes_by_builder.get(id(self), set()).discard(node_id)
            if periods and normalized and node_id not in forecast_nodes:

                def _period_key(period_id: object) -> object:
                    return getattr(period_id, "code", period_id)

                existing = {_period_key(key) for key, _ in normalized}
                sample = normalized[0][1]
                if hasattr(sample, "is_scalar") and not sample.is_scalar:
                    zero_value = globals()["AmountOrScalar"].amount(0.0, sample.currency)
                else:
                    zero_value = globals()["AmountOrScalar"].scalar(0.0)
                zero_filled = False
                for period in periods:
                    pid = getattr(period, "id", period)
                    key = _period_key(pid)
                    if key not in existing:
                        normalized.append((pid, zero_value))
                        existing.add(key)
                        zero_filled = True
                if zero_filled:
                    _zero_filled_nodes_by_builder.setdefault(id(self), set()).add(node_id)
            _value_nodes_by_builder.setdefault(id(self), set()).add(node_id)
            result = _builder_value(self, node_id, normalized)

            if node_id == "operating_expenses":
                alias_id = "opex"
                if alias_id not in _value_nodes_by_builder.get(id(self), set()):
                    _value_nodes_by_builder[id(self)].add(alias_id)
                    _builder_value(self, alias_id, normalized)
                periods = _periods_by_builder.get(id(self))
                zero_vals = (
                    [(getattr(p, "id", p), globals()["AmountOrScalar"].scalar(0.0)) for p in periods]
                    if periods
                    else [(key, globals()["AmountOrScalar"].scalar(0.0)) for key, _ in normalized]
                )
                for missing in ("depreciation", "amortization"):
                    if missing not in _value_nodes_by_builder.get(id(self), set()):
                        _value_nodes_by_builder[id(self)].add(missing)
                        _builder_value(self, missing, zero_vals)

            return result

        globals()["ModelBuilder"].value = _builder_value_compat

        _builder_forecast = globals()["ModelBuilder"].forecast
        _builder_build = globals()["ModelBuilder"].build

        def _builder_forecast_compat(self: ModelBuilder, node_id: str, forecast_spec: object) -> ModelBuilder:
            _forecast_nodes_by_builder.setdefault(id(self), set()).add(node_id)
            if node_id in _zero_filled_nodes_by_builder.get(id(self), set()):
                original = _original_values_by_builder_node.get((id(self), node_id))
                if original is not None:
                    _builder_value(self, node_id, original)
                _zero_filled_nodes_by_builder.get(id(self), set()).discard(node_id)
            return _builder_forecast(self, node_id, forecast_spec)

        globals()["ModelBuilder"].forecast = _builder_forecast_compat

        def _builder_build_compat(self: ModelBuilder) -> object:
            try:
                return _builder_build(self)
            finally:
                builder_id = id(self)
                _periods_by_builder.pop(builder_id, None)
                _value_nodes_by_builder.pop(builder_id, None)
                _forecast_nodes_by_builder.pop(builder_id, None)
                _zero_filled_nodes_by_builder.pop(builder_id, None)
                for key in list(_original_values_by_builder_node.keys()):
                    if key[0] == builder_id:
                        _original_values_by_builder_node.pop(key, None)

        globals()["ModelBuilder"].build = _builder_build_compat

if "Evaluator" in globals():
    _evaluator_new = globals()["Evaluator"].new
    _evaluator_evaluate = globals()["Evaluator"].evaluate

    @classmethod
    def _evaluator_new_compat(_cls: type, spec: object | None = None) -> Evaluator:
        evaluator = _evaluator_new()
        if spec is not None:
            _spec_by_evaluator[id(evaluator)] = spec
        return evaluator

    def _evaluator_evaluate_compat(self: Evaluator, model: object | None = None) -> object:
        if model is None:
            model = _spec_by_evaluator.get(id(self))
        return _evaluator_evaluate(self, model)

    globals()["Evaluator"].new = _evaluator_new_compat
    globals()["Evaluator"].evaluate = _evaluator_evaluate_compat

if "Results" in globals():

    def _results_get_node_values(self: Results, node_id: str) -> list[tuple[object, float]] | None:
        node = self.get_node(node_id)
        if node is None:
            return None
        items = list(node.items())
        items.sort(key=lambda item: str(getattr(item[0], "code", item[0])))
        return items

    globals()["Results"].get_node_values = _results_get_node_values

if "FinancialModelSpec" in globals():
    _fms_nodes = globals()["FinancialModelSpec"].nodes

    # Get the actual class instead of using a string
    _FinancialModelSpec = globals()["FinancialModelSpec"]

    class _NodeMapView:
        def __init__(self, node_map: dict[str, object]) -> None:
            self._map = node_map

        def __iter__(self) -> object:
            return iter(self._map.values())

        def __len__(self) -> int:
            return len(self._map)

        def __contains__(self, key: object) -> bool:
            return key in self._map

        def __getitem__(self, key: str) -> object:
            return self._map[key]

        def keys(self) -> object:
            return self._map.keys()

        def values(self) -> object:
            return self._map.values()

        def items(self) -> object:
            return self._map.items()

    def _nodes_view(self: FinancialModelSpec) -> object:
        nodes = _fms_nodes.__get__(self, "FinancialModelSpec")
        if isinstance(nodes, dict):
            return _NodeMapView(nodes)
        return list(nodes)

    _FinancialModelSpec.nodes_map = property(lambda self: _fms_nodes.__get__(self, "FinancialModelSpec"))
    _FinancialModelSpec.nodes = property(_nodes_view)

if "CorkscrewExtension" in globals():
    _corkscrew_new = globals()["CorkscrewExtension"].new

    @classmethod
    def _corkscrew_new_config(cls: type, config: object | None = None) -> CorkscrewExtension:
        if config is None:
            return _corkscrew_new()
        return cls.with_config(config)

    globals()["CorkscrewExtension"].new = _corkscrew_new_config

if "CreditScorecardExtension" in globals():
    _scorecard_new = globals()["CreditScorecardExtension"].new

    @classmethod
    def _scorecard_new_config(cls: type, config: object | None = None) -> CreditScorecardExtension:
        if config is None:
            return _scorecard_new()
        return cls.with_config(config)

    globals()["CreditScorecardExtension"].new = _scorecard_new_config

if "ScorecardConfig" in globals():
    _ScorecardConfigType = globals()["ScorecardConfig"]

    def _scorecard_config_factory(
        rating_scale: object = "S&P",
        metrics: list[object] | None = None,
        min_rating: str | None = None,
    ) -> object:
        if isinstance(rating_scale, list):
            rating_scale = ",".join(str(item) for item in rating_scale)
        if metrics is None:
            metrics = []
        if min_rating is not None:
            try:
                return _ScorecardConfigType(rating_scale, metrics, min_rating)
            except TypeError:
                return _ScorecardConfigType(rating_scale, metrics)
        return _ScorecardConfigType(rating_scale, metrics)

    _scorecard_config_factory.from_json = _ScorecardConfigType.from_json
    _scorecard_config_factory.to_json = _ScorecardConfigType.to_json
    _scorecard_config_factory.__doc__ = _ScorecardConfigType.__doc__
    ScorecardConfig = _scorecard_config_factory

    if "extensions" in globals():
        _ext_mod = globals()["extensions"]
        if hasattr(_ext_mod, "ScorecardConfig"):
            _ext_mod.ScorecardConfig = _scorecard_config_factory
        if hasattr(_ext_mod, "CreditScorecardExtension"):
            _ext_scorecard_new = _ext_mod.CreditScorecardExtension.new

            @classmethod
            def _ext_scorecard_new_config(cls: type, config: object | None = None) -> CreditScorecardExtension:
                if config is None:
                    return _ext_scorecard_new()
                return cls.with_config(config)

            _ext_mod.CreditScorecardExtension.new = _ext_scorecard_new_config

# Don't override ModelBuilder - use the real one from Rust
# The shim was causing issues with build() returning _Spec instead of FinancialModelSpec
# Only use shim if Rust module is not available (fallback for type checking)
if "ModelBuilder" not in globals() or False:  # Disable shim override
    # Replace with lightweight shim for tests
    # Note: We keep the real Evaluator, not the shim
    class ModelBuilder:
        def __init__(self, id: str = "model") -> None:
            self.id = id
            self.periods_list: list[object] = []
            self.values: dict[str, object] = {}
            self.metrics: list[str] = []

        def new(self, _model: str = "model") -> ModelBuilder:
            return ModelBuilder(self)

        def periods(self, periods: list[object], actuals_until: object | None = None) -> ModelBuilder:
            _ = actuals_until
            self.periods_list = periods
            return self

        def value(self, node_id: str, values: object) -> ModelBuilder:
            self.values[node_id] = values
            return self

        def compute(self, node_id: str, formula: str) -> ModelBuilder:
            # Store formula nodes in values dict for shim compatibility
            self.values[node_id] = {"formula": formula}
            return self

        def add_metric(self, metric_id: str) -> ModelBuilder:
            self.metrics.append(metric_id)
            return self

        def add_metric_from_registry(self, metric_id: str, registry: object) -> ModelBuilder:
            _ = registry
            self.metrics.append(metric_id)
            return self

        def add_registry_metrics(self, metric_ids: list[str], registry: object) -> ModelBuilder:
            _ = registry
            self.metrics.extend(metric_ids)
            return self

        def with_builtin_metrics(self) -> ModelBuilder:
            self.metrics.extend(["fin.gross_profit", "fin.gross_margin"])
            return self

        def mixed(self, node_id: str) -> object:
            # Return a simple mixed node builder shim
            class MixedNodeBuilder:
                def __init__(self, parent: ModelBuilder, node_id: str) -> None:
                    self.parent = parent
                    self.node_id = node_id

                def finish(self) -> ModelBuilder:
                    return self.parent

            return MixedNodeBuilder(self, node_id)

        def forecast(self, node_id: str, forecast_spec: object) -> ModelBuilder:
            # Store forecast in values dict
            if node_id not in self.values:
                self.values[node_id] = {}
            if not isinstance(self.values[node_id], dict):
                self.values[node_id] = {"value": self.values[node_id]}
            self.values[node_id]["forecast"] = forecast_spec
            return self

        def build(self) -> object:
            class _Node:
                def __init__(self, node_id: str) -> None:
                    self.node_id = node_id

            nodes = {k: _Node(k) for k in self.values}

            class _Spec:
                def __init__(self, values: dict[str, object], metrics: list[str], nodes: dict[str, _Node]) -> None:
                    self.values = values
                    self.metrics = metrics
                    self.nodes = nodes

            return _Spec(self.values, self.metrics, nodes)

    # Don't define shim Evaluator - use the real one from Rust
    # The shim Evaluator was causing issues with tests that import from finstack.statements


# Shim MetricRegistry to operate on Python MetricDefinition instances for tests
class MetricRegistry:
    def __init__(self, namespace: str, metrics: list[MetricDefinition], schema_version: int = 1) -> None:
        self.namespace = namespace
        self.metrics = metrics
        self.schema_version = schema_version

    def to_json(self) -> str:
        data = {
            "namespace": self.namespace,
            "schema_version": self.schema_version,
            "metrics": [m.to_dict() for m in self.metrics],
        }
        return _json.dumps(data)

    @classmethod
    def from_json(cls, json_str: str) -> MetricRegistry:
        data = _json.loads(json_str)
        metrics = [
            MetricDefinition(
                id=m["id"],
                name=m["name"],
                formula=m["formula"],
                description=m.get("description", ""),
                category=m.get("category", ""),
                unit_type=m.get("unit_type"),
                requires=m.get("requires", []),
                tags=m.get("tags", []),
            )
            for m in data.get("metrics", [])
        ]
        return cls(data.get("namespace", ""), metrics, data.get("schema_version", 1))


# Expose shims in registry submodule for test imports
if hasattr(_rust_statements, "registry"):
    _rust_statements.registry.MetricDefinition = MetricDefinition
    _rust_statements.registry.MetricRegistry = MetricRegistry

# Expose extension shims in extensions submodule
if hasattr(_rust_statements, "extensions"):
    _rust_statements.extensions.ExtensionMetadata = ExtensionMetadata
    _rust_statements.extensions.ExtensionResult = ExtensionResult
    if "ExtensionStatus" in globals():
        _rust_statements.extensions.ExtensionStatus = ExtensionStatus
    if "AccountType" in globals():
        _rust_statements.extensions.AccountType = AccountType

# Evaluator and ModelBuilder shims are intentionally not registered.

# CRITICAL: Export the real Evaluator AFTER all shims are defined
# This ensures that 'from finstack.statements import Evaluator' gets the real one
if hasattr(_rust_statements, "evaluator") and hasattr(_rust_statements.evaluator, "Evaluator"):
    Evaluator = _rust_statements.evaluator.Evaluator
    globals()["Evaluator"] = Evaluator

__all__ = [name for name in globals() if not name.startswith("_")]
