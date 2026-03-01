"""Helpers to convert string labels into strongly-typed valuation enums.

These utilities mirror the Rust parsing functions and only perform label
normalization and enum construction. They do not execute any pricing logic.
"""

from __future__ import annotations
from .parameters import OptionType, ExerciseStyle, SettlementType, PayReceive

def option_type(label: str) -> OptionType:
    """Parse a label (``\"call\"``/``\"put\"``) into :class:`OptionType`."""
    ...

def exercise_style(label: str) -> ExerciseStyle:
    """Parse an exercise style label (``\"european\"``/``\"american\"``/``\"bermudan\"``)."""
    ...

def settlement_type(label: str) -> SettlementType:
    """Parse a settlement type label (``\"physical\"``/``\"cash\"``)."""
    ...

def pay_receive(label: str) -> PayReceive:
    """Parse a leg direction label (``\"pay_fixed\"``/``\"receive_fixed\"``)."""
    ...
