"""Core bindings (Rust).

Thin re-export of the ``finstack.core`` Rust extension module.
"""

from __future__ import annotations

from finstack import finstack as _finstack
from finstack._binding_exports import export_rust_members, set_public_all

export_rust_members(globals(), _finstack.core, package_name=__name__)
set_public_all(globals(), helper_names={"annotations"})
