"""XVA (Valuation Adjustments) framework: CVA, exposure profiling, netting, and collateral."""

from __future__ import annotations
from datetime import date
from typing import Optional

from ..core.market_data.context import MarketContext
from ..core.market_data.term_structures import DiscountCurve, HazardCurve

class XvaConfig:
    """Configuration for XVA calculations."""
    def __init__(
        self,
        time_grid: list[float] | None = None,
        recovery_rate: float = 0.40,
        include_wrong_way_risk: bool = False,
    ) -> None: ...
    @property
    def time_grid(self) -> list[float]: ...
    @property
    def recovery_rate(self) -> float: ...
    @property
    def include_wrong_way_risk(self) -> bool: ...
    def __repr__(self) -> str: ...

class CsaTerms:
    """Credit Support Annex terms for collateralization."""
    def __init__(
        self,
        threshold: float,
        mta: float,
        mpor_days: int = 10,
        independent_amount: float = 0.0,
    ) -> None: ...
    @property
    def threshold(self) -> float: ...
    @property
    def mta(self) -> float: ...
    @property
    def mpor_days(self) -> int: ...
    @property
    def independent_amount(self) -> float: ...
    def __repr__(self) -> str: ...

class NettingSet:
    """Netting set specification under an ISDA master agreement."""
    def __init__(
        self,
        id: str,
        counterparty_id: str,
        csa: CsaTerms | None = None,
    ) -> None: ...
    @property
    def id(self) -> str: ...
    @property
    def counterparty_id(self) -> str: ...
    def __repr__(self) -> str: ...

class ExposureProfile:
    """Exposure profile computed at each time grid point."""
    @property
    def times(self) -> list[float]: ...
    @property
    def mtm_values(self) -> list[float]: ...
    @property
    def epe(self) -> list[float]: ...
    @property
    def ene(self) -> list[float]: ...
    def __repr__(self) -> str: ...

class XvaResult:
    """Result of XVA calculations."""
    @property
    def cva(self) -> float: ...
    @property
    def epe_profile(self) -> list[tuple[float, float]]: ...
    @property
    def ene_profile(self) -> list[tuple[float, float]]: ...
    @property
    def pfe_profile(self) -> list[tuple[float, float]]: ...
    @property
    def max_pfe(self) -> float: ...
    @property
    def effective_epe_profile(self) -> list[tuple[float, float]]: ...
    @property
    def effective_epe(self) -> float: ...
    def __repr__(self) -> str: ...

def apply_netting(instrument_values: list[float]) -> float:
    """Apply close-out netting to instrument MtM values."""
    ...

def apply_collateral(gross_exposure: float, csa: CsaTerms) -> float:
    """Apply CSA collateral terms to reduce gross exposure."""
    ...

def compute_exposure_profile(
    instruments: list,
    market: MarketContext,
    as_of: date,
    config: XvaConfig,
    netting_set: NettingSet,
) -> ExposureProfile:
    """Compute exposure profile for a portfolio of instruments."""
    ...

def compute_cva(
    exposure_profile: ExposureProfile,
    hazard_curve: HazardCurve,
    discount_curve: DiscountCurve,
    recovery_rate: float,
) -> XvaResult:
    """Compute unilateral CVA from an exposure profile."""
    ...

__all__ = [
    "XvaConfig",
    "CsaTerms",
    "NettingSet",
    "ExposureProfile",
    "XvaResult",
    "apply_netting",
    "apply_collateral",
    "compute_exposure_profile",
    "compute_cva",
]
