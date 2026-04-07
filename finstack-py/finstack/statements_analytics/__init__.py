"""statements_analytics — canonical Python path for the finstack-statements-analytics crate.

Provides analysis tools, extension types, and financial statement templates.
"""

from __future__ import annotations

import importlib as _importlib

# Import via canonical sub-packages.
analysis = _importlib.import_module(".analysis", __name__)
extensions = _importlib.import_module(".extensions", __name__)
templates = _importlib.import_module(".templates", __name__)

__all__ = ["analysis", "extensions", "templates"]
