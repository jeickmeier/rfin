from importlib import import_module

# Import the compiled Rust extension module (built by maturin)
_rust = import_module(__name__)

# Re-export everything for convenient `from rfin import ...` usage
globals().update({k: v for k, v in _rust.__dict__.items() if not k.startswith("__")})

# Keep original docstring
__doc__ = _rust.__doc__

# Tell linters/type-checkers we have explicit exports
if hasattr(_rust, "__all__"):
    __all__ = _rust.__all__
