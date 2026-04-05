# Extending Finstack

Guides for adding new functionality to Finstack. Each guide walks through the
full end-to-end process: Rust implementation → Python binding → WASM binding →
tests.

- [Add an Instrument](add-instrument.md) — New instrument type, end-to-end
- [Add a Pricer](add-pricer.md) — Implement the Pricer trait, register, test
- [Add a Python Binding](add-python-binding.md) — PyO3 wrapper, `.pyi` stub, parity test
- [Add a WASM Binding](add-wasm-binding.md) — wasm-bindgen wrapper, TypeScript types
- [Add a Metric](add-metric.md) — New risk metric, key convention, aggregation
- [Add Market Data](add-market-data.md) — New curve or surface type, calibration
