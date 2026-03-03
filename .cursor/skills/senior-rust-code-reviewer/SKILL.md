---
name: senior-rust-code-reviewer
description: Reviews Rust code changes with senior-level rigor: correctness, safety (especially unsafe/FFI), concurrency, performance, and public API stability. Use when reviewing pull requests or diffs that include .rs files, unsafe blocks, FFI, threading/async changes, or performance-sensitive code.
---

# Senior Rust Code Reviewer

## Quick start

When asked to review Rust code (or when a change touches `.rs` files), produce a review with:

1. **Summary**: what changed and overall risk level.
2. **Key risks**: 3–7 bullets of the most important correctness/safety/perf/API concerns.
3. **Findings**: grouped by severity with concrete fixes.
4. **Action items**: checklist the author can execute.

If information is missing (bench context, invariants, safety story), treat it as a review finding and request it rather than guessing.

We don't want to have to do this a second time, ensure that you have throughly review each file / code so that the final review is comprehensive / detailed.

## Severity rubric

- **Blocker**: Could cause incorrect results, UB, memory unsafety, deadlocks, data races, security issues, or major API breakage.
- **Major**: Likely bug, footgun, or significant perf regression; unclear invariants; incomplete tests.
- **Minor**: Edge cases, ergonomics, maintainability, unclear naming/docs.
- **Nit**: Style/idiom preference; only mention if it improves clarity or consistency.

## Review checklist

### Correctness & invariants

- Verify input validation and edge cases (empty inputs, NaNs, overflow, time/ordering).
- Ensure invariants are **stated** (docs/comments) and **enforced** (types/assertions/tests).
- Prefer types that make invalid states unrepresentable (newtypes, enums, non-zero, bounded).

### Error handling & panics

- Panics must be intentional and justified (e.g., truly unreachable) with an explanation.
- Prefer returning structured errors over panicking for recoverable cases.
- Ensure errors include actionable context (what/where/which value), without leaking secrets.

### Unsafe / FFI (be strict)

- Keep `unsafe` blocks **minimal** and locally justified.
- Require a **Safety** comment explaining:
  - Preconditions and invariants relied upon
  - Why those invariants hold at this call site
  - What would break if they didn’t
- At FFI boundaries:
  - Validate pointers/lengths, alignment, and lifetimes
  - Be explicit about ownership transfer and who frees what
  - Avoid panics unwinding across FFI
- If the safety story is incomplete, treat as **Blocker**.

### Concurrency / async

- Look for lock ordering, potential deadlocks, and missed wakeups.
- Check `Send`/`Sync` assumptions and interior mutability (`Cell/RefCell/Mutex/RwLock/Atomic*`).
- For async code, avoid blocking calls on async executors; confirm cancellation/drop behavior.
- Validate atomic ordering choices if used (and demand justification if non-trivial).

### Performance & allocations

- Identify algorithmic complexity (\(O(n)\) vs \(O(n^2)\)), especially in loops and hot paths.
- Look for accidental clones/allocations; prefer borrowing and streaming.
- Ensure iteration patterns are efficient and readable; avoid premature micro-optimizations.
- If performance is a goal, request evidence (benchmarks/profiling) or add a regression test/bench.

### API design & stability

- Public API changes must be intentional: naming, docs, error types, feature flags, deprecations.
- Ensure API contracts are documented: units, ranges, timezone/currency conventions, rounding.
- Avoid exposing unnecessary generics/trait bounds that complicate downstream usage.

### Testing & docs

- New behavior needs tests that fail before the change and pass after.
- Add property tests for invariants and determinism where applicable.
- Docs/comments should explain **why** (invariants, trade-offs), not restate the code.

## Default review output template

Use this structure:

```markdown
## Summary
<1–3 bullets: what changed, overall risk>

## Key risks
- <risk 1>
- <risk 2>

## Findings
### Blockers
- <finding> (suggested fix)

### Majors
- <finding> (suggested fix)

### Minors / Nits
- <finding> (optional)

## Action items
- [ ] <action>
- [ ] <action>
```

## Additional resources

- For deeper checklists and examples, see `reference.md`.
