# Valuations Notebooks

This directory contains 18 educational Jupyter notebooks demonstrating the finstack valuations crate functionality through Python bindings.

## Status

**Current Status: 7/18 notebooks execute successfully**

These notebooks are educational demonstrations of valuations functionality. The working notebooks (01, 03-05, 07, 14, 16) show correct Python API usage. The remaining notebooks contain conceptual examples that demonstrate intended patterns but may require API adjustments.

### Fully Working Notebooks ✅

- 01_valuations_intro_pricer_registry.ipynb - Registry, Bond basics
- 03_valuations_cashflows_and_schedules.ipynb - Concepts
- 04_valuations_risk_and_attribution.ipynb - Risk metrics
- 05_valuations_bonds.ipynb - Fixed, floating, zero-coupon bonds
- 07_valuations_caps_floors_swaptions.ipynb - Vol surfaces
- 14_valuations_variance_and_quanto.ipynb - Variance instruments
- 16_valuations_convertibles_and_hybrids.ipynb - Hybrid products

### Needs API Updates ⚠️

- 02, 06, 08-13, 15, 17-18 - Require constructor/method name verification

For fully working examples, see `finstack-py/examples/scripts/valuations/instruments/`.

## Notebook Structure

### Foundation (01-04)

- **01_valuations_intro_pricer_registry.ipynb** - ✅ Working - Registry architecture and basic bond pricing
- **02_valuations_market_data_and_curves.ipynb** - Discount/forward curves, hazard curves, vol surfaces, FX
- **03_valuations_cashflows_and_schedules.ipynb** - Cashflow builders, fixed/floating coupons, amortization
- **04_valuations_risk_and_attribution.ipynb** - DV01/CS01/Greeks, metrics registry, attribution

### Rates (05-07)

- **05_valuations_bonds.ipynb** - Treasury, corporate, FRN, zero-coupon, callable, PIK bonds
- **06_valuations_swaps_and_basis.ipynb** - IRS, basis swaps, FRAs, deposits
- **07_valuations_caps_floors_swaptions.ipynb** - Caps, floors, swaptions with vol surfaces

### Credit (08-09)

- **08_valuations_credit_derivatives.ipynb** - Single-name CDS, CDS indices, CDS options
- **09_valuations_structured_credit.ipynb** - CDO/CLO tranches, base correlation

### Equity & FX (10-11)

- **10_valuations_equity_derivatives.ipynb** - Equity spot, baskets, European/American options, Greeks
- **11_valuations_fx_derivatives.ipynb** - FX spot, forwards, swaps, vanilla options, barriers

### Exotics (12-14)

- **12_valuations_path_dependent_options.ipynb** - Asian, lookback, cliquet, range accrual
- **13_valuations_barrier_and_autocallable.ipynb** - Barrier options, autocallable notes
- **14_valuations_variance_and_quanto.ipynb** - Variance swaps, quanto options

### Structured Products (15-16)

- **15_valuations_private_credit.ipynb** - Term loans, revolvers, repos, PE funds
- **16_valuations_convertibles_and_hybrids.ipynb** - Convertibles, TIPS, inflation swaps, TRS

### Advanced (17-18)

- **17_valuations_monte_carlo_deep_dive.ipynb** - MC processes, variance reduction, LSMC
- **18_valuations_calibration.ipynb** - Curve/surface calibration workflows

## Usage

1. **Verify API compatibility**: Check `finstack/valuations/instruments/__init__.pyi` for available instruments
2. **Update method calls**: Adjust constructor calls to match actual Python bindings
3. **Run incrementally**: Execute cells one at a time to catch API mismatches

## Available Instruments (Confirmed)

Based on `/finstack-py/finstack/valuations/instruments/__init__.pyi`:

- ✅ Bond
- ✅ Deposit
- ✅ InterestRateSwap (not `IRS`)
- ✅ ForwardRateAgreement (not `FRA`)
- ✅ InterestRateOption (caps/floors, not `CapFloor`)
- ✅ BasisSwap
- ✅ FxSpot, FxOption, FxSwap
- ✅ Equity, EquityOption
- ✅ CreditDefaultSwap (not `Cds`)
- ✅ CdsIndex, CdsOption, CdsTranche
- ✅ ConvertibleBond
- ✅ VarianceSwap
- ✅ InflationLinkedBond, InflationSwap
- ✅ Basket
- ✅ Repo
- ✅ EquityTotalReturnSwap, FiIndexTotalReturnSwap
- ✅ TermLoan, RevolvingCredit
- ✅ PrivateMarketsFund
- ✅ StructuredCredit

## Missing / Not Yet Exposed

Some instruments mentioned in notebooks may not yet have Python bindings:

- Swaptions (may be part of InterestRateOption)
- Barrier options (various types)
- Asian options (arithmetic/geometric)
- Lookback options
- Autocallable notes
- Cliquet options
- Quanto options
- Range accrual notes
- FX barrier options

## Next Steps

1. Map fictional APIs to actual Python binding methods
2. Test each notebook incrementally
3. Add real market data examples
4. Cross-reference with existing example scripts in `finstack-py/examples/scripts/valuations/`

## Contributing

When updating notebooks:

1. Verify API calls against type stubs in `finstack-py/finstack/valuations/`
2. Test execution before committing
3. Use deterministic examples (fixed dates, seeds)
4. Follow the existing educational narrative structure
