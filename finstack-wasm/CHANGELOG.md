# Changelog

All notable changes to the finstack-wasm crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial CHANGELOG.md for tracking breaking changes and releases
- GitHub Actions CI/CD workflow for WASM builds, tests, and bundle size tracking
- wasm-opt optimization script in package.json
- Comprehensive package.json metadata (keywords, homepage, bugs URL)
- Feature flag documentation in Cargo.toml

### Changed
- Replaced all `unwrap()` calls with `expect()` calls with clear safety documentation
- Enhanced error messages with detailed safety invariants

### Fixed
- Potential panic in metric ID parsing now has explicit invariant documentation
- Fallback date creation now has safety documentation

## [0.1.0] - Initial Release

### Added
- WebAssembly bindings for finstack-core
- WebAssembly bindings for finstack-statements
- WebAssembly bindings for finstack-valuations
- WebAssembly bindings for finstack-scenarios
- WebAssembly bindings for finstack-portfolio
- Support for both web and nodejs targets
- Comprehensive examples with React + TypeScript + Vite
- Complete feature parity with Python bindings
- Zero unsafe code
- Extensive documentation (4010+ doc comments)
- Test suite with wasm_bindgen_test

### Features
- **Core primitives**: Currency, Money, Date, Calendar, DayCount
- **Market data**: Discount/Forward/Hazard/Inflation curves, FX matrices, Vol surfaces
- **Instruments**: 30+ instrument types (bonds, swaps, options, credit, structured products)
- **Pricing & risk**: PricerRegistry with standard models, risk metrics (DV01, CS01, Greeks)
- **Calibration**: Curve calibrators for discount, forward, hazard, inflation, and vol surfaces
- **Statements**: Financial model builder with forecasting and evaluation
- **Scenarios**: Deterministic scenario engine with market shocks and stress testing
- **Portfolio**: Position management and aggregation with explicit FX handling

[Unreleased]: https://github.com/rustfin/rfin/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/rustfin/rfin/releases/tag/v0.1.0

