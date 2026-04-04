"""Tree-based bond pricing configuration and OAS calculation."""

from __future__ import annotations
import datetime
from .....core.market_data.context import MarketContext

class TreeModelChoice:
    """Choice of short-rate model for the bond pricing tree.

    Use the class methods to create instances:

    - ``TreeModelChoice.ho_lee()`` — Ho-Lee / BDT model (default)
    - ``TreeModelChoice.hull_white(kappa, sigma)`` — Hull-White 1-factor
    - ``TreeModelChoice.hull_white_calibrated(surface_id)`` — Hull-White calibrated to swaptions
    """

    @classmethod
    def ho_lee(cls) -> TreeModelChoice:
        """Ho-Lee / BDT model (default)."""
        ...
    @classmethod
    def hull_white(cls, kappa: float, sigma: float) -> TreeModelChoice:
        """Hull-White 1-factor with user-specified parameters.

        Parameters
        ----------
        kappa
            Mean reversion speed (e.g., 0.03 for 3%).
        sigma
            Short rate volatility (e.g., 0.01 for 100 bp).
        """
        ...
    @classmethod
    def hull_white_calibrated(cls, swaption_vol_surface_id: str) -> TreeModelChoice:
        """Hull-White 1-factor calibrated to co-terminal swaptions.

        Parameters
        ----------
        swaption_vol_surface_id
            ID of the swaption volatility surface in the market context.
        """
        ...
    @property
    def model_type(self) -> str:
        """Model type: ``"HoLee"``, ``"HullWhite"``, or ``"HullWhiteCalibratedToSwaptions"``."""
        ...
    @property
    def kappa(self) -> float | None:
        """Mean reversion speed (Hull-White only)."""
        ...
    @property
    def sigma(self) -> float | None:
        """Short rate volatility (Hull-White only)."""
        ...
    @property
    def swaption_vol_surface_id(self) -> str | None:
        """Swaption vol surface ID (calibrated Hull-White only)."""
        ...
    def __repr__(self) -> str: ...

class TreePricerConfig:
    """Configuration for tree-based bond pricing (callable/putable bonds, OAS).

    Controls the tree structure, convergence settings, and solver parameters
    for option-adjusted spread calculations.
    """

    def __init__(
        self,
        *,
        tree_steps: int = 100,
        volatility: float = 0.01,
        tolerance: float = 1e-6,
        max_iterations: int = 50,
        initial_bracket_size_bp: float | None = 1000.0,
        mean_reversion: float | None = None,
        tree_model: TreeModelChoice | None = None,
    ) -> None: ...
    @classmethod
    def production_ho_lee(cls, normal_vol: float) -> TreePricerConfig:
        """Production Ho-Lee with normal volatility (e.g., 0.01 = 100 bps)."""
        ...
    @classmethod
    def production_bdt(cls, lognormal_vol: float) -> TreePricerConfig:
        """Production BDT with lognormal volatility (e.g., 0.20 = 20%)."""
        ...
    @classmethod
    def default_bdt(cls) -> TreePricerConfig:
        """Default BDT with 20% lognormal volatility."""
        ...
    @classmethod
    def high_precision(cls, calibrated_vol: float) -> TreePricerConfig:
        """High-precision configuration (200 steps, < 0.5 bp accuracy)."""
        ...
    @classmethod
    def fast(cls, calibrated_vol: float) -> TreePricerConfig:
        """Fast screening configuration (50 steps, ~2-5 bp accuracy)."""
        ...
    @classmethod
    def hull_white(cls, kappa: float, sigma: float) -> TreePricerConfig:
        """Hull-White 1-factor with user-specified parameters."""
        ...
    @classmethod
    def hull_white_calibrated(cls, swaption_vol_surface_id: str) -> TreePricerConfig:
        """Hull-White 1-factor calibrated to swaption volatilities."""
        ...
    @property
    def tree_steps(self) -> int: ...
    @property
    def volatility(self) -> float: ...
    @property
    def tolerance(self) -> float: ...
    @property
    def max_iterations(self) -> int: ...
    @property
    def initial_bracket_size_bp(self) -> float | None: ...
    @property
    def mean_reversion(self) -> float | None: ...
    @property
    def tree_model(self) -> TreeModelChoice: ...
    def __repr__(self) -> str: ...

class TreePricer:
    """Tree-based pricer for bonds with embedded options and OAS calculations.

    Provides methods for calculating option-adjusted spread (OAS) and
    pricing bonds at a given OAS.
    """

    def __init__(self, config: TreePricerConfig | None = None) -> None:
        """Create a tree pricer.

        Parameters
        ----------
        config
            Custom configuration. If ``None``, uses the default.
        """
        ...
    def calculate_oas(
        self,
        bond: "Bond",
        market: MarketContext,
        as_of: datetime.date,
        clean_price_pct: float,
    ) -> float:
        """Calculate option-adjusted spread (OAS) for a bond.

        Parameters
        ----------
        bond
            Bond instrument (may have call/put options).
        market
            Market data including discount and optionally hazard curves.
        as_of
            Valuation date.
        clean_price_pct
            Market clean price as percentage of par (e.g., 98.5).

        Returns
        -------
        float
            OAS in basis points (e.g., 150.0 means 150 bp).
        """
        ...
    @staticmethod
    def price_from_oas(
        bond: "Bond",
        market: MarketContext,
        as_of: datetime.date,
        oas_decimal: float,
    ) -> float:
        """Price a bond at a given OAS using the short-rate tree.

        Parameters
        ----------
        bond
            Bond instrument.
        market
            Market data.
        as_of
            Valuation date.
        oas_decimal
            OAS in decimal form (e.g., 0.015 = 150 bp).

        Returns
        -------
        float
            Dirty price in currency units.
        """
        ...
    def __repr__(self) -> str: ...

from . import Bond
