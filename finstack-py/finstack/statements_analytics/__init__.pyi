"""statements_analytics — canonical path for the finstack-statements-analytics crate.

Provides analysis tools, extension types, and financial statement templates.
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
