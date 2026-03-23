"""statements_analytics — canonical Python path for the finstack-statements-analytics crate.

Provides analysis tools, extension types, and financial statement templates.
The backward-compat paths ``finstack.statements.analysis`` and
``finstack.statements.templates`` emit :class:`DeprecationWarning` on import;
prefer the ``finstack.statements_analytics.*`` sub-packages for new code.
"""

from __future__ import annotations

import importlib as _importlib

# Import via canonical sub-packages — avoids triggering deprecated shims.
analysis = _importlib.import_module(".analysis", __name__)
extensions = _importlib.import_module(".extensions", __name__)
templates = _importlib.import_module(".templates", __name__)

__all__ = ["analysis", "extensions", "templates"]
