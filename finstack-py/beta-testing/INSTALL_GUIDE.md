# Finstack Python Bindings - Beta Installation Guide

This guide provides step-by-step installation instructions for beta testers.

## 📋 Prerequisites

### System Requirements

- **Operating System**: macOS 11+, Linux (Ubuntu 20.04+, Debian 11+, RHEL 8+), or Windows 10+
- **Python**: Version 3.9, 3.10, 3.11, or 3.12 (3.12 recommended)
- **Memory**: 4GB RAM minimum, 8GB recommended
- **Disk Space**: 500MB for installation

### Check Python Version

```bash
# Check Python version
python --version

# Should output: Python 3.9.x, 3.10.x, 3.11.x, or 3.12.x
# If not, install Python from https://www.python.org/downloads/
```

### Install pip (if needed)

```bash
# On macOS/Linux
curl https://bootstrap.pypa.io/get-pip.py -o get-pip.py
python get-pip.py

# On Windows (if Python installed via python.org)
# pip should already be installed
python -m pip --version
```

## 🚀 Installation Methods

### Method 1: Install from Wheel (Recommended)

This is the simplest method for beta testers.

#### Step 1: Download Wheel

Find the wheel file for your platform in the beta distribution:

- **macOS (Intel)**: `finstack-1.0.0b1-cp312-cp312-macosx_11_0_x86_64.whl`
- **macOS (ARM/M1/M2)**: `finstack-1.0.0b1-cp312-cp312-macosx_11_0_arm64.whl`
- **macOS (Universal)**: `finstack-1.0.0b1-cp312-cp312-macosx_11_0_universal2.whl`
- **Linux**: `finstack-1.0.0b1-cp312-cp312-manylinux_2_28_x86_64.whl`
- **Windows**: `finstack-1.0.0b1-cp312-cp312-win_amd64.whl`

> **Note**: Replace `cp312` with your Python version (`cp39` = Python 3.9, `cp310` = Python 3.10, etc.)

#### Step 2: Create Virtual Environment

```bash
# Create virtual environment
python -m venv finstack-beta-env

# Activate virtual environment
# On macOS/Linux:
source finstack-beta-env/bin/activate

# On Windows (Command Prompt):
finstack-beta-env\Scripts\activate.bat

# On Windows (PowerShell):
finstack-beta-env\Scripts\Activate.ps1
```

#### Step 3: Install Wheel

```bash
# Navigate to wheel directory
cd /path/to/beta/distribution

# Install wheel
pip install finstack-1.0.0b1-cp312-cp312-macosx_11_0_arm64.whl
```

#### Step 4: Verify Installation

```bash
# Check import works
python -c "import finstack; print(finstack.__version__)"

# Should output: 1.0.0b1

# Check available modules
python -c "import finstack; print(dir(finstack))"
```

### Method 2: Install from Test PyPI

If the beta is published to Test PyPI:

```bash
# Create and activate virtual environment (as above)

# Install from Test PyPI
pip install --index-url https://test.pypi.org/simple/ \
    --extra-index-url https://pypi.org/simple/ \
    finstack==1.0.0b1

# Verify installation
python -c "import finstack; print(finstack.__version__)"
```

> **Note**: `--extra-index-url` is needed for dependencies like `numpy` and `polars` from regular PyPI.

### Method 3: Build from Source (Advanced)

For contributors or advanced testers who want to modify code:

#### Step 1: Clone Repository

```bash
git clone https://github.com/your-org/finstack.git
cd finstack/finstack-py
git checkout v1.0.0-beta.1
```

#### Step 2: Install Rust

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify Rust installation
rustc --version
```

#### Step 3: Install Maturin

```bash
pip install maturin
```

#### Step 4: Build and Install

```bash
# Development build (faster, includes debug info)
maturin develop

# Release build (optimized, slower to compile)
maturin develop --release

# Verify installation
python -c "import finstack; print(finstack.__version__)"
```

## 🧪 Post-Installation Testing

### Run Quick Smoke Test

Create a file `test_install.py`:

```python
"""Quick smoke test for finstack installation."""
import finstack

def test_imports():
    """Test that all major modules can be imported."""
    from finstack import Currency, Money, Date, DayCount
    from finstack import Bond, Deposit, InterestRateSwap
    from finstack import ScenarioSpec, ScenarioEngine
    from finstack import Portfolio, PortfolioBuilder
    from finstack import ModelBuilder, Evaluator
    print("✓ All imports successful")

def test_currency():
    """Test currency operations."""
    usd = Currency.parse("USD")
    eur = Currency.parse("EUR")
    
    m1 = Money.from_code(100.0, "USD")
    m2 = Money.from_code(50.0, "USD")
    
    total = m1.add(m2)
    assert total.amount == 150.0
    print("✓ Currency operations work")

def test_date():
    """Test date operations."""
    date = Date.parse("2024-01-15")
    assert date.year == 2024
    assert date.month == 1
    assert date.day == 15
    print("✓ Date operations work")

def test_bond_creation():
    """Test bond instrument creation."""
    usd = Currency.parse("USD")
    notional = Money.from_code(1_000_000.0, "USD")
    issue = Date.parse("2024-01-15")
    maturity = Date.parse("2029-01-15")
    
    bond = Bond.fixed_semiannual(
        "TEST-2029",
        notional,
        0.05,  # 5% coupon
        issue,
        maturity,
        "USD-OIS"
    )
    print("✓ Bond creation works")

if __name__ == "__main__":
    test_imports()
    test_currency()
    test_date()
    test_bond_creation()
    print("\n✅ All smoke tests passed!")
```

Run the smoke test:

```bash
python test_install.py
```

Expected output:
```
✓ All imports successful
✓ Currency operations work
✓ Date operations work
✓ Bond creation works

✅ All smoke tests passed!
```

### Run Unit Tests (Optional)

If you cloned the repository:

```bash
# Install test dependencies
pip install pytest

# Run unit tests
cd finstack-py
pytest tests/ -v

# Run specific test modules
pytest tests/test_core.py -v
pytest tests/test_valuations.py -v
```

## 🐛 Troubleshooting

### Issue: Import Error "No module named 'finstack'"

**Cause**: Virtual environment not activated or package not installed.

**Solution**:
```bash
# Activate virtual environment
source finstack-beta-env/bin/activate  # macOS/Linux
# OR
finstack-beta-env\Scripts\activate.bat  # Windows

# Reinstall package
pip install --force-reinstall finstack-1.0.0b1-*.whl
```

### Issue: "DLL load failed" on Windows

**Cause**: Missing Visual C++ Redistributable.

**Solution**:
1. Download and install [Microsoft Visual C++ Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist)
2. Restart terminal
3. Try importing again

### Issue: "No matching distribution found"

**Cause**: Wheel not compatible with your Python version or platform.

**Solution**:
```bash
# Check your Python version
python --version

# Check your platform
python -c "import platform; print(platform.platform())"

# Install correct wheel for your platform and Python version
```

### Issue: "ModuleNotFoundError: No module named 'polars'"

**Cause**: Missing dependency (should auto-install with wheel).

**Solution**:
```bash
# Install dependencies manually
pip install polars pandas numpy
```

### Issue: Build from source fails with "Rust compiler not found"

**Cause**: Rust not installed or not in PATH.

**Solution**:
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add to PATH
source $HOME/.cargo/env

# Verify
rustc --version
```

### Issue: Performance is slow / import takes a long time

**Cause**: Debug build or first import (JIT compilation).

**Solution**:
- For wheel users: This should not happen; wheels are pre-compiled.
- For source builds: Use `maturin develop --release` instead of `maturin develop`.
- Subsequent imports will be faster due to Python's bytecode caching.

### Issue: "Symbol not found" on macOS

**Cause**: Binary not compatible with macOS version.

**Solution**:
```bash
# Check macOS version
sw_vers

# Ensure you're using the universal2 wheel for compatibility
pip install finstack-1.0.0b1-cp312-cp312-macosx_11_0_universal2.whl
```

## 📚 Next Steps

After successful installation:

1. **Read the Beta Testing Guide**: See `README.md` for testing instructions
2. **Try the Quickstart**: Follow `docs/tutorials/quickstart.rst`
3. **Explore Examples**: Check `examples/` directory for working code
4. **Run a Use Case**: Implement a workflow relevant to your persona

## 💬 Getting Help

If you encounter issues not covered here:

- **Slack**: #finstack-beta-testing
- **Email**: beta-testing@finstack.io
- **GitHub Issues**: Create issue with label "beta-feedback" and "installation"
- **Office Hours**: Tuesdays 2-3pm EST (Zoom link in Slack)

When reporting installation issues, please include:

1. Operating system and version
2. Python version (`python --version`)
3. Wheel file name
4. Full error message
5. Output of `pip list`

## 🔧 Uninstallation

To remove finstack after beta testing:

```bash
# Deactivate virtual environment
deactivate

# Remove virtual environment
rm -rf finstack-beta-env

# OR, if installed in system Python
pip uninstall finstack
```

---

**Thank you for participating in the beta test!**

Your feedback will help us deliver a production-ready v1.0.0 release.

---

**Version**: 1.0.0-beta.1  
**Last Updated**: [Date]  
**Contact**: beta-testing@finstack.io
