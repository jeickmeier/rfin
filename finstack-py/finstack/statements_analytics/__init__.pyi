"""statements_analytics — canonical path for the finstack-statements-analytics crate.

Provides analysis tools, extension types, and financial statement templates.
The backward-compat paths ``finstack.statements.analysis``,
``finstack.statements.extensions``, and ``finstack.statements.templates``
remain fully intact.

Note:
    ``finstack.statements.analysis`` / ``.extensions`` / ``.templates`` remain
    importable as backward-compatible aliases pointing to this package.
"""

from __future__ import annotations
from . import analysis as analysis
from . import extensions as extensions
from . import templates as templates

__all__ = [
    "analysis",
    "extensions",
    "templates",
]
