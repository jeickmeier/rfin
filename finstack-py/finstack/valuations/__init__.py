"""Valuations bindings (Rust).

Thin re-export of the ``finstack.valuations`` Rust extension module. The
``instruments`` and ``calibration`` subpackages have their own Python
``__init__.py`` that pulls the Rust surface through ``export_rust_members``
so IDEs pick up the paired ``.pyi`` stubs; they are imported explicitly here
to ensure they are registered as attributes of this package.
"""

from __future__ import annotations

from importlib import import_module as _import_module

from finstack import finstack as _finstack
from finstack._binding_exports import export_rust_members, set_public_all

# Export everything except the two subpackages that have their own Python
# __init__.py; those are loaded below via Python's import machinery.
export_rust_members(
    globals(),
    _finstack.valuations,
    package_name=__name__,
    excluded={"instruments", "calibration"},
)

calibration = _import_module(f"{__name__}.calibration")
instruments = _import_module(f"{__name__}.instruments")

set_public_all(globals(), helper_names={"annotations"})
