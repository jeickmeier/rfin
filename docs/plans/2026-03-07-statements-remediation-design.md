# Statements Remediation Design

## Goal

Remediate the highest-impact correctness issues in `finstack/statements` with a correctness-first approach, prioritizing mathematically wrong results and practitioner-visible semantic bugs over backward compatibility.

## Scope

This remediation covers four tracks:

1. Evaluator semantics
2. Capital structure correctness
3. Analysis correctness
4. Audit and normalization semantics

The work intentionally changes current behavior when the existing behavior is economically or mathematically wrong.

## Design Principles

- Correctness over compatibility for buggy behavior
- Regression-test every fix before implementation
- Keep changes staged by domain so failures are attributable
- Prefer explicit validation failures over silent truncation or coercion
- Preserve existing architecture where possible and avoid speculative refactors

## Track 1: Evaluator Semantics

### Problems addressed

- Historical and rolling functions degrade non-column expressions to current-period values
- `quantile()` double-counts the current observation
- `coalesce()` treats zero as missing
- Window and lag arguments silently truncate non-integer inputs
- Monetary typing drifts to scalar in derived results

### Design

Introduce expression-aware historical evaluation for formula functions so they evaluate the supplied expression at each historical point rather than only for simple column references. Tighten argument validation for lag/window functions so invalid non-integer or non-finite inputs fail explicitly. Treat `coalesce()` as first non-NaN rather than first non-zero. Preserve and propagate monetary value types where the type can be determined safely, and fail or warn on invalid mixed-currency semantics instead of silently flattening everything to `f64`.

## Track 2: Capital Structure Correctness

### Problems addressed

- ECF sweep can be applied multiple times when no single target instrument is specified
- Cashflow aggregation destroys pay/receive sign semantics
- Waterfall behavior is insufficiently validated in multi-instrument cases

### Design

Make sweep allocation cash-conserving across the instrument set, with deterministic allocation order and no possibility of reusing the same sweep amount for multiple instruments. Preserve signed cashflow economics from valuations and classify flows using signed business semantics instead of absolute values. Expand integration and waterfall tests to prove conservation, sign correctness, and post-sweep balance behavior.

## Track 3: Analysis Correctness

### Problems addressed

- DCF uses all periods rather than true forecast cashflows
- DCF derives net debt from the terminal model period instead of valuation-date balance sheet semantics
- Market-aware DCF entry point ignores market-aware evaluation
- Scenario overrides overwrite all periods, including actuals

### Design

DCF should evaluate statements with the correct evaluation context, build cashflows from the intended forecast region, and derive net debt using valuation-date-consistent logic. Scenario overrides should become explicit scenario shocks or forecast-period-only overrides rather than blanket time-series replacement. The implementation should favor semantics that match how a practitioner would interpret scenario definitions and valuation inputs.

## Track 4: Audit and Normalization Semantics

### Problems addressed

- Corkscrew tolerance behaves like a relative tolerance despite being documented as an absolute rounding threshold
- Adjustment caps use `abs(base)` and over-permit add-backs under negative EBITDA

### Design

Align audit and normalization rules with their documented and lender-facing semantics. Corkscrew validation should use an absolute threshold if the configuration is documented as an absolute tolerance. Adjustment caps should not manufacture add-back capacity from negative EBITDA unless explicitly configured to do so.

## Testing Strategy

- Add one failing regression test per behavior change
- Verify each new test fails for the expected reason before implementation
- Keep each domain fix followed by focused test runs
- Run full `cargo test -p finstack-statements` after the tranche is complete

## Risks

- Some existing tests or downstream callers may rely on buggy behavior
- Tightened validation may surface currently silent model issues
- Monetary typing fixes may force a more explicit stance on mixed-currency formulas

## Non-Goals

- Broad API redesign of the statements crate
- New user-facing features unrelated to the quantified review findings
- Large architectural rewrites outside the reviewed correctness issues
