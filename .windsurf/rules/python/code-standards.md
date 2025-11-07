---
trigger: model_decision
description: When python code standards are needed
globs:
---
# Finstack Python Bindings — Code Standards, Docstrings, and Binding Principles

This document defines standards for the `finstack-py` Python bindings (PyO3-based) to ensure consistency, determinism, currency‑safety, and ergonomic APIs. Follow these rules for all new and edited bindings.

## Goals and Principles
- Deterministic behavior; no hidden non‑determinism or global state leaks.
- Currency‑safety: never perform cross‑currency arithmetic in the bindings.
- No new business logic in bindings. Bindings are thin wrappers over Rust core.
- Stable shapes and names: predictable serde/ABI; keep docstrings and signatures stable.
- Ergonomic, discoverable Python APIs with complete docstrings and examples.
- Deny `unsafe`; match core error semantics via idiomatic Python exceptions.

## Module Layout and Registration
- Mirror Rust core structure:
  - `src/core/{currency, money, dates, market_data, math, cashflow, ...}`
  - `src/valuations/...` for valuations bindings.
- Each submodule exposes `register(py, parent)` and returns exported names.
- Parent module re‑exports submodule symbols into `__all__` for discoverability.
- Keep `__all__` exhaustive and sorted; expose only public APIs.

## Parsing and Type Conversions
- Prefer shared parsers in `core/common/args.rs` and label helpers:
  - `CurrencyArg`, `RoundingModeArg`, `BusinessDayConventionArg`, `InterpStyleArg`, `ExtrapolationPolicyArg`.
- Accept both enum instances and snake/kebab-case strings for options.
- For dates, use `py_to_date` and `date_to_py` from `core/utils.rs`.
- For currency/money, use `extract_currency` and `extract_money` to enforce safety.

## Error Mapping
- Convert core errors via `core/error.rs`:
  - Missing id → `KeyError`
  - Validation/argument errors → `ValueError`
  - Calibration/operational failures → `RuntimeError`
  - Interpolation out of bounds → `ValueError`
- Never `unwrap()` on user inputs; use `?` with `core_to_py`.

## Docstring and Signature Standards
- Always provide `#[pyo3(text_signature = "...")]` on public functions and constructors.
- Add module `__doc__` and class/method docstrings using Rust `///` with NumPy-style sections:
  - Summary line, then: "Parameters", "Returns", "Raises", "Examples" (when helpful).
  - Parameter names/types reflect Python view, not Rust internals.
  - Use lowercase snake‑case for enum labels in examples.
- Include at least one example for nontrivial APIs; keep outputs realistic and stable.
- Keep lines readable (<100 chars where practical).

## API Design
- Names: snake_case for functions; PascalCase for classes/enums.
- Constructors:
  - Use `#[new]`/`#[classattr]` for constructors and constants.
  - Provide `from_name` classmethods for enums parsing snake/kebab case.
- Prefer immutable containers; expose builders (`*Builder`) for mutation.
- Avoid surprising coercions. Be explicit about accepted types (e.g., `Currency or str`).

## Performance and Safety
- Do not add heavy computation in bindings; delegate to core.
- Release the GIL only inside core (already handled in Rust).
- Avoid unnecessary clones; clone only when semantically needed.
- Keep caches/singletons in core; bindings may expose read-only views/stats.

## Currency and FX Policy Visibility
- Never perform implicit FX conversion. Expose policies (e.g., `FxConversionPolicy`) and return structured results (`FxRateResult`).
- Preserve policy metadata in result envelopes.

## Determinism and Rounding
- Respect global rounding via `FinstackConfig`; do not implement ad‑hoc rounding.
- Honor output scale/rounding context on formatting paths exposed to Python.

## Tests, Examples, and Stubs
- Provide runnable examples under `finstack-py/examples/` (scripts, notebooks).
- Ensure `uv run maturin develop --release` builds cleanly before committing.
- Prefer `pyo3-stubgen` to generate `.pyi` after APIs stabilize; keep stubs minimal and synced.

## Review Checklist (per PR)
- [ ] Public APIs have `text_signature` and docstrings with Parameters/Returns/Raises.
- [ ] Uses shared parsers (`args.rs`) and `normalize_label` for string inputs.
- [ ] Errors map through `core_to_py`; no `unwrap` on user inputs.
- [ ] No cross‑currency math; `Money` ops enforce same currency.
- [ ] No business logic duplicated in bindings.
- [ ] `__all__` updated and sorted; module registered under parent.
- [ ] Examples added/updated; notebooks/scripts run successfully.
- [ ] `cargo fmt`/`cargo clippy` clean; `uv run maturin develop` succeeds.

## Style Nuggets
- Prefer early returns and small functions; avoid deep nesting.
- Keep `__repr__` informative and stable.
- For operator overloads (`__add__`, etc.), check types and currencies; raise `TypeError`/`ValueError`.
- Maintain parity with Rust names where possible; adapt only for Python idioms.# Finstack Python Bindings — Code Standards, Docstrings, and Binding Principles

This document defines standards for the `finstack-py` Python bindings (PyO3-based) to ensure consistency, determinism, currency‑safety, and ergonomic APIs. Follow these rules for all new and edited bindings.

## Goals and Principles
- Deterministic behavior; no hidden non‑determinism or global state leaks.
- Currency‑safety: never perform cross‑currency arithmetic in the bindings.
- No new business logic in bindings. Bindings are thin wrappers over Rust core.
- Stable shapes and names: predictable serde/ABI; keep docstrings and signatures stable.
- Ergonomic, discoverable Python APIs with complete docstrings and examples.
- Deny `unsafe`; match core error semantics via idiomatic Python exceptions.

## Module Layout and Registration
- Mirror Rust core structure:
  - `src/core/{currency, money, dates, market_data, math, cashflow, ...}`
  - `src/valuations/...` for valuations bindings.
- Each submodule exposes `register(py, parent)` and returns exported names.
- Parent module re‑exports submodule symbols into `__all__` for discoverability.
- Keep `__all__` exhaustive and sorted; expose only public APIs.

## Parsing and Type Conversions
- Prefer shared parsers in `core/common/args.rs` and label helpers:
  - `CurrencyArg`, `RoundingModeArg`, `BusinessDayConventionArg`, `InterpStyleArg`, `ExtrapolationPolicyArg`.
- Accept both enum instances and snake/kebab-case strings for options.
- For dates, use `py_to_date` and `date_to_py` from `core/utils.rs`.
- For currency/money, use `extract_currency` and `extract_money` to enforce safety.

## Error Mapping
- Convert core errors via `core/error.rs`:
  - Missing id → `KeyError`
  - Validation/argument errors → `ValueError`
  - Calibration/operational failures → `RuntimeError`
  - Interpolation out of bounds → `ValueError`
- Never `unwrap()` on user inputs; use `?` with `core_to_py`.

## Docstring and Signature Standards
- Always provide `#[pyo3(text_signature = "...")]` on public functions and constructors.
- Add module `__doc__` and class/method docstrings using Rust `///` with NumPy-style sections:
  - Summary line, then: "Parameters", "Returns", "Raises", "Examples" (when helpful).
  - Parameter names/types reflect Python view, not Rust internals.
  - Use lowercase snake‑case for enum labels in examples.
- Include at least one example for nontrivial APIs; keep outputs realistic and stable.
- Keep lines readable (<100 chars where practical).

## API Design
- Names: snake_case for functions; PascalCase for classes/enums.
- Constructors:
  - Use `#[new]`/`#[classattr]` for constructors and constants.
  - Provide `from_name` classmethods for enums parsing snake/kebab case.
- Prefer immutable containers; expose builders (`*Builder`) for mutation.
- Avoid surprising coercions. Be explicit about accepted types (e.g., `Currency or str`).

## Performance and Safety
- Do not add heavy computation in bindings; delegate to core.
- Release the GIL only inside core (already handled in Rust).
- Avoid unnecessary clones; clone only when semantically needed.
- Keep caches/singletons in core; bindings may expose read-only views/stats.

## Currency and FX Policy Visibility
- Never perform implicit FX conversion. Expose policies (e.g., `FxConversionPolicy`) and return structured results (`FxRateResult`).
- Preserve policy metadata in result envelopes.

## Determinism and Rounding
- Respect global rounding via `FinstackConfig`; do not implement ad‑hoc rounding.
- Honor output scale/rounding context on formatting paths exposed to Python.

## Tests, Examples, and Stubs
- Provide runnable examples under `finstack-py/examples/` (scripts, notebooks).
- Ensure `uv run maturin develop --release` builds cleanly before committing.
- Prefer `pyo3-stubgen` to generate `.pyi` after APIs stabilize; keep stubs minimal and synced.

## Review Checklist (per PR)
- [ ] Public APIs have `text_signature` and docstrings with Parameters/Returns/Raises.
- [ ] Uses shared parsers (`args.rs`) and `normalize_label` for string inputs.
- [ ] Errors map through `core_to_py`; no `unwrap` on user inputs.
- [ ] No cross‑currency math; `Money` ops enforce same currency.
- [ ] No business logic duplicated in bindings.
- [ ] `__all__` updated and sorted; module registered under parent.
- [ ] Examples added/updated; notebooks/scripts run successfully.
- [ ] `cargo fmt`/`cargo clippy` clean; `uv run maturin develop` succeeds.

## Style Nuggets
- Prefer early returns and small functions; avoid deep nesting.
- Keep `__repr__` informative and stable.
- For operator overloads (`__add__`, etc.), check types and currencies; raise `TypeError`/`ValueError`.
- Maintain parity with Rust names where possible; adapt only for Python idioms.