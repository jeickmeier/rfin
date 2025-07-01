#!/bin/bash
# Test script to verify the setup works

set -e

echo "Testing Python setup..."

# Create clean virtual environment
echo "Creating test virtual environment..."
rm -rf .venv-test
uv venv .venv-test

# Activate it
echo "Activating test environment..."
source .venv-test/bin/activate

# Install dependencies
echo "Installing dependencies..."
uv pip install maturin

# Build the Python extension
echo "Building rfin-python..."
cd rfin-python
python -m maturin develop --release
cd ..

# Test import
echo "Testing import..."
python -c "import rfin; print(f'Success! RustFin version: {rfin.__version__}')"

# Cleanup
deactivate
rm -rf .venv-test

echo "Test completed successfully!"