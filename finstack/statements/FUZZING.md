# Fuzzing Guide for finstack-statements

This document describes how to run fuzz tests for the `finstack-statements` crate.

## Prerequisites

Install `cargo-fuzz`:

```bash
cargo install cargo-fuzz
```

## Running Fuzz Tests

### Parse Formula Fuzzer

Tests the DSL parser for panics and crashes:

```bash
cd finstack/statements
cargo fuzz run parse_formula -- -max_total_time=60
```

For longer fuzzing sessions:

```bash
cargo fuzz run parse_formula -- -max_total_time=3600  # 1 hour
```

### JSON Deserialization Fuzzer

Tests model deserialization with malformed JSON:

```bash
cd finstack/statements
cargo fuzz run deserialize_model -- -max_total_time=60
```

## Analyzing Results

If a crash is found, artifacts will be saved to `fuzz/artifacts/<target_name>/`.

To reproduce a crash:

```bash
cargo fuzz run <target_name> fuzz/artifacts/<target_name>/<crash_file>
```

## Continuous Integration

For CI, run fuzz tests with a time limit:

```bash
cargo fuzz run parse_formula -- -max_total_time=60 -rss_limit_mb=2048
cargo fuzz run deserialize_model -- -max_total_time=60 -rss_limit_mb=2048
```

## Corpus

The fuzzer builds a corpus of interesting inputs in `fuzz/corpus/<target_name>/`.
These can be committed to version control to improve fuzzing over time.

## Notes

- Fuzzing requires nightly Rust: `rustup default nightly`
- Fuzz tests verify that no panics occur, even with malformed input
- The `deny_unknown_fields` serde attribute ensures graceful handling of invalid JSON

