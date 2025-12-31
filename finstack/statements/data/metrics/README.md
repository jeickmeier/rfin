# Financial Metrics Registry

This directory contains JSON metric definitions for the `finstack-statements` registry.

## Metric Conventions

### Leverage & Coverage Metrics

Leverage and coverage metrics in `fin_leverage.json` follow standard credit analysis conventions:

#### EBITDA Construction

- All coverage metrics use EBITDA = `revenue - cogs - opex + depreciation + amortization`
- This assumes D&A are captured in their own line items, not embedded in COGS/opex
- For accurate comparisons, ensure consistent D&A classification across periods

#### Interest Expense

- Interest expense should include all forms of interest:
  - Cash interest payments
  - PIK (payment-in-kind) interest accruals
  - Amortization of debt issuance costs (if applicable per your accounting policy)
- When using capital structure integration (`cs.interest_expense`), this automatically includes PIK

#### Principal Payments

- Principal payments are not tax-deductible
- Debt service coverage ratios may understate true coverage since EBITDA is pre-tax
- Consider using EBIAT (EBIT × (1 - tax_rate)) for more conservative analysis

#### Capitalized Interest

- If interest is capitalized during construction periods, it will not appear in interest_expense
- This can distort coverage ratios during development phases
- Adjust formulas manually if capitalized interest materially affects analysis

### Common Thresholds (Industry Guidelines)

- **Interest Coverage**: > 1.5x (investment grade), > 2.5x (strong)
- **Debt Service Coverage**: > 1.25x (typical covenant), > 1.5x (comfortable)
- **Debt/EBITDA**: < 3.0x (conservative), < 4.0x (acceptable for most industries)

### Customization

When defining custom metrics:

1. Use qualified references for registry metrics: `fin.ebitda` not `ebitda`
2. Document assumptions about line-item classification
3. Note any industry-specific adjustments
4. Specify whether ratios use TTM or period values

## File Structure

- `fin_basic.json` - Core income statement metrics
- `fin_leverage.json` - Leverage and coverage ratios
- Additional registries can be loaded via `ModelBuilder::with_metrics()`

## See Also

- Main registry documentation: `finstack/statements/src/registry/`
- DSL function reference: `finstack/statements/src/dsl/`
