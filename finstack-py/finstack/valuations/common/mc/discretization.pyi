"""Discretization scheme descriptors for Monte Carlo simulation."""

from __future__ import annotations

class ExactGbmScheme:
    """Exact discretization for Geometric Brownian Motion.

    Uses the analytical log-normal solution (no discretization error):
        S_{t+dt} = S_t * exp((r - q - 0.5*sigma^2)*dt + sigma*sqrt(dt)*Z)

    This is the recommended scheme for GBM processes.
    """

    def __init__(self) -> None: ...

class EulerMaruyamaScheme:
    """Euler-Maruyama discretization (first-order explicit).

    Generic scheme for any SDE:
        X_{t+dt} = X_t + mu(t, X_t)*dt + sigma(t, X_t)*sqrt(dt)*Z

    Properties:
        - Weak order: O(dt)
        - Strong order: O(sqrt(dt))

    Use when no exact or specialized scheme is available.
    """

    def __init__(self) -> None: ...

class LogEulerScheme:
    """Log-Euler discretization.

    Euler scheme applied in log-space for processes with multiplicative noise.
    More stable than standard Euler for GBM-like processes but less accurate
    than the exact scheme.
    """

    def __init__(self) -> None: ...

class MilsteinScheme:
    """Milstein discretization (higher-order strong convergence).

    Adds a correction term to Euler-Maruyama:
        X_{t+dt} = X_t + mu*dt + sigma*sqrt(dt)*Z + 0.5*sigma*sigma'*(Z^2 - 1)*dt

    Properties:
        - Weak order: O(dt) (same as Euler)
        - Strong order: O(dt) (better than Euler's O(sqrt(dt)))

    Note: Only exact for processes with proportional volatility (GBM-like).
    """

    def __init__(self) -> None: ...

class LogMilsteinScheme:
    """Log-Milstein discretization.

    Milstein scheme applied in log-space. More stable for GBM-like processes.
    """

    def __init__(self) -> None: ...

class QeHestonScheme:
    """Quadratic-Exponential (QE) scheme for Heston stochastic volatility.

    Andersen (2008) scheme that ensures positive variance while maintaining
    good accuracy.

    Args:
        psi_c: Critical psi value (default 1.5)
        use_exact_integrated_variance: If True, uses exact conditional
            expectation for integrated variance. Default: False.
    """

    def __init__(
        self,
        psi_c: float = 1.5,
        use_exact_integrated_variance: bool = False,
    ) -> None: ...

    @property
    def psi_c(self) -> float:
        """Critical psi value for the QE switch."""
        ...

    @property
    def use_exact_integrated_variance(self) -> bool:
        """Whether exact integrated variance is used."""
        ...

class QeCirScheme:
    """Quadratic-Exponential (QE) scheme for CIR process.

    Ensures positive values while maintaining accuracy.

    Args:
        psi_c: Critical psi value (default 1.5)
    """

    def __init__(self, psi_c: float = 1.5) -> None: ...

    @property
    def psi_c(self) -> float:
        """Critical psi value."""
        ...

class ExactHullWhite1FScheme:
    """Exact discretization for Hull-White one-factor model.

    Uses the analytical solution for the OU process:
        r_{t+dt} = r_t * exp(-kappa*dt) + theta*(1 - exp(-kappa*dt))
                   + sigma * sqrt((1 - exp(-2*kappa*dt)) / (2*kappa)) * Z
    """

    def __init__(self) -> None: ...

class JumpEulerScheme:
    """Jump-Euler discretization for jump-diffusion processes.

    Combines Euler-Maruyama for the continuous part with Poisson arrival
    and log-normal jumps for the jump component.

    Suitable for Merton jump-diffusion and Bates models.
    """

    def __init__(self) -> None: ...

class ExactSchwartzSmithScheme:
    """Exact discretization for Schwartz-Smith two-factor model.

    Uses the analytical solution for the coupled OU + ABM system
    with correlation.
    """

    def __init__(self) -> None: ...

__all__ = [
    "ExactGbmScheme",
    "EulerMaruyamaScheme",
    "LogEulerScheme",
    "MilsteinScheme",
    "LogMilsteinScheme",
    "QeHestonScheme",
    "QeCirScheme",
    "ExactHullWhite1FScheme",
    "JumpEulerScheme",
    "ExactSchwartzSmithScheme",
]
