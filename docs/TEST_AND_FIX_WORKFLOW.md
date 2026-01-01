# Test and Fix Workflow

This directory contains a comprehensive test and auto-fix workflow for the RFin project that ensures code quality across all languages (Rust, Python, TypeScript) before committing, merging, or deploying.

## 🚀 Quick Start

### Run all tests and auto-fix issues

```bash
./run-tests-and-fix
```

### Run tests for specific languages

```bash
./run-tests-and-fix --rust-only      # Rust only
./run-tests-and-fix --python-only    # Python only
./run-tests-and-fix --wasm-only      # WASM only
./run-tests-and-fix --ui-only        # UI only
```

### Generate coverage reports

```bash
./run-tests-and-fix --coverage
```

### Skip slow tests

```bash
./run-tests-and-fix --skip-slow
```

## 📋 Features

### ✅ Automated Checks

- **Rust**: Formatting (rustfmt), Linting (clippy), Unit tests, Doc tests
- **Python**: Formatting (ruff), Linting (ruff), Unit tests (pytest)
- **WASM**: Formatting (prettier), Linting (eslint), Unit tests
- **UI**: Formatting (prettier), Linting (eslint), Unit tests (vitest)
- **Schema Validation**: JSON schema parity tests
- **Pre-commit Hooks**: Full integration with existing hooks

### 🔧 Auto-Fix Capabilities

- Automatic formatting for all languages
- Auto-fixable linting issues
- Import organization
- Dead code removal
- Security issue fixes

### 📊 Coverage Reports

- Rust: `cargo llvm-cov` with HTML output
- Python: `pytest-cov` with HTML output
- UI: Vitest coverage with HTML output

## 🛠️ Configuration Options

### Command Line Options

| Option | Description |
|--------|-------------|
| `--rust-only` | Run only Rust tests and checks |
| `--python-only` | Run only Python tests and checks |
| `--wasm-only` | Run only WASM tests and checks |
| `--ui-only` | Run only UI tests and checks |
| `--no-fix` | Skip automatic fixes |
| `--coverage` | Generate coverage reports |
| `--skip-slow` | Skip slow-running tests |
| `--verbose` | Enable verbose output |
| `--help, -h` | Show help message |

### Environment Variables

```bash
export SKIP_SLOW=true      # Skip slow tests
export COVERAGE=true       # Generate coverage reports
export VERBOSE=true        # Enable verbose output
```

## 📦 Make Targets

The workflow integrates with the existing Makefile:

```bash
make test-and-fix      # Run all tests and fixes
make test-fix-rust     # Rust only
make test-fix-python   # Python only
make test-fix-wasm     # WASM only
make test-fix-ui       # UI only
```

## 🔄 CI/CD Integration

### GitHub Actions

The workflow includes a GitHub Actions configuration (`.github/workflows/test-and-fix.yml`) that:

1. Runs on every push and PR
2. Supports manual triggering with options
3. Can create PRs with auto-fixes
4. Uploads coverage reports
5. Comments on PRs with results

### Manual Trigger

You can manually trigger the workflow with custom options:

1. Go to Actions → Test and Auto-Fix
2. Click "Run workflow"
3. Choose your options:
   - Run only specific language tests
   - Generate coverage reports
   - Skip slow tests
   - Create PR with fixes

## 🪝 Pre-commit Integration

### Quick Pre-commit Hook

For faster commits, use the quick pre-commit hook:

```bash
# Install the quick pre-commit hook
cp .git/hooks/pre-commit-test-fix .git/hooks/pre-commit

# Set quick mode for faster commits
export QUICK_MODE=true
git commit -m "your message"
```

### Full Pre-commit Hook

For thorough checks before commits:

```bash
# Use the full workflow as pre-commit
./run-tests-and-fix
git add -A  # Stage any auto-fixes
git commit -m "your message"
```

## 📝 Usage Examples

### Daily Development

```bash
# Before committing
./run-tests-and-fix

# Quick check for staged files only
QUICK_MODE=true ./run-tests-and-fix

# Run with coverage
./run-tests-and-fix --coverage
```

### Pre-merge Checks

```bash
# Full suite including slow tests
./run-tests-and-fix

# Generate coverage for review
./run-tests-and-fix --coverage --skip-slow
```

### CI/CD Pipeline

```yaml
# In your CI pipeline
- name: Run tests and fixes
  run: ./run-tests-and-fix --no-fix --coverage

- name: Upload coverage
  uses: actions/upload-artifact@v4
  with:
    name: coverage
    path: |
      target/llvm-cov/html/
      finstack-py/htmlcov/
      finstack-ui/coverage/
```

## 🔍 Troubleshooting

### Common Issues

1. **Rust tests fail**:

   ```bash
   # Check specific test
   cargo test <test_name>

   # Update dependencies
   cargo update
   ```

2. **Python tests fail**:

   ```bash
   # Rebuild bindings
   cd finstack-py
   python -m maturin develop --release

   # Check specific test
   pytest tests -k <test_name>
   ```

3. **WASM tests fail**:

   ```bash
   # Rebuild WASM package
   cd finstack-wasm
   wasm-pack build --target web

   # Install dependencies
   npm ci
   ```

4. **Auto-fix doesn't work**:
   - Check if issues are auto-fixable
   - Run with `--verbose` for more details
   - Some issues require manual intervention

### Log File

All operations are logged to `.test-fix.log` in the project root.

## 🎯 Best Practices

1. **Before committing**: Always run `./run-tests-and-fix`
2. **Large changes**: Use `--skip-slow` for faster iteration
3. **PRs**: Enable coverage for code review
4. **CI/CD**: Use `--no-fix` in CI to avoid modifying code
5. **Daily**: Use quick mode for frequent commits

## 🤝 Contributing

To add new checks or fixes:

1. Edit `run-tests-and-fix`
2. Add corresponding Make targets
3. Update the GitHub Actions workflow
4. Update this documentation

## 📄 License

This workflow is part of the RFin project and follows the same license terms.
