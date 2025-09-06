# Code Coverage

This project uses `cargo-llvm-cov` for generating code coverage reports. This tool provides accurate coverage information using LLVM's built-in coverage instrumentation.

## Prerequisites

The coverage tools are already installed and configured in this project. If you need to install them manually:

```bash
# Install LLVM tools
rustup component add llvm-tools-preview

# Install cargo-llvm-cov
cargo install cargo-llvm-cov
```

## Available Coverage Commands

### Quick Coverage Summary
```bash
make coverage
# or directly:
cargo llvm-cov --package finstack-core --package finstack-valuations --package finstack-statements --package finstack-scenarios --package finstack-portfolio --package finstack-io --package finstack-analysis --package finstack-structured-credit
```

This runs all tests and prints a coverage summary to the terminal, showing:
- Line coverage percentage
- Function coverage percentage
- Region coverage percentage
- Detailed breakdown by file

### Generate HTML Report
```bash
make coverage-html
# or directly:
cargo llvm-cov --package finstack-core --package finstack-valuations --package finstack-statements --package finstack-scenarios --package finstack-portfolio --package finstack-io --package finstack-analysis --package finstack-structured-credit --html
```

This generates a detailed HTML report in `target/llvm-cov/html/` that you can open in your browser to explore:
- File-by-file coverage details
- Line-by-line coverage highlighting
- Coverage statistics and trends

### Generate HTML Report and Open in Browser
```bash
make coverage-open
# or directly:
cargo llvm-cov --package finstack-core --package finstack-valuations --package finstack-statements --package finstack-scenarios --package finstack-portfolio --package finstack-io --package finstack-analysis --package finstack-structured-credit --open
```

This generates the HTML report and automatically opens it in your default browser.

### Generate LCOV Report for CI
```bash
make coverage-lcov
# or directly:
cargo llvm-cov --package finstack-core --package finstack-valuations --package finstack-statements --package finstack-scenarios --package finstack-portfolio --package finstack-io --package finstack-analysis --package finstack-structured-credit --lcov
```

This generates an LCOV format report suitable for continuous integration systems and coverage reporting tools.

## Coverage Configuration

The project includes a `.cargo/config.toml` file that configures `cargo-llvm-cov` with sensible defaults:

- **HTML generation**: Enabled by default
- **Test inclusion**: Includes test code in coverage
- **Doctest inclusion**: Includes documentation tests
- **LCOV output**: Generates LCOV format for CI integration

## Excluded Crates

The Python and WASM bindings are **completely excluded** from coverage analysis and workspace builds:

- **`finstack-py`**: Python bindings (PyO3-based glue code)
- **`finstack-wasm`**: WebAssembly bindings (wasm-bindgen-based glue code)

### Exclusion Implementation

1. **Workspace Configuration**: Removed from `default-members` in `Cargo.toml`
2. **Coverage Commands**: Explicit `--exclude` flags in all Makefile coverage targets
3. **File Filtering**: Regex patterns ignore all binding-related files 
4. **Build Isolation**: Coverage runs without compiling binding dependencies

### Why Bindings Are Excluded

1. **No Rust Business Logic**: Primarily FFI glue code with minimal computation
2. **Language-Specific Testing**: Tested in Python/JavaScript, not Rust unit tests  
3. **Compilation Dependencies**: Require Python/Node.js environments
4. **Coverage Accuracy**: Including them would artificially lower percentages
5. **Metadata Conflicts**: Bindings can cause coverage metadata mismatches

## Coverage Output

Coverage reports are generated in the following locations:

- **HTML Report**: `target/llvm-cov/html/index.html`
- **LCOV Report**: `target/llvm-cov/lcov.info`
- **Coverage Data**: `target/llvm-cov/`

## Current Coverage Status

As of the last run, the project has:
- **Overall Coverage**: 62.11% (regions), 75.82% (lines), 68.97% (functions)
- **Function Mismatches**: 141 functions (~4.8% mismatch rate)
- **Core Library**: High coverage in fundamental modules (dates, math, money)
- **Valuations**: Good coverage in core pricing and risk functionality  
- **Bindings**: Completely excluded from coverage analysis and builds

### Coverage by Crate
- **finstack-core**: Well-tested fundamentals (dates, math, market data)
- **finstack-valuations**: Core instrument pricing with room for improvement in metrics
- **Other crates**: Mostly placeholder implementations awaiting development

## Function Mismatch Issues

Coverage reports may show warnings like "X functions have mismatched data". This is caused by LLVM coverage metadata conflicts between compilation units. Common causes and fixes:

### Causes of Function Mismatches
1. **Generic functions**: Functions like `fn foo<T>()` that get monomorphized for each type `T`
2. **Cross-crate inlining**: Generic functions used across multiple crates
3. **Incremental compilation**: Stale coverage metadata from previous builds
4. **Trait object vs generic dispatch**: Mixed usage patterns

### Solutions Applied
1. **Refactored generic functions to use trait objects**: The `build_with_metrics` function was changed from generic to using trait objects to avoid monomorphization
2. **Disabled incremental compilation**: Set `CARGO_INCREMENTAL=0` in coverage commands
3. **Updated build profiles**: Added non-incremental profiles for coverage builds
4. **Clean builds**: Always clean artifacts before coverage runs

### Solutions Applied to Reduce Mismatches

1. **Refactored main generic functions**:
   - Changed `build_with_metrics<I>()` to `build_with_metrics_dyn()` using trait objects
   - Fixed macro-generated trait method calls to use trait objects
   - Converted `impl Into<String>` parameters to `&str` to eliminate monomorphization

2. **Added coverage-specific optimizations**:
   - Disabled incremental compilation with `CARGO_INCREMENTAL=0`
   - Added `#[inline(never)]` to problematic generic functions
   - Enhanced build profiles for coverage consistency

3. **Improved coverage configuration**:
   - Added `skip-functions = true` to ignore dead code
   - Added `remap-path-prefix = true` for path consistency
   - Added `no-demangle = true` to reduce inlining conflicts

### Best Practices to Prevent Mismatches

1. **Prefer trait objects over generics** for functions used across many types:
   ```rust
   // Bad: causes monomorphization
   fn process<T: Trait>(item: T) -> Result<()> { ... }
   
   // Good: uses trait objects
   fn process(item: &dyn Trait) -> Result<()> { ... }
   ```

2. **Use `#[inline(never)]`** on generic functions if trait objects aren't possible

3. **Avoid cross-crate generic instantiation** by keeping generic functions within single crates

4. **Eliminate `impl Into<String>` parameters**:
   ```rust
   // Bad: creates multiple monomorphizations
   pub fn with_name(self, name: impl Into<String>) -> Self { ... }
   
   // Good: single concrete implementation
   pub fn with_name(self, name: &str) -> Self { ... }
   ```

5. **Run coverage with clean builds**:
   ```bash
   make clean && make coverage
   ```

### Current Status

After comprehensive optimization, the project has:
- **137 functions with mismatched data** (down from 142+, ~4.7% mismatch rate)
- **68.97% function coverage**, **75.82% line coverage**
- **Stable coverage infrastructure** with anti-mismatch measures

The remaining 137 mismatches are primarily from fundamental trait implementations across instrument types, which is acceptable for a complex financial computation library.

## Improving Coverage

To improve coverage:

1. **Add Tests**: Write unit tests for untested functions and modules
2. **Integration Tests**: Add tests that exercise multiple components together
3. **Edge Cases**: Test error conditions and boundary cases
4. **Focus on Core**: Prioritize testing the core business logic in `finstack-core` and `finstack-valuations`

## CI Integration

For continuous integration, use the LCOV output:

```yaml
# Example GitHub Actions step
- name: Generate coverage report
  run: make coverage-lcov

- name: Upload coverage to Codecov
  uses: codecov/codecov-action@v3
  with:
    file: target/llvm-cov/lcov.info
```

## Troubleshooting

### Common Issues

1. **"LLVM tools not found"**: Run `rustup component add llvm-tools-preview`
2. **"cargo-llvm-cov not found"**: Run `cargo install cargo-llvm-cov`
3. **Coverage seems low**: Check if tests are actually running the code paths you expect

### Performance Notes

- Coverage generation adds overhead to test execution
- HTML reports can be large for big projects
- Consider using `--no-cfg-coverage` if you encounter compilation issues

## References

- [cargo-llvm-cov Documentation](https://github.com/taiki-e/cargo-llvm-cov)
- [LLVM Coverage Documentation](https://clang.llvm.org/docs/SourceBasedCodeCoverage.html)
- [LCOV Format Specification](http://ltp.sourceforge.net/coverage/lcov/geninfo.1.php)
