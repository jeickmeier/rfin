# Observability Conventions (Rust)

This repo uses **`tracing`** as the single, consistent observability API for Rust code (events, spans, and structured fields).

## Policy

- **Domain crates** (e.g. `finstack-core`, `finstack-valuations`, `finstack-portfolio`, `finstack-statements`) must:
  - Emit diagnostics via `tracing::{trace, debug, info, warn, error}!` and spans (`tracing::instrument`, `tracing::span!`, etc.)
  - **Not** depend directly on the `log` crate.

- **Adapters / binaries / integrations** (CLI, WASM host, Python bindings, examples) are responsible for:
  - Installing a `tracing` subscriber (formatter + filters)
  - Enabling **log-compatibility** so `log`-based third-party dependencies still appear in output

## Bridging guidance

Many ecosystem crates still emit through the `log` facade. The recommended approach is:

- **Collect `log` records into `tracing`** at the application boundary using `tracing_log::LogTracer`
- Use `tracing_subscriber` (or another subscriber) as the single output pipeline

This keeps the internal API surface area uniform (`tracing` everywhere) while remaining compatible with dependencies that use `log`.

## Rationale

- `tracing` supports spans + structured fields, which is useful for deeply nested valuation / scenario / statement evaluation pipelines.
- A single observability API avoids "missing logs" surprises when composing crates.
