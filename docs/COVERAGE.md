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

The Python and WASM bindings are intentionally excluded from coverage analysis:

- **`finstack-py`**: Python bindings (not tested in Rust tests)
- **`finstack-wasm`**: WebAssembly bindings (not tested in Rust tests)

These bindings are excluded because:
1. They don't contain Rust business logic that needs coverage
2. They're primarily glue code between Rust and other languages
3. They're tested separately in their respective language ecosystems
4. Including them would artificially lower the coverage percentage

## Coverage Output

Coverage reports are generated in the following locations:

- **HTML Report**: `target/llvm-cov/html/index.html`
- **LCOV Report**: `target/llvm-cov/lcov.info`
- **Coverage Data**: `target/llvm-cov/`

## Current Coverage Status

As of the last run, the project has:
- **Overall Coverage**: ~59.49% (regions), ~67.42% (lines)
- **Core Library**: High coverage in tested modules
- **Valuations**: Good coverage in core functionality
- **Python/WASM Bindings**: Excluded from coverage analysis

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
