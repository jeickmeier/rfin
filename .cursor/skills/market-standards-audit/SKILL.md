---
name: market-standards-audit
description: Systematically audit quantitative code against professional library standards (QuantLib, Bloomberg, Numerix, FinCad). Use when verifying pricers, algorithms, and defaults match market conventions, when adding new instruments, or when the user asks about market standard compliance, professional library parity, or convention correctness.
---

# Market Standards Audit

## Quick start

When auditing quantitative code against market standards:

1. **Identify the instrument type** and locate the relevant standard checklist
2. **Verify conventions**: day counts, business days, compounding, settlement
3. **Check pricing algorithms**: compare against QuantLib/Bloomberg methodology
4. **Validate defaults**: ensure parameters match professional library defaults
5. **Document deviations**: any intentional differences from standards must be justified

## Audit output template

```markdown
## Audit: [Instrument Type]

### Standard compliance
| Area | Expected (QuantLib/BBG) | Implementation | Status |
|------|------------------------|----------------|--------|
| Day count | ACT/360 | ACT/360 | ✅ |
| Business day | Modified Following | Following | ⚠️ |

### Findings
- [Finding with standard reference]

### Recommendations
- [ ] [Specific fix to match standard]
```

## Core conventions by market

### Interest rate markets

| Market | Day count | Business day | Fixing lag | Payment lag |
|--------|-----------|--------------|------------|-------------|
| USD SOFR | ACT/360 | Modified Following | 2 days | 2 days |
| USD Fed Funds | ACT/360 | Following | 0 days | 1 day |
| EUR ESTR | ACT/360 | Modified Following | 2 days | 2 days |
| GBP SONIA | ACT/365F | Modified Following | 0 days | 0 days |
| JPY TONAR | ACT/365F | Modified Following | 2 days | 2 days |
| CHF SARON | ACT/360 | Modified Following | 2 days | 2 days |

### FX markets

| Pair type | Spot days | Quote convention | Day count |
|-----------|-----------|------------------|-----------|
| G10/G10 | T+2 | Standard (EUR/USD) | ACT/360 |
| USD/CAD | T+1 | USD terms | ACT/360 |
| USD/MXN | T+2 | USD terms | ACT/360 |
| USD/TRY | T+1 | USD terms | ACT/360 |

### Bond markets

| Market | Day count | Settlement | Price quote |
|--------|-----------|------------|-------------|
| US Treasury | ACT/ACT ICMA | T+1 | Clean price |
| US Corporate | 30/360 | T+2 | Clean price |
| UK Gilt | ACT/ACT ICMA | T+1 | Clean price |
| German Bund | ACT/ACT ICMA | T+2 | Clean price |

## Instrument-specific standards

For detailed standards by instrument, see:

- [rates-standards.md](rates-standards.md) - Swaps, FRAs, caps/floors, swaptions
- [fx-standards.md](fx-standards.md) - FX forwards, swaps, options
- [fixed-income-standards.md](fixed-income-standards.md) - Bonds, repos, term loans
- [equity-standards.md](equity-standards.md) - Options, variance swaps, TRS
- [algorithm-standards.md](algorithm-standards.md) - Pricing algorithms, root-finding, interpolation

## Common audit failures

| Issue | Professional standard | Common mistake |
|-------|----------------------|----------------|
| Compounding mismatch | OIS: daily compounding with shift | Simple compounding or no shift |
| Settlement lag | Instrument-specific (see tables) | Hardcoded T+2 for all |
| Stub handling | Short front stub default | No stub or wrong direction |
| Notional exchange | XCCY: initial + final | Missing exchanges |
| Fixing source | Official fixing (e.g., SOFR from FRBNY) | Generic "overnight rate" |
| Business day calendar | Instrument-specific (joint calendars) | Single calendar |
| Roll convention | EOM for month-end trades | No EOM handling |

## Validation requirements

Every pricer must pass:

1. **Round-trip test**: Construct from market quote → price = quote (< 0.01bp error)
2. **QuantLib parity**: Match QuantLib within tolerance (< 0.1bp for rates)
3. **Greeks consistency**: Numerical Greeks stable and correct sign
4. **Boundary behavior**: Correct at expiry, zero vol, extreme rates
