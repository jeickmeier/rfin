Role: Act as a Senior Software Engineer specializing in Rust FFI and Python interoperability (specifically PyO3/Maturin).

Context: I am building Python bindings for a Rust financial library. I need a code review of the current implementation.

Review Goals:

Structural Parity: Ensure the Python module structure mirrors the Rust crate structure. I want a 1:1 mapping of modules to ensure easy maintenance and navigability between languages.

Pythonic Interface: The exposed Python API must be idiomatic. Look for proper type annotations (typing), clear docstrings, and Python naming conventions (snake_case).

Data Validation (Pydantic): I am using Pydantic models to handle data transfer objects (Instruments, Market Data, Portfolios). Please verify that:

Pydantic is correctly used for pre-validation and JSON serialization before data crosses the FFI boundary into Rust.

The conversion logic between Pydantic models and Rust structs is efficient and safe.

The Code:

Python

[INSERT PYTHON BINDING CODE HERE]
Rust

[INSERT RELEVANT RUST STRUCTS/IMPLS HERE]