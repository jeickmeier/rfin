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
