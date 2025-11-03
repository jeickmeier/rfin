"""Structured credit instruments (ABS, RMBS, CMBS, CLO)."""

from .mod import StructuredCredit
from . import waterfall

__all__ = ["StructuredCredit", "waterfall"]
