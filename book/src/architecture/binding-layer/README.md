# Binding Layer

All business logic lives in Rust crates. The binding crates — `finstack-py`
(Python/PyO3) and `finstack-wasm` (WASM/wasm-bindgen) — handle only:

1. **Type conversion** — Rust types ↔ Python/JS types
2. **Error mapping** — Rust `Result<T, E>` → Python exceptions / JS errors
3. **Ergonomic helpers** — `__repr__`, `__eq__`, fluent builders

This constraint ensures any feature available in Python is automatically
available in WASM (and vice versa) once the thin wrapper is written.

## Detail Pages

- [Python Bindings](python-bindings.md) — PyO3 wrapper pattern, `.pyi` stubs
- [WASM Bindings](wasm-bindings.md) — wasm-bindgen pattern, TypeScript types
