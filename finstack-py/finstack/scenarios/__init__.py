"""Scenarios module wrapper - re-exports from Rust extension with Python additions.

This module provides a Python package wrapper around the Rust scenarios module,
allowing for additional pure-Python helper modules alongside the Rust bindings.
"""

import sys as _sys
import types as _types

# Import the Rust module
try:
    from finstack import finstack as _finstack

    _rust_scenarios = _finstack.scenarios
    Currency = _finstack.Currency

    # Re-export everything from the Rust scenarios module
    for _name in dir(_rust_scenarios):
        if not _name.startswith("_"):
            _attr = getattr(_rust_scenarios, _name)
            globals()[_name] = _attr
            # Register submodules in sys.modules for direct imports
            if isinstance(_attr, _types.ModuleType):
                _sys.modules[f"{__name__}.{_name}"] = _attr

    # Aliases for parity with docs/tests
    if hasattr(_rust_scenarios, "CurveKind"):
        CurveKind = _rust_scenarios.CurveKind
        if not hasattr(CurveKind, "Forward") and hasattr(CurveKind, "Forecast"):
            CurveKind.Forward = CurveKind.Forecast
        if not hasattr(CurveKind, "Hazard") and hasattr(CurveKind, "ParCDS"):
            CurveKind.Hazard = CurveKind.ParCDS
        if not hasattr(CurveKind, "DISCOUNT") and hasattr(CurveKind, "Discount"):
            CurveKind.DISCOUNT = CurveKind.Discount
        if not hasattr(CurveKind, "FORWARD"):
            if hasattr(CurveKind, "Forecast"):
                CurveKind.FORWARD = CurveKind.Forecast
            elif hasattr(CurveKind, "Forward"):
                CurveKind.FORWARD = CurveKind.Forward
        if not hasattr(CurveKind, "HAZARD"):
            if hasattr(CurveKind, "ParCDS"):
                CurveKind.HAZARD = CurveKind.ParCDS
            elif hasattr(CurveKind, "Hazard"):
                CurveKind.HAZARD = CurveKind.Hazard
        if not hasattr(CurveKind, "INFLATION") and hasattr(CurveKind, "Inflation"):
            CurveKind.INFLATION = CurveKind.Inflation

    # Monkey-patch ScenarioSpec with property accessors expected by tests
    if hasattr(_rust_scenarios, "ScenarioSpec"):
        ScenarioSpec = _rust_scenarios.ScenarioSpec
        # PyO3 getters are already descriptors, but we need to make them work
        # both as properties (scenario.id) and as callables (scenario.id())
        _spec_id_attr = _rust_scenarios.ScenarioSpec.id
        _spec_name_attr = _rust_scenarios.ScenarioSpec.name
        _spec_description_attr = _rust_scenarios.ScenarioSpec.description
        _spec_priority_attr = _rust_scenarios.ScenarioSpec.priority
        _spec_operations_attr = _rust_scenarios.ScenarioSpec.operations

        class _PropertyOrCallable:
            """Descriptor that works as both property and callable."""

            def __init__(self, attr: object) -> None:
                self.attr = attr

            def __get__(self, obj: object | None, objtype: type | None = None) -> object:
                if obj is None:
                    return self
                value = self.attr.__get__(obj, objtype)
                # Create a callable that returns the value

                # Make it compare equal to the value for property-style access
                # We'll use a custom class that behaves like the value
                class _ValueWrapper:
                    def __init__(self, val: object) -> None:
                        self._val = val

                    def __call__(self) -> object:
                        return self._val

                    def __eq__(self, other: object) -> bool:
                        return self._val == other

                    def __ne__(self, other: object) -> bool:
                        return self._val != other

                    __hash__ = None

                    def __repr__(self) -> str:
                        return repr(self._val)

                    def __str__(self) -> str:
                        return str(self._val)

                    def __len__(self) -> int:
                        return len(self._val)

                    def __iter__(self) -> object:
                        return iter(self._val)

                    def __getitem__(self, key: object) -> object:
                        return self._val[key]

                return _ValueWrapper(value)

        ScenarioSpec.id = _PropertyOrCallable(_spec_id_attr)
        ScenarioSpec.name = _PropertyOrCallable(_spec_name_attr)
        ScenarioSpec.description = _PropertyOrCallable(_spec_description_attr)
        ScenarioSpec.priority = _PropertyOrCallable(_spec_priority_attr)
        ScenarioSpec.operations = _PropertyOrCallable(_spec_operations_attr)
        ScenarioSpec.scenario_id = property(lambda self: self.id)

        _scenario_from_json = ScenarioSpec.from_json

        @classmethod
        def _scenario_from_json_compat(_cls: type, json_str: str) -> ScenarioSpec:
            if '"scenario_id"' in json_str and '"id"' not in json_str:
                try:
                    import json as _json

                    data = _json.loads(json_str)
                    if "scenario_id" in data and "id" not in data:
                        data["id"] = data.pop("scenario_id")
                        json_str = _json.dumps(data)
                except _json.JSONDecodeError:
                    # Ignore JSON parsing errors - use original string
                    pass
            return _scenario_from_json(json_str)

        ScenarioSpec.from_json = _scenario_from_json_compat

    if hasattr(_rust_scenarios, "TenorMatchMode"):
        TenorMatchMode = _rust_scenarios.TenorMatchMode
        if not hasattr(TenorMatchMode, "EXACT") and hasattr(TenorMatchMode, "Exact"):
            TenorMatchMode.EXACT = TenorMatchMode.Exact
        if not hasattr(TenorMatchMode, "INTERPOLATE") and hasattr(TenorMatchMode, "Interpolate"):
            TenorMatchMode.INTERPOLATE = TenorMatchMode.Interpolate

    if hasattr(_rust_scenarios, "VolSurfaceKind"):
        VolSurfaceKind = _rust_scenarios.VolSurfaceKind
        if not hasattr(VolSurfaceKind, "EQUITY") and hasattr(VolSurfaceKind, "Equity"):
            VolSurfaceKind.EQUITY = VolSurfaceKind.Equity

    if hasattr(_rust_scenarios, "OperationSpec"):
        OperationSpec = _rust_scenarios.OperationSpec
        _op_curve_node_bp = OperationSpec.curve_node_bp
        _op_equity_price_pct = OperationSpec.equity_price_pct
        _op_fx_pct = OperationSpec.market_fx_pct
        _op_roll_forward = OperationSpec.time_roll_forward

        def _curve_node_bp_compat(
            curve_kind: object,
            curve_id: str,
            tenor: object,
            bp: float,
            _match_mode: object | None = None,
        ) -> OperationSpec:
            nodes = tenor
            if isinstance(nodes, str):
                nodes = [(nodes, bp)]
            elif isinstance(nodes, (list, tuple)) and nodes and isinstance(nodes[0], str):
                nodes = [(node, bp) for node in nodes]
            return _op_curve_node_bp(curve_kind, curve_id, nodes, _match_mode)

        def _equity_price_pct_compat(ids: object, pct: float) -> OperationSpec:
            if isinstance(ids, str):
                ids = [ids]
            return _op_equity_price_pct(ids, pct)

        def _fx_pct_compat(base: object, quote: object, pct: float) -> OperationSpec:
            if isinstance(base, str) or isinstance(quote, str):
                base = Currency(base) if isinstance(base, str) else base
                quote = Currency(quote) if isinstance(quote, str) else quote
            return _op_fx_pct(base, quote, pct)

        def _roll_forward_compat(
            tenor: str, apply_shocks: bool = True, roll_mode: object | None = None
        ) -> OperationSpec:
            return _op_roll_forward(tenor, apply_shocks, roll_mode)

        OperationSpec.curve_node_bp = staticmethod(_curve_node_bp_compat)
        OperationSpec.equity_price_pct = staticmethod(_equity_price_pct_compat)
        OperationSpec.market_fx_pct = staticmethod(_fx_pct_compat)
        OperationSpec.time_roll_forward = staticmethod(_roll_forward_compat)

    if hasattr(_rust_scenarios, "ScenarioEngine"):
        ScenarioEngine = _rust_scenarios.ScenarioEngine
        _engine_apply = ScenarioEngine.apply

        def _engine_apply_compat(self: ScenarioEngine, scenario: object, context: object) -> object:
            from datetime import date as _date

            from finstack.statements import FinancialModelSpec

            # If context is already a Rust ExecutionContext, use it directly
            if hasattr(context, "__class__") and context.__class__.__module__.startswith("finstack"):
                # Check if it's specifically a MarketContext
                if "MarketContext" in str(context.__class__):
                    # MarketContext passed directly - wrap it
                    as_of = _date.today()
                    model = FinancialModelSpec("scenario", [])
                    exec_ctx = _rust_scenarios.ExecutionContext(context, model, as_of)
                    return _engine_apply(self, scenario, exec_ctx)
                else:
                    # Already an ExecutionContext
                    return _engine_apply(self, scenario, context)

            # Otherwise, extract components and create Rust ExecutionContext
            as_of = getattr(context, "as_of", None) or _date.today()
            market = getattr(context, "market", None)
            model = getattr(context, "model", FinancialModelSpec("scenario", []))

            if market is None:
                # Create a minimal market context if none provided
                market = _rust_scenarios.MarketContext()

            exec_ctx = _rust_scenarios.ExecutionContext(market, model, as_of)
            return _engine_apply(self, scenario, exec_ctx)

        ScenarioEngine.apply = _engine_apply_compat

except ImportError:
    # Fallback for type checking / stub generation
    pass

# Import Python helper modules
from . import builder, dsl

__all__ = [
    "ApplicationReport",
    "Compounding",
    "CurveKind",
    "ExecutionContext",
    "OperationSpec",
    "RateBindingSpec",
    "RollForwardReport",
    "ScenarioEngine",
    "ScenarioSpec",
    "TenorMatchMode",
    "VolSurfaceKind",
    "builder",
    "dsl",
]
