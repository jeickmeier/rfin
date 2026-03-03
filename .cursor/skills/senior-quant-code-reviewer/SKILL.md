---
name: senior-quant-code-reviewer
description: Reviews quantitative finance code for mathematical correctness, numerical precision, and market standard compliance. Use when reviewing pricing models, risk calculations, curve construction, margin calculations, cashflow generation, or any financial mathematics implementation in Rust or Python.
---

# Senior Quant Code Reviewer

## Quick start

When reviewing quantitative finance code, produce a review with:

1. **Summary**: what changed, mathematical/numerical risk level.
2. **Quantitative concerns**: 3–7 bullets on math correctness, precision, and market standard issues.
3. **Findings**: grouped by severity with concrete fixes and references to industry standards.
4. **Action items**: checklist with specific mathematical/numerical improvements.

After each review cycle, re-check the code and update the review. Continue iterating until there are no remaining action items. If any action items remain, treat this as an incomplete review and perform another review cycle.

If mathematical assumptions are unstated, treat as a finding. Quant code must be explicit about conventions, precision requirements, and edge cases.

## Severity rubric

- **Blocker**: Incorrect math, wrong formula, precision loss causing material P&L/risk errors, violation of market standards that would cause trade breaks.
- **Major**: Numerical instability in edge cases, missing edge case handling, undocumented conventions, inconsistent precision.
- **Minor**: Suboptimal algorithm choice, missing validation, unclear variable naming for financial quantities.
- **Nit**: Style preference, documentation improvements.

## Review checklist

### Mathematical correctness

- Verify formulas against authoritative sources (Hull, Brigo-Mercurio, ISDA docs).
- Check sign conventions: pay vs receive, long vs short, accrued interest direction.
- Validate Greeks calculations: bump sizes, finite difference schemes, analytical vs numerical.
- Ensure compounding and discounting logic is consistent throughout.

### Numerical precision & stability

- Identify precision-sensitive operations: small differences of large numbers, near-zero denominators.
- Check for appropriate use of `f64` vs `Decimal` based on precision requirements.
- Look for catastrophic cancellation in subtraction of similar values.
- Verify root-finding/optimization convergence criteria and iteration limits.
- Check interpolation schemes at boundaries and extrapolation behavior.

### Day count & date conventions

- Verify day count convention implementation matches ISDA/market standards.
- Check business day adjustment rules (Modified Following, etc.).
- Validate stub period handling (front/back, short/long).
- Ensure roll date logic handles month-end conventions correctly.
- Verify holiday calendar integration and market-specific calendars.

### Curve construction & interpolation

- Check interpolation method appropriateness (log-linear discount, monotonic splines, etc.).
- Verify bootstrap convergence and instrument ordering.
- Look for negative forward rates or discount factors > 1.
- Validate curve bump methodology for risk calculations.
- Check tenor/pillar point selection and gap handling.

### Pricing models

- Verify model assumptions are documented and appropriate for instrument type.
- Check boundary conditions (at expiry, at barriers, at zero vol).
- Validate payoff logic for all exercise scenarios.
- Ensure Greeks are consistent with pricing (no arbitrage).
- Check vol surface interpolation and extrapolation.

### Risk & sensitivities

- Verify bump sizes are appropriate (1bp for rates, 1% relative for vol).
- Check for consistent shift methodology (parallel, twist, butterfly).
- Validate aggregation logic (netting, cross-gamma handling).
- Ensure proper treatment of convexity and cross-effects.
- Check scenario generation for stress testing validity.

### Cashflow generation

- Verify notional exchange logic (initial, final, intermediate).
- Check accrual period calculation and fixing date logic.
- Validate reset/payment lag handling.
- Ensure compounding cashflows are correctly accumulated.
- Check amortization schedule application.

### Margin calculations

- Verify ISDA SIMM methodology compliance (risk weights, correlations).
- Check Schedule IM calculation per regulatory requirements.
- Validate netting set aggregation rules.
- Ensure proper delta/vega/curvature bucketing.
- Check concentration thresholds and margin period of risk.

## Quantitative red flags

Watch for these common errors:

| Issue | Symptom | Fix |
|-------|---------|-----|
| Wrong compounding | Rates don't match market quotes | Use market convention (ACT/360, semi-annual, etc.) |
| Precision loss | Greeks unstable or noisy | Use appropriate bump sizes, consider AD |
| Sign error | P&L has wrong direction | Trace pay/receive through entire calculation |
| Off-by-one | Cashflows missing or duplicated | Validate against trade economics |
| Calendar mismatch | Dates don't match confirms | Use correct market calendars |
| Interpolation artifacts | Spiky forwards or Greeks | Check spline boundary conditions |

## Default review output template

```markdown
## Summary
<1–3 bullets: what changed, quantitative risk level>

## Quantitative concerns
- <concern with reference to market standard or formula>
- <precision/stability issue>

## Findings
### Blockers
- <mathematical error> (correct formula, source reference)

### Majors
- <numerical issue> (suggested fix with rationale)

### Minors / Nits
- <improvement> (optional)

## Action items
- [ ] <specific quantitative fix>
- [ ] <validation/test to add>
```

## Additional resources

- For detailed domain knowledge and standards, see [reference.md](reference.md).
- For code examples of common issues, see [examples.md](examples.md).
