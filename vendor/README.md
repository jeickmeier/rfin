# Vendored Dependencies

This directory contains local path copies of third-party crates that the
workspace patches through the root `Cargo.toml` `[patch.crates-io]` section.
Each copy is an unmodified snapshot of the published crate at the version
encoded in its directory name (e.g. `polars-error-0.53.0`).

## Why these are vendored

The Finstack workspace depends on Polars 0.53 via `pyo3-polars` 0.26. A subset
of the Polars 0.53 crate family, as published to crates.io, has feature-matrix
and transitive-dependency combinations that conflict with our workspace's
other requirements (notably `jsonschema`, `getrandom`, and the pinned
`wasm-bindgen` version). Vendoring the affected crates at a single known-good
release lets us lock the feature set, unblock the WASM build, and keep
`cargo-deny` advisories stable.

The vendored set is intentionally minimal — only the crates whose published
artifacts we had to override are carried locally. Any Polars crate not listed
here is consumed directly from crates.io.

## Current patches

| Crate           | Version | Upstream repo                                        |
|-----------------|---------|------------------------------------------------------|
| `pyo3-polars`   | 0.26.0  | https://github.com/pola-rs/pyo3-polars               |
| `polars-error`  | 0.53.0  | https://github.com/pola-rs/polars                    |
| `polars-plan`   | 0.53.0  | https://github.com/pola-rs/polars                    |
| `polars-lazy`   | 0.53.0  | https://github.com/pola-rs/polars                    |
| `polars-utils`  | 0.53.0  | https://github.com/pola-rs/polars                    |

## Upgrade checklist

Revisit on every Polars minor release:

1. Try removing one entry at a time from `[patch.crates-io]` in root `Cargo.toml`.
2. Run `cargo build --workspace --all-features` and `cargo test -p finstack-wasm --target wasm32-unknown-unknown` for each removal.
3. If both pass, delete the corresponding `vendor/<crate>-<version>/` directory.
4. If the whole set can be upgraded at once, replace every path entry with the new published version in lockstep and re-vendor only what still conflicts.
5. Update this README's "Current patches" table and commit the change with a note about what the upgrade unblocked.

## What NOT to change here

Do not edit the files inside `vendor/*/` directly — they must stay byte-identical
to the published crate at the pinned version so we can trivially diff against
upstream when an upgrade attempt fails. Any fix required for a vendored crate
should be landed upstream first, then consumed via a version bump here.
