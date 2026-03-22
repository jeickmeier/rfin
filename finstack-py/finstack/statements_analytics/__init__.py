"""statements_analytics — canonical Python path for the finstack-statements-analytics crate.

Provides analysis tools, extension types, and financial statement templates.
The backward-compat paths ``finstack.statements.analysis``,
``finstack.statements.extensions``, and ``finstack.statements.templates``
remain fully intact and point to the same underlying Rust modules.
"""

from __future__ import annotations

from finstack.statements import analysis as analysis
from finstack.statements import extensions as extensions
from finstack.statements import templates as templates

__all__ = ["analysis", "extensions", "templates"]
