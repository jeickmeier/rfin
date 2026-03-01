"""Extension system for statement models."""

from __future__ import annotations
from .extensions import (
    ExtensionMetadata,
    ExtensionStatus,
    ExtensionResult,
    ExtensionContext,
    ExtensionRegistry,
    CorkscrewExtension,
    CreditScorecardExtension,
)

__all__ = [
    "ExtensionMetadata",
    "ExtensionStatus",
    "ExtensionResult",
    "ExtensionContext",
    "ExtensionRegistry",
    "CorkscrewExtension",
    "CreditScorecardExtension",
]
