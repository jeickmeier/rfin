"""XVA (Valuation Adjustments) framework: CVA, exposure profiling, netting, and collateral."""

from __future__ import annotations
from datetime import date
from typing import Optional

from ..core.market_data.context import MarketContext
from ..core.market_data.term_structures import DiscountCurve, HazardCurve
from ..core.currency import Currency

class FundingConfig:
    """Funding cost/benefit configuration for FVA calculations."""
    def __init__(
        self,
        funding_spread_bps: float,
        funding_benefit_bps: float | None = None,
    ) -> None: ...
    @property
    def funding_spread_bps(self) -> float: ...
    @property
    def funding_benefit_bps(self) -> float | None: ...
    @property
    def effective_benefit_bps(self) -> float: ...
    def __repr__(self) -> str: ...

class XvaConfig:
    """Configuration for XVA calculations."""
    def __init__(
        self,
        time_grid: list[float] | None = None,
        recovery_rate: float = 0.40,
        own_recovery_rate: float | None = None,
        funding: FundingConfig | None = None,
    ) -> None: ...
    @property
    def time_grid(self) -> list[float]: ...
    @property
    def recovery_rate(self) -> float: ...
    @property
    def own_recovery_rate(self) -> float | None: ...
    @property
    def funding(self) -> FundingConfig | None: ...
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
        reporting_currency: Currency | str | None = None,
    ) -> None: ...
    @property
    def id(self) -> str: ...
    @property
    def counterparty_id(self) -> str: ...
    @property
    def reporting_currency(self) -> Currency | None: ...
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
    @property
    def diagnostics(self) -> dict[str, int] | None: ...
    def __repr__(self) -> str: ...

class XvaResult:
    """Result of XVA calculations."""
    @property
    def cva(self) -> float: ...
    @property
    def dva(self) -> float | None: ...
    @property
    def fva(self) -> float | None: ...
    @property
    def bilateral_cva(self) -> float | None: ...
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

def compute_dva(
    exposure_profile: ExposureProfile,
    own_hazard_curve: HazardCurve,
    discount_curve: DiscountCurve,
    own_recovery_rate: float,
) -> float:
    """Compute debit valuation adjustment from the negative exposure profile."""
    ...

def compute_fva(
    exposure_profile: ExposureProfile,
    discount_curve: DiscountCurve,
    funding_spread_bps: float,
    funding_benefit_bps: float,
) -> float:
    """Compute funding valuation adjustment from the exposure profile."""
    ...

def compute_bilateral_xva(
    exposure_profile: ExposureProfile,
    counterparty_hazard_curve: HazardCurve,
    own_hazard_curve: HazardCurve,
    discount_curve: DiscountCurve,
    counterparty_recovery_rate: float,
    own_recovery_rate: float,
    funding: FundingConfig | None = None,
) -> XvaResult:
    """Compute bilateral XVA including CVA, DVA, and optional FVA."""
    ...

__all__ = [
    "FundingConfig",
    "XvaConfig",
    "CsaTerms",
    "NettingSet",
    "ExposureProfile",
    "XvaResult",
    "apply_netting",
    "apply_collateral",
    "compute_exposure_profile",
    "compute_cva",
    "compute_dva",
    "compute_fva",
    "compute_bilateral_xva",
]
