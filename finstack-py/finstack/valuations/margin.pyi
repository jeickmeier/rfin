"""Margin and collateral management type stubs."""

from __future__ import annotations

from typing import Any

from finstack.core.currency import Currency
from finstack.core.money import Money

class MarginTenor:
    """Margin call frequency."""

    DAILY: MarginTenor
    WEEKLY: MarginTenor
    MONTHLY: MarginTenor
    ON_DEMAND: MarginTenor

    @property
    def name(self) -> str: ...

class ImMethodology:
    """Initial margin methodology."""

    HAIRCUT: ImMethodology
    SIMM: ImMethodology
    SCHEDULE: ImMethodology
    INTERNAL_MODEL: ImMethodology
    CLEARING_HOUSE: ImMethodology

    @property
    def name(self) -> str: ...

class MarginCallTiming:
    def __init__(
        self,
        *,
        notification_deadline_hours: int | None = ...,
        response_deadline_hours: int | None = ...,
        dispute_resolution_days: int | None = ...,
        delivery_grace_days: int | None = ...,
    ) -> None: ...
    @staticmethod
    def regulatory_standard() -> MarginCallTiming: ...
    @property
    def notification_deadline_hours(self) -> int: ...
    @property
    def response_deadline_hours(self) -> int: ...
    @property
    def dispute_resolution_days(self) -> int: ...
    @property
    def delivery_grace_days(self) -> int: ...

class VmParameters:
    def __init__(
        self,
        threshold: Money,
        mta: Money,
        *,
        rounding: Money | None = ...,
        independent_amount: Money | None = ...,
        frequency: MarginTenor | str | None = ...,
        settlement_lag: int | None = ...,
    ) -> None: ...
    @staticmethod
    def regulatory_standard(currency: Currency | str) -> VmParameters: ...
    @staticmethod
    def with_threshold(threshold: Money, mta: Money) -> VmParameters: ...
    @property
    def threshold(self) -> Money: ...
    @property
    def mta(self) -> Money: ...
    @property
    def rounding(self) -> Money: ...
    @property
    def independent_amount(self) -> Money: ...
    @property
    def frequency(self) -> MarginTenor: ...
    @property
    def settlement_lag(self) -> int: ...

class ImParameters:
    def __init__(
        self,
        methodology: ImMethodology | str,
        mpor_days: int,
        threshold: Money,
        mta: Money,
        *,
        segregated: bool = ...,
    ) -> None: ...
    @staticmethod
    def simm_standard(currency: Currency | str) -> ImParameters: ...
    @staticmethod
    def schedule_based(currency: Currency | str) -> ImParameters: ...
    @staticmethod
    def cleared(currency: Currency | str) -> ImParameters: ...
    @staticmethod
    def repo_haircut(currency: Currency | str) -> ImParameters: ...
    @property
    def methodology(self) -> ImMethodology: ...
    @property
    def mpor_days(self) -> int: ...
    @property
    def threshold(self) -> Money: ...
    @property
    def mta(self) -> Money: ...
    @property
    def segregated(self) -> bool: ...

class EligibleCollateralSchedule:
    def __init__(self) -> None: ...
    @staticmethod
    def cash_only() -> EligibleCollateralSchedule: ...
    @staticmethod
    def bcbs_standard() -> EligibleCollateralSchedule: ...
    @staticmethod
    def us_treasuries() -> EligibleCollateralSchedule: ...
    @property
    def default_haircut(self) -> float | None: ...
    @property
    def rehypothecation_allowed(self) -> bool: ...
    @property
    def eligible(self) -> Any: ...

class CsaSpec:
    def __init__(
        self,
        id: str,
        base_currency: Currency | str,
        vm_params: VmParameters,
        *,
        im_params: ImParameters | None = ...,
        eligible_collateral: EligibleCollateralSchedule | None = ...,
        call_timing: MarginCallTiming | None = ...,
        collateral_curve_id: str,
    ) -> None: ...
    @staticmethod
    def usd_regulatory() -> CsaSpec: ...
    @staticmethod
    def eur_regulatory() -> CsaSpec: ...
    @property
    def id(self) -> str: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def vm_params(self) -> VmParameters: ...
    @property
    def im_params(self) -> ImParameters | None: ...
    @property
    def eligible_collateral(self) -> EligibleCollateralSchedule: ...
    @property
    def call_timing(self) -> MarginCallTiming: ...
    @property
    def collateral_curve_id(self) -> str: ...
    def requires_im(self) -> bool: ...
    def vm_threshold(self) -> Money: ...
    def im_threshold(self) -> Money | None: ...

__all__ = [
    "MarginTenor",
    "ImMethodology",
    "MarginCallTiming",
    "VmParameters",
    "ImParameters",
    "EligibleCollateralSchedule",
    "CsaSpec",
]
