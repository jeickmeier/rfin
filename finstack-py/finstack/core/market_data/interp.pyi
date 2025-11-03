"""Interpolation and extrapolation bindings.

Provides interpolation styles and extrapolation policies
for curve construction and evaluation.
"""

class InterpStyle:
    """Interpolation style for curve construction.

    Available styles:
    - Linear: Linear interpolation
    - MonotoneConvex: Monotone convex interpolation
    - CubicSpline: Cubic spline interpolation
    - LogLinear: Log-linear interpolation
    """

    @classmethod
    def from_name(cls, name: str) -> InterpStyle: ...
    """Create from string name.
    
    Parameters
    ----------
    name : str
        Style name (case-insensitive).
        
    Returns
    -------
    InterpStyle
        Interpolation style instance.
    """

    @property
    def name(self) -> str: ...
    """Get the style name.
    
    Returns
    -------
    str
        Human-readable style name.
    """

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

# Interpolation style constants
Linear: InterpStyle
MonotoneConvex: InterpStyle
CubicSpline: InterpStyle
LogLinear: InterpStyle

class ExtrapolationPolicy:
    """Extrapolation policy for curve evaluation.

    Available policies:
    - FlatZero: Extrapolate with zero
    - FlatForward: Extrapolate with last forward rate
    - Linear: Linear extrapolation
    - Constant: Constant extrapolation
    """

    @classmethod
    def from_name(cls, name: str) -> ExtrapolationPolicy: ...
    """Create from string name.
    
    Parameters
    ----------
    name : str
        Policy name (case-insensitive).
        
    Returns
    -------
    ExtrapolationPolicy
        Extrapolation policy instance.
    """

    @property
    def name(self) -> str: ...
    """Get the policy name.
    
    Returns
    -------
    str
        Human-readable policy name.
    """

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

# Extrapolation policy constants
FlatZero: ExtrapolationPolicy
FlatForward: ExtrapolationPolicy
Linear: ExtrapolationPolicy
Constant: ExtrapolationPolicy
