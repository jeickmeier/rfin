---
description: Run the finstack-simplify phased workflow (Audit → Plan → Refactor → Verify) on a target scope of the finstack workspace
argument-hint: [crate-or-module, e.g. statements/checks or margin or core/market_data/surfaces]
allowed-tools: Read, Write, Edit, Grep, Glob, Bash(git:*), Bash(make:*), Bash(uv:*), Bash(wc:*), Bash(cargo:*), TodoWrite
---

Invoke the **finstack-simplify** skill to run a phased Audit → Plan → Refactor → Verify loop on the target scope.

**Target scope:** `$ARGUMENTS`

If `$ARGUMENTS` is empty, ask the user to specify a crate or module (e.g., `statements/checks`, `margin`, `core/market_data/surfaces`, `scenarios/adapters`). **Do not** audit the whole workspace — per-crate audits are actionable, whole-workspace audits balloon into noise.

## What this command does

1. Loads the `finstack-simplify` skill. **Read every file under `.claude/skills/finstack-simplify/` including the references/ and examples/ subdirectories** before doing anything else. Do not rely on general knowledge — this skill has specific conventions tailored to the finstack triplet.

2. Starts with **Phase 1 (Audit)**:
   - Reads every Rust file in the target scope plus its Python binding (`finstack-py/src/bindings/<crate>/...`) and WASM binding (`finstack-wasm/src/api/<crate_ns>/...`) counterparts.
   - Applies every pattern from `references/slop-patterns.md`.
   - Applies the cross-binding drift checks from `references/binding-drift.md`.
   - Cross-checks against `references/financial-invariants.md`.
   - Produces an Audit Report in the exact format of `examples/audit-report.md`.

3. Stops and waits for the user to confirm scope and priorities before Phase 2.

4. Only after user confirmation: produces a **Consolidation Plan** per `examples/consolidation-plan.md`, then stops again.

5. Only after user picks a slice: executes exactly one slice following `references/refactor-tactics.md` and `references/workflow.md`, produces a **Refactor Diff** per `examples/refactor-diff.md`, runs the full Verify commands, and stops.

## Rules

- **Never** skip a phase.
- **Never** execute more than one slice per invocation without explicit checkpointing.
- **Never** claim "green" without pasting the last lines of `make lint-*` / `make test-*` output.
- **Never** run `cargo test` directly (project rule — no doc tests in the loop).
- **Never** use `--no-verify` on commits or `-c commit.gpgsign=false`.
- **Always** update Python binding, WASM binding, `.pyi`, and `parity_contract.toml` in the same commit as any Rust public-surface change.
- **Always** re-read the slice's target files at Phase 3 start — do not rely on stale audit-time memory.
- **If you find yourself adding an abstraction** to simplify things: stop. The simplification is almost certainly "delete," not "add." Re-read `references/refactor-tactics.md`.
- **If a verify step fails**: stop, report, hypothesize, propose minimal fix, wait for the user's call. Do not power through.

## When to use this command

- The user says "simplify `<crate>`" or "clean up `<module>`" or "there's slop in `<area>`".
- After a large feature merge, as a hygiene pass on the changed crates.
- Before a release, as part of `production-release-prep`.

## When NOT to use this command

- For generic non-finstack code. Use `simplicity-auditor` or `code-simplifier` instead.
- For performance work. Use `performance-reviewer`.
- For bug hunting. Use `bug_hunting`.
- For new feature design. Use `superpowers:brainstorming`.

Begin with Phase 1 on scope: `$ARGUMENTS`
