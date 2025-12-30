#!/bin/bash
# Setup script for Python development environment using uv

set -e

echo "Setting up Python development environment with uv..."

# Clear any existing Python virtual environment variables to avoid conflicts
unset VIRTUAL_ENV
unset PYTHONPATH

# Check if uv is installed
if ! command -v uv &> /dev/null; then
    echo "Installing uv..."
    curl -LsSf https://astral.sh/uv/install.sh | sh
    echo "uv installed. You may need to restart your shell or source your profile."
    echo "Re-running with new uv installation..."
    # Try to source common profile files
    if [ -f "$HOME/.bashrc" ]; then
        source "$HOME/.bashrc" 2>/dev/null || true
    fi
    if [ -f "$HOME/.zshrc" ]; then
        source "$HOME/.zshrc" 2>/dev/null || true
    fi
    # Add to PATH for this session
    export PATH="$HOME/.cargo/bin:$PATH"
fi

# Create virtual environment
echo "Creating virtual environment..."
uv venv

# Install development dependencies using uv pip (which works with the venv)
echo "Installing dependencies..."
uv pip install maturin pytest pytest-benchmark black mypy ruff ipython jupyter

echo "Building finstack-py..."
cd finstack-py
# Activate the virtual environment and run maturin
source ../.venv/bin/activate
maturin develop --release
deactivate
cd ..

echo ""
echo "✅ Setup complete! Virtual environment is ready."
echo ""
echo "To activate the environment:"
echo "  source .venv/bin/activate"
echo ""
echo "To run the example:"
echo "  python examples/python/primitives_python_example.py"
echo ""
echo "Or run directly with uv:"
echo "  uv run python examples/python/primitives_python_example.py"