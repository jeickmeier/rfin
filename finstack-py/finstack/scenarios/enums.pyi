"""Enum bindings for scenarios Python module."""

from __future__ import annotations
from typing import ClassVar

class CurveKind:
    """Identifies which family of curve an operation targets.

    Use class attributes: CurveKind.Discount, CurveKind.Forward, etc.

    Examples:
        >>> from finstack.scenarios import CurveKind
        >>> kind = CurveKind.Discount
    """

    # Class attributes
    Discount: ClassVar[CurveKind]
    Forward: ClassVar[CurveKind]
    ParCDS: ClassVar[CurveKind]
    Inflation: ClassVar[CurveKind]
    Commodity: ClassVar[CurveKind]
    VolIndex: ClassVar[CurveKind]

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class VolSurfaceKind:
    """Identifies which category of volatility surface an operation targets.

    Use class attributes: VolSurfaceKind.Equity, VolSurfaceKind.Credit, etc.

    Examples:
        >>> from finstack.scenarios import VolSurfaceKind
        >>> kind = VolSurfaceKind.Equity
    """

    # Class attributes
    Equity: ClassVar[VolSurfaceKind]
    Credit: ClassVar[VolSurfaceKind]
    Swaption: ClassVar[VolSurfaceKind]

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class TenorMatchMode:
    """Strategy for aligning requested tenor bumps with curve pillars.

    Use class attributes: TenorMatchMode.Exact, TenorMatchMode.Interpolate

    Examples:
        >>> from finstack.scenarios import TenorMatchMode
        >>> mode = TenorMatchMode.Interpolate
    """

    # Class attributes
    Exact: ClassVar[TenorMatchMode]
    Interpolate: ClassVar[TenorMatchMode]

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
