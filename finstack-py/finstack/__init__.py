"""High-level Python API built on the finstack Rust core.

Import directly from :mod:`finstack` to work with currencies, configuration,
money arithmetic, business-day calendars, and market data primitives. The
compiled extension underneath provides rich docstrings and type hints so these
re-exports stay discoverable in IDEs.
"""

from collections.abc import MutableMapping as _MutableMapping
from pathlib import Path as _Path
import sys as _sys
import types as _types
from typing import Any as _Any

from . import finstack as _finstack

__all__ = tuple(
    getattr(
        _finstack,
        "__all__",
        [name for name in dir(_finstack) if not name.startswith("_")],
    )
)

for _name in __all__:
    if hasattr(_finstack, _name):
        globals()[_name] = getattr(_finstack, _name)


def _walk_and_register_nested(
    _parent_mod: _Any,
    _qualname: str,
    _skip_top_level: bool = False,
    _modules: _MutableMapping[str, _Any] | None = None,
    _module_type: type = _types.ModuleType,
) -> None:
    """Recursively register nested submodules under sys.modules.

    Only registers nested submodules (not the top-level module itself) when
    skip_top_level is True. This allows Python packages to coexist with Rust
    submodules.
    """
    import sys

    if _modules is None:
        _modules = sys.modules
    _seen: set[int] = set()

    def _recurse(_mod: object, _qname: str, _depth: int) -> None:
        if id(_mod) in _seen:
            return
        _seen.add(id(_mod))

        for _attr_name in dir(_mod):
            if _attr_name.startswith("_"):
                continue
            try:
                _attr = getattr(_mod, _attr_name)
            except AttributeError:
                continue
            if isinstance(_attr, _module_type):
                _fqname = f"{__name__}.{_qname}.{_attr_name}"
                # Only register if not skipping top level, or if we're at depth > 0
                if not _skip_top_level or _depth > 0:
                    _modules[_fqname] = _attr
                _recurse(_attr, f"{_qname}.{_attr_name}", _depth + 1)

    _recurse(_parent_mod, _qualname, 0)


# Get the package directory
_pkg_path = _Path(__file__).parent


def _setup_hybrid_module(_rust_mod: _Any, _qualname: str, _pkg_dir: _Path) -> _Any:
    """Set up a module that combines Python package with Rust bindings.

    If a Python package exists (has __init__.py), use that and DON'T register
    the Rust module in sys.modules for the top level.
    If no Python package, register the Rust module directly.
    """
    if _pkg_dir.is_dir() and (_pkg_dir / "__init__.py").exists():
        # Python package exists - import it and let it handle Rust re-exports
        # Don't register in sys.modules - let Python's import system handle it
        import importlib

        _py_mod = importlib.import_module(f".{_qualname}", __name__)
        # Register nested Rust submodules (like core.expr, core.dates, etc.)
        _walk_and_register_nested(_rust_mod, _qualname, _skip_top_level=True)
        return _py_mod
    else:
        # No Python package - use Rust module directly
        _sys.modules[f"{__name__}.{_qualname}"] = _rust_mod
        _walk_and_register_nested(_rust_mod, _qualname, _skip_top_level=False)
        return _rust_mod


# Set up each submodule
_rust_core = _finstack.core
_core = _setup_hybrid_module(_rust_core, "core", _pkg_path / "core")
globals()["core"] = _core

_rust_scenarios = _finstack.scenarios
_scenarios = _setup_hybrid_module(_rust_scenarios, "scenarios", _pkg_path / "scenarios")
globals()["scenarios"] = _scenarios

_rust_valuations = _finstack.valuations
_valuations = _setup_hybrid_module(_rust_valuations, "valuations", _pkg_path / "valuations")
globals()["valuations"] = _valuations

_rust_statements = _finstack.statements
_statements = _setup_hybrid_module(_rust_statements, "statements", _pkg_path / "statements")
globals()["statements"] = _statements

_rust_portfolio = _finstack.portfolio
_portfolio = _setup_hybrid_module(_rust_portfolio, "portfolio", _pkg_path / "portfolio")
globals()["portfolio"] = _portfolio

del (
    _finstack,
    _name,
    _rust_core,
    _core,
    _rust_scenarios,
    _scenarios,
    _rust_valuations,
    _valuations,
    _rust_statements,
    _statements,
    _rust_portfolio,
    _portfolio,
    _types,
    _setup_hybrid_module,
    _Any,
    _MutableMapping,
    _Path,
    _pkg_path,
)
