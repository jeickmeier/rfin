"""Typed dict definitions for ``CalibrationEnvelope``.

These are documentation/typing aids for analysts who construct calibration
envelopes as Python dicts. They mirror the Rust `CalibrationEnvelope` schema
in [`finstack-valuations`] and produce JSON that ``calibrate`` and ``dry_run``
accept verbatim.

Coverage (Phase 5 v1):
- All top-level structures: ``CalibrationEnvelope``, ``CalibrationPlan``,
  ``CalibrationStep``.
- The four most common step kinds: ``discount``, ``forward``, ``hazard``,
  ``vol_surface``.
- Shared building blocks: ``Tenor``, ``Pillar``, ``RateDeposit``,
  ``RateSwap``, ``CdsParSpread``.

Other step kinds (inflation, swaption_vol, base_correlation, student_t,
hull_white, cap_floor_hull_white, svi_surface, xccy_basis, parametric) and
quote variants (FRA, futures, FX, bond, vol, inflation, CDS upfront, CDS
tranche, xccy) are not yet typed — fall back to ``dict[str, Any]`` for those
or work from the JSON Schema (see Phase 3).

These TypedDicts are documentation only — no runtime validation. Use
``dry_run`` (Phase 4) for structural checks.
"""

from __future__ import annotations

from typing import Any, Literal, NotRequired, TypedDict

# =============================================================================
# Shared building blocks
# =============================================================================


class Tenor(TypedDict):
    """A relative tenor like ``5Y`` or ``3M``.

    Maps to ``finstack_core::dates::Tenor``. Serializes as
    ``{"count": 5, "unit": "years"}``.
    """

    count: int
    unit: Literal["days", "weeks", "months", "years"]


class TenorPillar(TypedDict):
    """The ``Tenor`` arm of the ``Pillar`` tagged union.

    Serializes as ``{"tenor": {"count": 5, "unit": "years"}}``.
    """

    tenor: Tenor


class DatePillar(TypedDict):
    """The absolute-date arm of the ``Pillar`` tagged union.

    Serializes as ``{"date": "2027-05-08"}``.
    """

    date: str


# `Pillar` is a snake_case-tagged enum on the Rust side; serde emits one
# variant per dict (`tenor` arm carries a Tenor; `date` arm carries an ISO
# date string).
Pillar = TenorPillar | DatePillar


class CdsConventionKey(TypedDict):
    """Currency + doc-clause pairing identifying CDS market conventions."""

    currency: str
    doc_clause: str


# =============================================================================
# MarketQuote variants (subset)
# =============================================================================
#
# `MarketQuote` is `#[serde(tag = "class", ...)]`. The inner enums are also
# tagged via `type`. We use the functional TypedDict form because `class` and
# `type` are reserved/built-in names in Python.


RateDeposit = TypedDict(
    "RateDeposit",
    {
        "class": Literal["rates"],
        "type": Literal["deposit"],
        "id": str,
        "index": str,
        "pillar": Pillar,
        "rate": float,
    },
)
"""A money-market deposit rate quote."""

RateSwap = TypedDict(
    "RateSwap",
    {
        "class": Literal["rates"],
        "type": Literal["swap"],
        "id": str,
        "index": str,
        "pillar": Pillar,
        "rate": float,
        "spread_decimal": NotRequired[float | None],
    },
)
"""A vanilla IRS par-rate quote."""

CdsParSpread = TypedDict(
    "CdsParSpread",
    {
        "class": Literal["cds"],
        "type": Literal["cds_par_spread"],
        "id": str,
        "entity": str,
        "convention": CdsConventionKey,
        "pillar": Pillar,
        "spread_bp": float,
        "recovery_rate": float,
    },
)
"""A CDS par-spread quote."""


# Union of the typed quote variants. Other variants serialize as untyped
# dicts — `dict[str, Any]` is the honest fallback.
MarketQuote = RateDeposit | RateSwap | CdsParSpread | dict[str, Any]

# =============================================================================
# Step variants (4 most common)
# =============================================================================


class DiscountStep(TypedDict):
    """A ``discount`` calibration step.

    Builds a discount factor curve from money-market quotes (deposits + IRS).
    """

    id: str
    quote_set: str
    kind: Literal["discount"]
    curve_id: str
    currency: str
    base_date: str
    method: NotRequired[str]
    interpolation: NotRequired[str]
    extrapolation: NotRequired[str]
    pricing_discount_id: NotRequired[str | None]
    pricing_forward_id: NotRequired[str | None]
    conventions: NotRequired[dict[str, Any]]


class ForwardStep(TypedDict):
    """A ``forward`` calibration step.

    Builds a forward (projection) curve at a given tenor against a
    pre-existing discount curve.
    """

    id: str
    quote_set: str
    kind: Literal["forward"]
    curve_id: str
    currency: str
    base_date: str
    tenor_years: float
    discount_curve_id: str
    method: NotRequired[str]
    interpolation: NotRequired[str]
    conventions: NotRequired[dict[str, Any]]


class HazardStep(TypedDict):
    """A ``hazard`` calibration step.

    Builds a hazard (default-intensity) curve from CDS par-spread or upfront
    quotes against a discount curve.
    """

    id: str
    quote_set: str
    kind: Literal["hazard"]
    curve_id: str
    entity: str
    seniority: str
    currency: str
    base_date: str
    discount_curve_id: str
    recovery_rate: NotRequired[float]
    notional: NotRequired[float]
    method: NotRequired[str]
    interpolation: NotRequired[str]
    par_interp: NotRequired[str]
    doc_clause: NotRequired[str]


class VolSurfaceStep(TypedDict):
    """A ``vol_surface`` calibration step (SABR-only today).

    Builds an equity / index volatility surface.
    """

    id: str
    quote_set: str
    kind: Literal["vol_surface"]
    surface_id: str
    base_date: str
    underlying_ticker: str
    model: str
    discount_curve_id: NotRequired[str | None]
    beta: NotRequired[float]
    target_expiries: NotRequired[list[float]]
    target_strikes: NotRequired[list[float]]
    spot_override: NotRequired[float | None]
    dividend_yield_override: NotRequired[float | None]
    expiry_extrapolation: NotRequired[str]


CalibrationStep = DiscountStep | ForwardStep | HazardStep | VolSurfaceStep | dict[str, Any]

# =============================================================================
# Top-level
# =============================================================================


class CalibrationPlan(TypedDict):
    """The plan inside a `CalibrationEnvelope`."""

    id: str
    description: NotRequired[str | None]
    quote_sets: dict[str, list[MarketQuote]]
    steps: list[CalibrationStep]
    settings: NotRequired[dict[str, Any]]


CalibrationEnvelope = TypedDict(
    "CalibrationEnvelope",
    {
        "$schema": NotRequired[str],
        "schema": Literal["finstack.calibration"],
        "plan": CalibrationPlan,
        "initial_market": NotRequired[dict[str, Any] | None],
    },
)
"""Top-level envelope accepted by ``calibrate`` / ``dry_run``.

Construct with:

    envelope: CalibrationEnvelope = {
        "schema": "finstack.calibration",
        "plan": {
            "id": "usd_curves",
            "quote_sets": {...},
            "steps": [...],
            "settings": {},
        },
        "initial_market": None,
    }
"""


__all__ = [
    "CalibrationEnvelope",
    "CalibrationPlan",
    "CalibrationStep",
    "CdsConventionKey",
    "CdsParSpread",
    "DatePillar",
    "DiscountStep",
    "ForwardStep",
    "HazardStep",
    "MarketQuote",
    "Pillar",
    "RateDeposit",
    "RateSwap",
    "Tenor",
    "TenorPillar",
    "VolSurfaceStep",
]
