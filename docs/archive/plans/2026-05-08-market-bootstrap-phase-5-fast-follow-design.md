# Market Bootstrap Phase 5 — Python TypedDict (Fast-Follow)

> **Superseded** in v3 envelope shape: see [2026-05-10-calibration-envelope-cleanup-design.md](2026-05-10-calibration-envelope-cleanup-design.md). References to `initial_market` in this document predate the v3 cleanup.

**Status:** Draft
**Date:** 2026-05-08
**Owner:** finstack-py
**Phase:** 5 of 5 (fast-follow)
**Depends on:** Phase 3 (JSON Schema) is helpful but not required
**Related specs:**
- Phase 1 — Canonical-path foundation: [2026-05-08-market-bootstrap-phase-1-foundation-design.md](2026-05-08-market-bootstrap-phase-1-foundation-design.md)
- Phase 2 — Reference catalog: [2026-05-08-market-bootstrap-phase-2-reference-catalog-design.md](2026-05-08-market-bootstrap-phase-2-reference-catalog-design.md)
- Phase 3 — IDE autocomplete: [2026-05-08-market-bootstrap-phase-3-ide-autocomplete-design.md](2026-05-08-market-bootstrap-phase-3-ide-autocomplete-design.md)
- Phase 4 — Diagnostics: [2026-05-08-market-bootstrap-phase-4-diagnostics-design.md](2026-05-08-market-bootstrap-phase-4-diagnostics-design.md)

## 1. Motivation

Phase 3 ships JSON Schema and TypeScript types, giving editor autocomplete to anyone editing JSON files or writing TypeScript. Python users still construct envelopes as untyped dicts. This phase ships TypedDict definitions so Python users get autocomplete on dict construction without runtime overhead.

This is fast-follow because:

- Phase 3's JSON Schema already gives Python users autocomplete in JSON files.
- TypedDicts hand-written or codegen'd from JSON Schema are mostly mechanical work.
- A heavier Pydantic alternative is deferred entirely until user feedback warrants it.

## 2. Goals

- Python TypedDict definitions covering `CalibrationEnvelope`, all step variants, all quote classes, and `CalibrationConfig`.
- Stubs published in `finstack-py/finstack/valuations/envelope.pyi` (or merged into the existing `__init__.pyi`).
- A Python test that constructs each TypedDict variant and round-trips through `calibrate`.
- Drift between TypedDicts and the Rust schema is flagged best-effort. (TypedDicts are not auto-generated from JSON Schema unless we adopt `datamodel-code-generator`; see §3.)

## 3. Non-Goals

- Pydantic models. Defer indefinitely; users can layer on top if needed.
- Runtime validation of TypedDicts. They are documentation/typing only.
- Auto-generation pipeline. Decide between hand-written (lower friction, drift risk) and codegen via `datamodel-code-generator` (more setup, no drift) during implementation. Hand-written is acceptable for v1; revisit if drift bites.

## 4. Scope — file-by-file

[finstack-py/finstack/valuations/envelope.pyi](../finstack-py/finstack/valuations/envelope.pyi):

```python
from typing import TypedDict, Literal, NotRequired, Sequence

# --- Quote classes (snake_case tag = "class") ---

class _RateQuoteDeposit(TypedDict):
    class_: Literal["rates"]  # serialized as "class" via alias mechanism
    kind: Literal["deposit"]
    id: str
    index: str
    pillar: str  # "1M", "1Y", or "YYYY-MM-DD"
    rate: float

# ... one TypedDict per RateQuote variant, MarketQuote variant, StepParams variant, etc.

class _CdsQuote(TypedDict):
    class_: Literal["cds"]
    id: str
    issuer: str
    seniority: str
    pillar: str
    spread: float
    recovery: NotRequired[float]

# (One TypedDict per concrete variant under each MarketQuote class.)

# --- Steps ---

class _DiscountStep(TypedDict):
    id: str
    quote_set: str
    kind: Literal["discount"]
    # ... discount-specific params

# (One TypedDict per StepParams variant.)

# --- Top-level ---

class CalibrationPlan(TypedDict):
    id: str
    description: NotRequired[str]
    quote_sets: dict[str, Sequence[dict]]  # MarketQuote union; Python TypedDict has limits
    steps: Sequence[dict]  # CalibrationStep union
    settings: NotRequired[dict]  # CalibrationConfig

class CalibrationEnvelope(TypedDict):
    schema: Literal["finstack.calibration"]
    plan: CalibrationPlan
    initial_market: NotRequired[dict]  # MarketContextState
```

(The actual TypedDict tree handles tagged-union dispatch as cleanly as Python's type system allows. `Literal` field types help editors narrow on the discriminator. For the stubbornly-untyped union arms, `dict` is honest about the gap; users still get top-level autocomplete on field names.)

[finstack-py/finstack/valuations/**init**.pyi](../finstack-py/finstack/valuations/__init__.pyi):
- Re-export `CalibrationEnvelope` and the related TypedDict types from the `envelope` module.

[finstack-py/tests/test_envelope_typeddict.py](../finstack-py/tests/test_envelope_typeddict.py):
- Construct each top-level TypedDict variant; serialize via `json.dumps`; round-trip through `calibrate`.
- Verify the constructed envelope passes through unchanged (acceptance test for TypedDict ↔ Rust-schema agreement).

## 5. Acceptance Criteria

- [ ] TypedDicts exist for all top-level envelope structures and at least the four most common step kinds (`discount`, `forward`, `hazard`, `vol_surface`).
- [ ] At least one user-facing example notebook cell uses TypedDicts for construction.
- [ ] `mypy` (or `pyright`, depending on project preference) passes on a Python file that constructs and uses an envelope via TypedDict.
- [ ] Round-trip test passes: TypedDict → JSON → `calibrate` → no errors.

## 6. Risks

- **Python's TypedDict has limits.** Tagged unions (e.g., the `kind`-discriminated `StepParams`) are awkward in TypedDict; `Literal` field types help but coverage is editor-dependent. JetBrains Python and Pyright handle discriminated unions better than vanilla mypy.
- **Drift.** Hand-written TypedDicts can drift from the Rust schema. The codegen alternative (`datamodel-code-generator` from the JSON Schema produced in Phase 3) prevents drift but adds toolchain complexity. Recommend hand-written for v1; revisit if drift bites in practice.
