## Unsafe / UB risk checklist (deeper)

- **Document invariants**: If a caller must uphold something, put it in docs and add debug asserts.
- **Minimize scope**: Prefer extracting safe helpers and confining `unsafe` to the smallest region.
- **Aliasing rules**: Be suspicious of:
  - Creating multiple `&mut` to overlapping regions
  - Casting raw pointers to references without proving validity and exclusivity
  - Reinterpreting bytes as typed data without alignment/provenance proof
- **Slice construction**: `from_raw_parts(_mut)` requires:
  - Non-null (or valid for zero-length), correctly aligned pointer
  - Length in elements fits allocation and doesn’t overflow `isize`
  - Memory is initialized for reads (or write-only usage is explicit)
- **FFI**:
  - Never unwind across FFI. If panics are possible, catch and convert to an error code.
  - Validate all inputs at the boundary; treat FFI inputs as untrusted.
  - Be explicit about ownership (who allocates / who frees) and ABI layout guarantees.

## Concurrency checklist (deeper)

- **Locking**:
  - Ensure consistent lock ordering across code paths.
  - Avoid holding locks across I/O, awaits, or long computations unless justified.
- **Atomics**:
  - Prefer `SeqCst` only when you can’t justify a weaker ordering.
  - If using `Acquire/Release/Relaxed`, require a short explanation of the “happens-before” intent.
- **Async**:
  - Avoid `std::sync::Mutex` in async contexts unless it’s guaranteed uncontended and non-awaiting.
  - Ensure spawned tasks have a shutdown/cancellation path; avoid “fire and forget” leaks.

## Performance checklist (deeper)

- **Hot path hygiene**:
  - Watch for hidden allocations (`format!`, `to_string`, `collect::<Vec<_>>()`).
  - Avoid repeated work in loops (recomputing constants, repeated parsing).
- **Data structures**:
  - Pick structures matching access patterns (contiguous vectors vs hash maps vs btrees).
  - Watch for hash DoS risks if inputs can be attacker-controlled; prefer hardened hashers where appropriate.
- **Benchmark discipline**:
  - Prefer micro-benches for tight loops and integration benches for end-to-end scenarios.
  - Compare before/after with stable inputs and report variance.

## Public API / semver checklist (deeper)

- **Breaking changes**:
  - Renames/removals/type changes are breaking; require justification and migration guidance.
  - Consider deprecation paths and compatibility layers if appropriate.
- **Errors**:
  - Expose stable error surfaces; avoid leaking internal types in public errors unless intentional.
  - Ensure error messages are useful and stable enough for logs (but not relied upon for parsing).
