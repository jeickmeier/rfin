# Beta Release Build Guide

This document provides step-by-step instructions for building and packaging the Finstack Python bindings beta release.

## 🎯 Pre-Release Checklist

Before building, ensure:

- [ ] All Phase 1-3 tasks completed (see `plan.md`)
- [ ] All tests passing: `make test-python`
- [ ] All linters passing: `make lint-python`
- [ ] Documentation builds successfully: `cd docs && make html`
- [ ] Version bumped to `1.0.0-beta.1` in:
  - [ ] `Cargo.toml` (`version = "1.0.0-beta.1"`)
  - [ ] `pyproject.toml` (`version = "1.0.0b1"`)
  - [ ] `__init__.py` (`__version__ = "1.0.0b1"`)
- [ ] CHANGELOG.md updated with beta release notes
- [ ] Git tag created: `git tag v1.0.0-beta.1`

## 🏗️ Build Environment Setup

### Install Build Dependencies

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install maturin
pip install maturin

# Install build tools
pip install build twine
```

### Verify Environment

```bash
# Check Rust version (should be 1.75+)
rustc --version

# Check Python version (should be 3.9+)
python --version

# Check maturin version (should be 1.4+)
maturin --version

# Verify workspace builds
cd finstack
cargo build --release
cargo test --release
```

## 📦 Building Wheels

### Single Platform Build

For local testing on your platform:

```bash
cd finstack-py

# Development build (debug, faster compilation)
maturin build

# Release build (optimized, slower compilation)
maturin build --release

# Output location
ls -lh target/wheels/
```

### Multi-Platform Builds

For distribution to beta testers on different platforms:

#### macOS (Universal2)

```bash
# Install targets
rustup target add aarch64-apple-darwin
rustup target add x86_64-apple-darwin

# Build universal wheel
maturin build --release --target universal2-apple-darwin

# Output: finstack-1.0.0b1-cp312-cp312-macosx_11_0_universal2.whl
```

#### Linux (manylinux)

Using Docker for compatibility:

```bash
# Pull manylinux image
docker pull quay.io/pypa/manylinux_2_28_x86_64

# Build wheel in container
docker run --rm -v $(pwd):/io \
  quay.io/pypa/manylinux_2_28_x86_64 \
  bash -c "cd /io && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    source $HOME/.cargo/env && \
    pip install maturin && \
    maturin build --release --manylinux 2_28"

# Output: finstack-1.0.0b1-cp312-cp312-manylinux_2_28_x86_64.whl
```

#### Windows

On Windows machine or CI:

```bash
# Install Visual Studio Build Tools
# https://visualstudio.microsoft.com/downloads/

# Build wheel
maturin build --release

# Output: finstack-1.0.0b1-cp312-cp312-win_amd64.whl
```

### Multi-Python Version Builds

To support multiple Python versions:

```bash
# Install Python versions (using pyenv)
pyenv install 3.9.18
pyenv install 3.10.13
pyenv install 3.11.7
pyenv install 3.12.1

# Build for each version
for version in 3.9 3.10 3.11 3.12; do
  pyenv local $version
  maturin build --release
done

# Verify wheels
ls -lh target/wheels/
```

## 🧪 Testing Wheels

### Local Installation Test

```bash
# Create clean virtual environment
python -m venv test-env
source test-env/bin/activate

# Install wheel
pip install target/wheels/finstack-1.0.0b1-cp312-cp312-macosx_11_0_arm64.whl

# Verify import
python -c "import finstack; print(finstack.__version__)"

# Run smoke tests
cd tests
pytest test_core.py -v
pytest test_valuations.py::test_bond_pricing -v
pytest test_scenarios.py::test_dsl_parser -v

# Cleanup
deactivate
rm -rf test-env
```

### Cross-Platform Testing

Test on each target platform:

1. **macOS (Intel and ARM)**:
   - Install on Intel Mac, verify import and basic tests
   - Install on ARM Mac (M1/M2), verify import and basic tests

2. **Linux (Ubuntu, Debian, RHEL)**:
   - Test on Ubuntu 22.04
   - Test on Debian 11
   - Test on RHEL 8

3. **Windows (10, 11)**:
   - Test on Windows 10
   - Test on Windows 11

### Package Metadata Check

```bash
# Extract wheel contents
unzip -l target/wheels/finstack-1.0.0b1-*.whl

# Check METADATA
unzip -p target/wheels/finstack-1.0.0b1-*.whl finstack-1.0.0b1.dist-info/METADATA

# Verify:
# - Correct version (1.0.0b1)
# - Correct author/maintainer
# - Correct dependencies
# - Correct classifiers (Beta status)
```

## 📋 Distribution Preparation

### Create Distribution Bundle

```bash
cd finstack-py

# Create distribution directory
mkdir -p dist/beta-1

# Copy wheels
cp target/wheels/finstack-1.0.0b1-*.whl dist/beta-1/

# Copy documentation
cp beta-testing/README.md dist/beta-1/
cp beta-testing/FEEDBACK_SURVEY.md dist/beta-1/
cp beta-testing/INSTALL_GUIDE.md dist/beta-1/

# Copy examples
cp -r examples dist/beta-1/

# Create checksums
cd dist/beta-1
for file in *.whl; do
  sha256sum $file > $file.sha256
done

# Create manifest
cat > MANIFEST.txt << EOF
Finstack Python Bindings - Beta 1 Distribution
================================================

Version: 1.0.0-beta.1
Release Date: $(date +%Y-%m-%d)
Git Commit: $(git rev-parse HEAD)

Wheels Included:
$(ls -1 *.whl)

Documentation:
- README.md: Beta testing guide
- FEEDBACK_SURVEY.md: Feedback questionnaire
- INSTALL_GUIDE.md: Installation instructions
- examples/: Working code examples

Checksums: SHA-256 (see *.whl.sha256 files)

Distribution Package Built By: $(whoami)
Build Date: $(date)
EOF

# Create tarball
cd ..
tar -czf finstack-python-1.0.0-beta.1.tar.gz beta-1/
```

### Upload to Test PyPI

For wider beta distribution:

```bash
# Create .pypirc with test credentials
cat > ~/.pypirc << EOF
[testpypi]
repository = https://test.pypi.org/legacy/
username = __token__
password = pypi-YOUR_TEST_TOKEN_HERE
EOF

# Upload to test PyPI
twine upload --repository testpypi target/wheels/finstack-1.0.0b1-*.whl

# Test installation from test PyPI
pip install --index-url https://test.pypi.org/simple/ finstack==1.0.0b1
```

## 📧 Beta Tester Distribution

### Direct Distribution (5-10 Testers)

```bash
# Create personalized packages
for tester in alice bob charlie dana eric; do
  mkdir -p dist/tester-$tester
  
  # Copy wheels for their platform
  cp dist/beta-1/finstack-1.0.0b1-*macos*.whl dist/tester-$tester/ || true
  cp dist/beta-1/finstack-1.0.0b1-*linux*.whl dist/tester-$tester/ || true
  cp dist/beta-1/finstack-1.0.0b1-*win*.whl dist/tester-$tester/ || true
  
  # Copy docs and examples
  cp dist/beta-1/README.md dist/tester-$tester/
  cp dist/beta-1/FEEDBACK_SURVEY.md dist/tester-$tester/
  cp dist/beta-1/INSTALL_GUIDE.md dist/tester-$tester/
  cp -r dist/beta-1/examples dist/tester-$tester/
  
  # Create personalized README
  cat > dist/tester-$tester/START_HERE.md << EOF
Hi $(echo $tester | sed 's/.*/\u&/')!

Welcome to the Finstack Python bindings beta test!

1. Start with INSTALL_GUIDE.md to install the wheel
2. Follow README.md for testing instructions
3. Complete FEEDBACK_SURVEY.md by [deadline]
4. Report bugs via GitHub with label "beta-feedback"

Your feedback is invaluable - thank you!

Finstack Team
EOF
  
  # Create tarball
  tar -czf dist/finstack-beta-$tester.tar.gz -C dist tester-$tester
done

# Send via email or upload to secure location
```

### Email Template

```markdown
Subject: Finstack Python Bindings Beta 1 - Testing Invitation

Hi [Name],

Thank you for agreeing to participate in the Finstack Python bindings beta testing program!

**What is Finstack?**
Finstack is a deterministic, cross-platform financial computation engine with a Rust core and first-class Python bindings. It provides production-grade APIs for pricing instruments, scenario analysis, portfolio management, and financial statement modeling.

**Beta Testing Timeline**:
- Start Date: [Date]
- Feedback Deadline: [Date + 2 weeks]
- Estimated Time: 4-6 hours total

**Getting Started**:
1. Download the beta package: [Link to tarball or wheel]
2. Follow the INSTALL_GUIDE.md for setup
3. Review the README.md for testing checklist
4. Complete the FEEDBACK_SURVEY.md by [deadline]

**What We Need from You**:
- Test 2-3 workflows relevant to your role ([persona])
- Provide feedback on API ergonomics and documentation
- Report any bugs or issues via GitHub
- Complete the feedback survey

**Support Channels**:
- Slack: #finstack-beta-testing
- Email: beta-testing@finstack.io
- Office Hours: Tuesdays 2-3pm EST

**Confidentiality**:
This beta is under NDA. Please do not share the package or details publicly until the GA release.

Thank you for helping make Finstack better!

Best regards,
The Finstack Team
```

## 📊 Post-Distribution Monitoring

### Track Installation Success

```bash
# Monitor downloads from test PyPI
# (requires PyPI stats API access)

# Check for error reports
gh issue list --label "beta-feedback" --state open

# Monitor Slack for questions
# #finstack-beta-testing channel
```

### Weekly Check-In

```bash
# Week 1: Installation and first impressions
# Week 2: Use case implementation and detailed feedback

# Send reminder email to non-respondents
# Offer office hours for troubleshooting
```

## 🔄 Beta 2 Preparation (If Needed)

If critical issues found:

```bash
# Fix issues in codebase
git checkout -b beta-2-fixes

# Cherry-pick critical fixes
git cherry-pick <commit-hash>

# Bump version to 1.0.0-beta.2
# Update CHANGELOG.md

# Rebuild wheels
maturin build --release

# Redistribute to affected testers
```

## ✅ Build Verification Checklist

Before distributing:

- [ ] All wheels build successfully for target platforms
- [ ] Wheels install cleanly in fresh virtual environments
- [ ] Basic smoke tests pass on all platforms
- [ ] Wheel size reasonable (<50MB uncompressed)
- [ ] No debug symbols in release wheels
- [ ] Correct Python version tags (cp39, cp310, cp311, cp312)
- [ ] Correct platform tags (manylinux, macosx, win_amd64)
- [ ] METADATA file contains correct information
- [ ] Distribution bundle includes all documentation
- [ ] Checksums generated for all wheels
- [ ] Beta testers identified and invited
- [ ] Communication channels set up (Slack, email)
- [ ] Issue templates created for feedback

## 📝 Build Notes

**Build Date**: _____________________________

**Git Commit**: _____________________________

**Wheel Sizes**:
- macOS (universal2): _____ MB
- Linux (manylinux_2_28): _____ MB
- Windows (win_amd64): _____ MB

**Build Time**: _____ minutes

**Platform Tested**:
- [ ] macOS (ARM)
- [ ] macOS (Intel)
- [ ] Linux (Ubuntu 22.04)
- [ ] Linux (Debian 11)
- [ ] Windows 11

**Known Issues**: _____________________________

**Build Executed By**: _____________________________

---

**Questions?** Contact build-automation@finstack.io
