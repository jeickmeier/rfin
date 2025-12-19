# Calibration + Market Review Fix Plan

Decisions confirmed by user:
- CDS schedules should follow IMM/standard roll conventions.
- Base correlation uses index notional.
- Futures and YoY inflation should be supported.
- Swaption ATM convention and SABR interpolation should be implemented.

## Commit 1: Hazard calibration cleanup + CDS upfront handling
Why
- Remove debug noise and fix double-counted upfront in hazard residuals.

Changes
- Remove `/tmp/handlers_debug.log` + `/tmp/hazard_debug.log` writes and stdout prints from `HazardBootstrapper`.
- Stop subtracting upfront in hazard residuals since CDS instrument already includes upfront.
- If `CalibrationQuote::Cds` keeps upfront info, leave it unused in hazard or set it to `None` in preparation to avoid confusion.

Files
- finstack/valuations/src/calibration/targets/hazard.rs
- finstack/valuations/src/calibration/prepared.rs (only if enum handling is adjusted)

Tests
- Add/adjust unit test to ensure CDS upfront quotes do not double-count in residuals.

---

## Commit 2: CDS tranche notional + upfront support
Why
- Notional is inverted today and tranche upfront is ignored outside calibration.

Changes
- Fix tranche notional to use `index_notional * (detachment - attachment)` and validate width > 0.
- Use `BaseCorrelationParams.series` when building tranche params (currently hard-coded 0).
- Add upfront handling to tranche instrument/pricer (e.g., `PricingOverrides` or explicit field) so PV includes upfront.

Files
- finstack/valuations/src/market/build/cds_tranche.rs
- finstack/valuations/src/instruments/cds_tranche/* (types/pricer if needed)
- finstack/valuations/src/calibration/targets/base_correlation.rs

Tests
- Unit test for tranche notional scaling.
- Unit test for upfront inclusion in tranche PV.

---

## Commit 3: Base correlation params usage + validation
Why
- Several BaseCorrelationParams are currently unused; validate quotes and use schedule overrides.

Changes
- Validate quote maturity vs `params.maturity_years` (tolerance-based) and index series vs `params.series` (if encoded in index id or provided via conventions).
- Use `params.detachment_points` to enforce expected tranche set (match/dedup/ordering) and provide a clear error if missing.
- Apply schedule overrides (`payment_frequency`, `day_count`, `business_day_convention`, `calendar_id`, `use_imm_dates`) when building tranche instruments.
- Make sorting NaN-safe: reject NaN detachment before `partial_cmp`.

Files
- finstack/valuations/src/calibration/targets/base_correlation.rs
- finstack/valuations/src/market/build/cds_tranche.rs (accept schedule overrides)
- finstack/valuations/src/calibration/api/schema.rs (if additional validation helpers needed)

Tests
- Validation tests for missing detachment points and maturity mismatch.

---

## Commit 4: CDS IMM/standard roll scheduling
Why
- CDS schedules should use IMM roll / standard ISDA conventions, not spot-start + None stub.

Changes
- Add schedule generation for CDS premium leg using IMM roll (e.g., quarterly IMM) and correct accrual start.
- Extend conventions to include IMM usage flags or roll rules if not already in `CdsConventions`.
- Update CDS builder to respect IMM/standard scheduling and stub selection.

Files
- finstack/valuations/src/market/build/cds.rs
- finstack/valuations/src/market/conventions/defs.rs
- finstack/valuations/src/instruments/cds/* (schedule or builder helpers)

Tests
- CDS schedule unit test (start/end and coupon dates match IMM roll).

---

## Commit 5: Enforce market index conventions for rates
Why
- Instrument construction should use market conventions from the index registry, not step-level calibration overrides.

Changes
- Remove any reliance on `RatesStepConventions` for instrument schedule/lag/BDC in rate instrument construction.
- Ensure discount/forward calibration builds instruments strictly from index conventions in the registry.
- Use `strict_pricing` only to validate missing/invalid registry conventions, not to override them.

Files
- finstack/valuations/src/calibration/targets/discount.rs
- finstack/valuations/src/calibration/targets/forward.rs
- finstack/valuations/src/market/build/rates.rs
- finstack/valuations/src/calibration/config.rs (helper wiring)

Tests
- Unit tests verifying settlement lag / payment delay overrides impact pillar dates.

---

## Commit 6: Interest rate futures support
Why
- Futures quotes are exposed in the API but the builder is unimplemented.

Changes
- Add futures contract conventions registry if missing (e.g., per contract ID).
- Implement `RateQuote::Futures` instrument building using `InterestRateFuture`.
- Use futures expiry as pillar time in discount/forward calibration.

Files
- finstack/valuations/src/market/build/rates.rs
- finstack/valuations/src/market/conventions/loaders/* (new registry)
- finstack/valuations/src/calibration/targets/discount.rs
- finstack/valuations/src/calibration/targets/forward.rs

Tests
- Builder unit test for futures.
- Calibration target test ensuring futures quotes produce positive pillar times.

---

## Commit 7: YoY inflation swap support
Why
- YoY quotes exist but calibration rejects them.

Changes
- Implement YoY inflation swap instrument construction in `InflationBootstrapper::prepare_single`.
- Use quote `frequency` and conventions to build YoY swaps.

Files
- finstack/valuations/src/calibration/targets/inflation.rs
- finstack/valuations/src/instruments/inflation_swap/* (if YoY support needs wiring)

Tests
- Unit test for YoY inflation quote preparation.

---

## Commit 8: Swaption ATM convention + SABR interpolation method
Why
- `atm_convention` and `sabr_interpolation` are currently ignored.

Changes
- Implement `AtmStrikeConvention`:
  - `SwapRate`: use forward par rate at expiry (current behavior).
  - `ParRate`: compute par rate from base-date schedule (spot-starting) or explicitly document if equal.
- Respect `SabrInterpolationMethod` via a `match` (currently only bilinear; error on unsupported variants if added later).

Files
- finstack/valuations/src/calibration/targets/swaption.rs
- finstack/valuations/src/calibration/api/schema.rs (if new interpolation variants are added later)

Tests
- Unit test for ATM convention branch.
- Unit test confirming `sabr_interpolation` path is selected.

---

## Commit 9: Registry loader error handling
Why
- Registry loaders panic on malformed data, causing hard process crashes.

Changes
- Change loader functions to return `Result<HashMap<...>>` and propagate errors to `ConventionRegistry`.
- Replace `panic!` in `build_lookup_map_mapped` and loader conversions with structured errors.
- Update `ConventionRegistry::global()` initialization to surface errors cleanly.

Files
- finstack/valuations/src/market/conventions/loaders/json.rs
- finstack/valuations/src/market/conventions/loaders/*.rs
- finstack/valuations/src/market/conventions/registry.rs

Tests
- Unit test for duplicate registry ids and invalid records returning `Err`.

---

## Suggested Validation Run
- `cargo test -p finstack_valuations calibration::targets::hazard`
- `cargo test -p finstack_valuations market::build::cds_tranche`
- `cargo test -p finstack_valuations calibration::targets::swaption`
- Add targeted unit tests per commit as above.
