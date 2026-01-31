# Changelog

All notable changes to the Finstack workspace will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Removed

The following deprecated APIs have been removed. See the migration guide below.

#### Cashflow Module
- `npv_constant()` - Use `npv()` with a `FlatCurve` or `npv_amounts()` for scalar flows

#### Math Module (Integration)
- `GaussHermiteQuadrature::order_5()` - Use `GaussHermiteQuadrature::new(5)?`
- `GaussHermiteQuadrature::order_7()` - Use `GaussHermiteQuadrature::new(7)?`
- `GaussHermiteQuadrature::order_10()` - Use `GaussHermiteQuadrature::new(10)?`
- `GaussHermiteQuadrature::order_15()` - Use `GaussHermiteQuadrature::new(15)?`
- `GaussHermiteQuadrature::order_20()` - Use `GaussHermiteQuadrature::new(20)?`

#### Discount Curve
- `DiscountCurve::zero_on_date()` - Use `zero_rate_on_date(date, Compounding::Continuous)`
- `DiscountCurve::zero_annual_on_date()` - Use `zero_rate_on_date(date, Compounding::Annual)`
- `DiscountCurve::zero_periodic_on_date()` - Use `zero_rate_on_date(date, Compounding::Periodic(n))`
- `DiscountCurve::zero_simple_on_date()` - Use `zero_rate_on_date(date, Compounding::Simple)`

### Migration Guide

#### NPV Calculations

```rust
// Before (deprecated)
let pv = npv_constant(&flows, 0.05, base, DayCount::Act365F)?;

// After (canonical)
// Option 1: Use npv with FlatCurve for Money flows
let curve = FlatCurve::new((1.0 + 0.05).ln(), base, DayCount::Act365F, "id");
let pv = npv(&curve, base, Some(DayCount::Act365F), &flows)?;

// Option 2: Use npv_amounts for scalar flows (simpler)
let pv = npv_amounts(&scalar_flows, 0.05, Some(base), Some(DayCount::Act365F))?;
```

#### Quadrature

```rust
// Before (deprecated)
let quad = GaussHermiteQuadrature::order_10();

// After (canonical)
let quad = GaussHermiteQuadrature::new(10)?;
```

#### Zero Rates

```rust
// Before (deprecated)
let r_cont = curve.zero_on_date(date)?;
let r_ann = curve.zero_annual_on_date(date)?;
let r_semi = curve.zero_periodic_on_date(date, 2)?;
let r_simple = curve.zero_simple_on_date(date)?;

// After (canonical)
use finstack_core::math::Compounding;

let r_cont = curve.zero_rate_on_date(date, Compounding::Continuous)?;
let r_ann = curve.zero_rate_on_date(date, Compounding::Annual)?;
let r_semi = curve.zero_rate_on_date(date, Compounding::SEMI_ANNUAL)?;
let r_simple = curve.zero_rate_on_date(date, Compounding::Simple)?;
```

### Breaking

- Removed legacy attribution curve helpers and deprecated CDS option constructors.
- Removed structured credit constructor `waterfall` parameter and `MetricId::AccruedInterest` alias.
- Removed legacy JSON aliases for swap spreads (`spread`) and swaption maturity (`tenor`).
- Removed `CashFlowBuilder::build()` (Rust/Python/JS/WASM); use `build_with_curves(None)` or the optional-market `buildWithCurves()`.
- Removed `MetricRegistry::compute_best_effort()`; use `compute()` (strict) or `Instrument::price_with_metrics()`.
- Removed `Instrument::matches_selector/has_tag/get_meta`; use `instrument.attributes().matches_selector/has_tag/get_meta`.
- Removed binomial tree barrier wrappers (`price_up_and_out`, `price_down_and_out`, `price_up_and_in`, `price_down_and_in`, `price_*_american`); use `price_barrier_out/in` variants.

### Python Bindings Enhancements (v1.0.0-beta.1)

**New Instrument Bindings** (8 instruments):

- BondFuture, BondFutureOption - Treasury futures with CTD analysis
- CrossCurrencySwap, InflationCapFloor - Dual-currency and inflation derivatives
- EquityIndexFuture, NonDeliverableForward, FxVarianceSwap - Equity and FX instruments
- CommodityOption, RealEstateAsset - Alternative asset classes

**Pricer Registry** (11 model keys added):

- Complete ModelKey coverage: HestonFourier, Normal, MonteCarloGBM, MonteCarloHeston, MonteCarloHullWhite1F
- Analytical methods: BarrierBSContinuous, AsianGeometricBS, AsianTurnbullWakeman, LookbackBSContinuous, QuantoBS, FxBarrierBSContinuous
- All 16 Rust model keys now accessible from Python

**Calibration Framework**:

- Plan-driven API (v2) for declarative calibration workflows
- All curve types: discount, forward, hazard, inflation, vol surfaces, base correlation
- Comprehensive quote types: RatesQuote, CreditQuote, VolQuote, InflationQuote
- Configuration: CalibrationConfig, SolverKind, ValidationMode

**Scenarios DSL and Builder** (800+ lines Python):

- Text-based DSL parser for scenario construction
- Fluent builder API for programmatic scenario building
- Full integration with existing ScenarioSpec and ScenarioEngine
- Comprehensive examples and documentation

**Statement Extensions**:

- Complete configuration API for Corkscrew (balance sheet validation)
- Credit Scorecard configuration (rating assignment)
- JSON serialization for all extension types

**Portfolio Management**:

- Book hierarchy support (Rust implementation complete)
- Margin and netting calculations (already exposed)
- Portfolio optimization framework (1200+ lines, 17 classes)

**Testing and Quality**:

- 411+ passing tests across all modules
- Property-based testing with Hypothesis (70+ property tests)
- Comprehensive parity tests (215+ test cases)
- Benchmark infrastructure with pytest-benchmark

**Documentation**:

- Sphinx API documentation site structure
- Tutorial series (installation, quickstart, core concepts)
- 40+ working examples covering all instrument types
- 20+ cookbook patterns for common workflows
- NumPy-style docstrings throughout

**Known Limitations**:

- Bucketed metrics (DV01/CS01/Vega by tenor) require Rust ValuationResult changes
- Python-side modules (scenarios.dsl, scenarios.builder) need package integration
- Some tests pending package rebuild for new registrations

**See**: `.zenflow/tasks/100-python-binding-7042/task-4.6-summary.md` for detailed release assessment

### Planned for 0.9.0

- Reduction of `expect()` and `panic!()` usage across all crates
- Additional market convention validations
- Performance optimizations for large portfolios
- Python bindings: Bucketed metrics exposure (pending Rust core changes)

### Planned for 1.0.0

- **BREAKING**: Removal of all deprecated APIs across crates
- Full compliance with safety lints (no panics in production code)
- Stabilized public APIs with backwards compatibility guarantees
- Python bindings: 100% API parity with Rust public surface

---

## [0.8.0] - 2024-12-20

Major release addressing critical safety issues and market convention compliance across the Finstack workspace. This release includes breaking changes to prevent silent failures in risk calculations and fix incorrect FX settlement dates.

### 🎯 Release Highlights

- **Critical Safety Fixes**: Metrics computation no longer silently fails; errors are surfaced explicitly
- **Market Convention Compliance**: FX settlement now uses correct joint business day logic per ISDA standards
- **API Safety**: Panicking constructors deprecated; safety lints enforced to prevent regressions
- **Comprehensive Testing**: 50+ new tests, golden reference files, and integration test suites
- **Migration Support**: Detailed migration guide with decision trees, examples, and FAQs

### ⚠️ Breaking Changes Summary

**Affects**: Applications using metrics, FX instruments, or multi-currency portfolios

| Change | Severity | Migration Effort |
|--------|----------|------------------|
| Metrics strict mode default | 🔴 Critical | 1-2 hours (add error handling) |
| FX spot date calculation | 🔴 Critical | 2-4 hours (verify dates, update tests) |
| Calendar resolution errors | 🟠 Major | 30 min - 1 hour (handle errors) |
| Swap spread field rename | 🟠 Major | 15-30 min (backwards compatible) |

**See**: `MIGRATION_GUIDE.md` for comprehensive migration instructions

### Changed by Crate

#### finstack-core (0.8.0)

**Added**:

- New error variants for better diagnostics:
  - `Error::UnknownMetric` - Unknown metric ID with list of available metrics
  - `Error::MetricNotApplicable` - Metric doesn't apply to instrument type
  - `Error::MetricCalculationFailed` - Computation failed with detailed cause
  - `Error::CircularDependency` - Cycle detected with full dependency path

#### finstack-valuations (0.8.0)

**Breaking Changes**:

- **Metrics**: `compute()` defaults to strict mode (errors instead of 0.0 for failures)
- **FX Settlement**: Uses joint business day counting (spot dates may differ near holidays)
- **Calendars**: `resolve_calendar()` returns errors for unknown IDs (no silent fallback)
- **Quote Units**: `RateQuote::Swap { spread }` → `{ spread_decimal }` for clarity

**Added**:

- `MetricRegistry::compute_best_effort()` - Opt-in legacy behavior (removed in 0.4.1)
- `MetricId::parse_strict()` - Strict metric name validation for user inputs
- `add_joint_business_days()` - Proper FX settlement date calculation
- `CalendarWrapper` - Better error messages for calendar resolution

**Fixed**:

- Calibration residual normalization (now scales by `residual_notional`)
- Metric dependency cycles now detected and reported with full path
- Results export uses correct `MetricId` constants (duration, DV01, etc.)

**Deprecated**:

- `CdsOption::new()` and related panicking constructors (removed in 0.8.x)
- Use `try_new()` variants instead for proper error handling

#### finstack-py (0.8.0)

**Added**:

- Python bindings for strict metric parsing: `MetricId.parse_strict()`
- Swap quote schema updated to use `spread_decimal` field

**Changed**:

- Error conversions updated for new error variants

#### finstack-wasm (0.8.0)

**Added**:

- WASM bindings for strict metric parsing: `MetricId.parseStrict()`
- TypeScript types updated for swap quote schema changes

**Changed**:

- Error mappings updated for new error variants

### Documentation

**New**:

- `MIGRATION_GUIDE.md` - Comprehensive migration instructions with decision trees
- API documentation for all new methods with examples

**Updated**:

- All changed APIs include `# Examples` and `# Errors` sections
- Cross-links between related APIs
- Migration paths documented in the migration guide and API docs

### Testing

**Integration Tests Added** (19 total):

- `metrics_strict_mode.rs` - 7 tests covering strict mode
- `fx_settlement.rs` - 12 tests covering joint business day logic

**Unit Tests Added** (50+):

- Metrics core: 19 tests (error paths, strict mode, dependency resolution)
- FX dates: 11 tests (joint calendar, error handling)
- Results export: 11 tests (metric key mappings)
- Calibration: 3 tests (residual normalization invariance)

**Golden Reference Files**:

- FX spot dates validated against ISDA, ECB, NYSE, Bank of England, JPX calendars

### Performance

**Benchmarks** (no significant regressions):

- Metrics strict mode: <1% overhead (within measurement noise)
- Calibration: <0.1% difference after residual fix
- FX settlement: ~5% slower (expected; now correct)

---

## [0.7.0] - 2024-11-15 (Previous Release)

### Baseline Behavior (Pre-0.8.0)

For comparison with 0.8.0 changes:

- Metrics computation used legacy best-effort mode (silent failures)
- FX spot dates used calendar days + adjustment (incorrect per ISDA)
- Calendar resolution fell back to `weekends_only` for unknown IDs
- Swap spread field ambiguous (decimal vs basis points)
- Panicking constructors available without warnings
- Calibration residuals not normalized by notional

### Features

- 40+ instrument types with comprehensive pricing
- Analytical and Monte Carlo pricing engines
- Curve calibration (bootstrap and optimization)
- P&L attribution (parallel, waterfall, metrics-based)
- Cashflow generation with credit and prepayment models
- Covenant management and forward projection

---

## Migration Resources

### Documentation

- **Migration Guide**: `MIGRATION_GUIDE.md` - Comprehensive migration instructions
- **Crate Changelog**: `finstack/valuations/CHANGELOG.md` - Detailed phase-by-phase changes
- **API Docs**: Run `cargo doc --open` for full documentation

### Test Files

- **Golden Tests**: `finstack/valuations/tests/integration/golden/data/instruments/`
- **Integration Tests**: `finstack/valuations/tests/integration/`
- **Migration Examples**: See test files for before/after patterns

### Support

- **Issue Tracker**: [GitHub Issues](https://github.com/yourusername/finstack/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/finstack/discussions)
- **Email**: <support@finstack.dev>

---

## Version History

| Version | Release Date | Status | Key Changes | Severity |
|---------|-------------|--------|-------------|----------|
| **0.8.0** | 2024-12-20 | Current | Safety fixes, market convention compliance | 🔴 Breaking |
| 0.7.0 | 2024-11-15 | Previous | Feature release | - |
| 0.9.0 | Q1 2025 | Planned | Internal cleanup, performance | 🟡 Non-breaking |
| 1.0.0 | Q2 2025 | Planned | Stable APIs, deprecation removal | 🔴 Breaking |

---

## Semantic Versioning Commitment

Starting with 0.8.0, the Finstack workspace follows strict semantic versioning:

- **MAJOR** (x.0.0): Breaking API changes
- **MINOR** (0.x.0): New features, deprecations (backwards compatible)
- **PATCH** (0.0.x): Bug fixes, documentation (backwards compatible)

**Pre-1.0 Note**: Minor version bumps (0.x) may include breaking changes as we stabilize APIs. All breaking changes are documented in this changelog with migration guides.

**Post-1.0 Guarantee**: Once 1.0.0 is released, MAJOR version bumps will be the only source of breaking changes.

---

## Upgrade Instructions

### Quick Upgrade (Recommended)

1. **Update dependencies** in `Cargo.toml`:

   ```toml
   finstack-core = "0.8"
   finstack-valuations = "0.8"
   ```

2. **Follow migration guide**: See `MIGRATION_GUIDE.md` decision tree

3. **Run tests**: Verify behavior with your test suite

4. **Handle new errors**: Add error handling for metrics and calendars

5. **Verify FX dates**: Check FX settlement dates if using multi-currency

### Gradual Migration (For Large Codebases)

1. **Phase 1** (Required): Update metrics to strict mode with explicit error handling
2. **Phase 2** (Recommended): Fix FX settlement if using multi-currency
3. **Phase 3** (If applicable): Update removed constructors to `try_*` variants

---

## Contributing

Found an issue or have a suggestion?

- **Bug Reports**: [GitHub Issues](https://github.com/yourusername/finstack/issues/new?template=bug_report.md)
- **Feature Requests**: [GitHub Discussions](https://github.com/yourusername/finstack/discussions/new?category=ideas)
- **Pull Requests**: See `CONTRIBUTING.md` for guidelines
- **Documentation**: Improvements welcome via PR

---

## License

This project is dual-licensed under MIT OR Apache-2.0.

---

**Changelog Format**: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
**Versioning**: [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
**Last Updated**: 2024-12-20
