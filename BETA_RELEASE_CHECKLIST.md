# Beta Release Checklist

## Completed Items ✅

### Code Quality & Cleanup
- ✅ **Dead Code Removal**: Removed all unnecessary `#[allow(dead_code)]` attributes and unused code
  - Cleaned up 45+ instances across Rust core, Python bindings, and WASM bindings
  - Removed unused utility functions from parse/error modules
  - Kept only genuinely-used internal helper methods
  
- ✅ **Documentation Alignment**: Updated all documentation to reflect `f64` decision
  - Changed "deterministic" → "reproducible" across docs
  - Updated README.md reproducibility policy
  - Cleaned up bindings documentation (Python & WASM)
  
- ✅ **Feature Flag Cleanup**: Removed `deterministic` feature flag
  - Standardized on `kahan_sum` for stable f64 summation
  - Updated math modules for consistent behavior

- ✅ **Code Safety**: Added `#![deny(unsafe_code)]` to core crate
  
- ✅ **Revolving Credit Decision**: Disabled incomplete `revolving_credit` module for beta
  - Module remains in codebase for future completion
  - Commented out in `mod.rs` to avoid incomplete API exposure

### CI/CD Infrastructure
- ✅ **Comprehensive CI Pipeline** (`.github/workflows/ci.yml`)
  - **Format check**: `cargo fmt`
  - **Linting**: `cargo clippy` with `-D warnings`
  - **Testing**: Multi-OS (Ubuntu, macOS, Windows) × Rust versions (stable, 1.78 MSRV)
  - **Coverage**: `cargo-llvm-cov` with 60% minimum threshold
  - **Python bindings**: Build and test across Python 3.9-3.12
  - **WASM bindings**: Build and test for web target
  - **Examples**: Verify all examples compile
  - **no_std check**: Verify core builds for embedded targets
  - **Dependency audit**: `cargo-deny` for licenses, advisories, bans
  - **MSRV check**: Ensure Rust 1.78 compatibility

- ✅ **Dependency Governance** (`deny.toml`)
  - License compliance (Apache-2.0/MIT compatible)
  - Security advisory checking
  - Multiple version warnings
  - Unknown source blocking

### Testing
- ✅ **All tests passing**: 2000+ tests across workspace
- ✅ **Examples compilation**: All examples build successfully
- ✅ **Doc tests**: All documentation examples verified

## Pending Items 🚧

### High Priority
- ⏳ **Serde Audit**: Audit serialization names and deny unknown fields
  - Add `#[serde(deny_unknown_fields)]` to public types
  - Create golden tests for schema stability
  - Document wire format guarantees

- ⏳ **Bindings Parity Check**: Verify Python/WASM API consistency
  - Regenerate Python `.pyi` stub files
  - Run example scripts in both bindings
  - Document any known limitations per platform

- ⏳ **Unused Dependencies**: Run `cargo-udeps` audit
  - Remove unnecessary dependencies
  - Document essential vs. optional deps

### Medium Priority  
- ⏳ **Observability Audit**: Ensure tracing spans and error context
  - Review error propagation chains
  - Add spans to critical computation paths
  - Document logging/tracing strategy

### Documentation
- ⏳ **Beta Release Notes**: Create comprehensive release notes
- ⏳ **Migration Guide**: Document breaking changes from alpha
- ⏳ **Known Limitations**: Document beta limitations clearly

### Performance
- ⏳ **Benchmark Suite**: Establish baseline performance benchmarks
- ⏳ **Memory Profiling**: Profile memory usage for large portfolios

## Decision Log

### `f64` vs `Decimal`
**Decision**: Use `f64` for beta release
**Rationale**: 
- Performance is critical for financial modeling
- `f64` precision is sufficient for most use cases (15-17 decimal digits)
- Consistent with industry standard (QuantLib, numpy)
- Can revisit `Decimal` support in future if demanded by users

**Mitigation**:
- Use Kahan summation for numerical stability
- Document precision limitations
- Use `Money` type with currency safety
- Test against QuantLib for parity

### Revolving Credit Facility
**Decision**: Disable for beta release
**Rationale**:
- Module is incomplete (missing pricer implementation)
- Better to ship without it than ship broken functionality
- Can be completed and released in a minor version update

**Action**: Module commented out in `instruments/mod.rs`

## Release Criteria

Before tagging beta:
1. All "High Priority" items completed
2. CI passing on all platforms
3. Coverage ≥ 60%
4. Documentation review complete
5. Examples tested manually
6. Beta release notes finalized

## Post-Beta Roadmap

### v0.4 (Post-Beta)
- Complete revolving credit facility
- Add more structured products
- Performance optimizations
- Enhanced error messages

### v1.0 Criteria
- Production deployments with feedback
- Comprehensive documentation
- 80%+ code coverage
- Full QuantLib parity for core instruments
- Stable API (semantic versioning)

## Notes

- **Test Coverage**: Current ~60-70% (estimated), aim for 70%+ before v1.0
- **MSRV**: Rust 1.78 chosen for stability/compatibility balance  
- **Platform Support**: Linux, macOS, Windows fully supported
- **Python Versions**: 3.9-3.12 supported (following numpy compat)
- **WASM**: web and node targets supported

