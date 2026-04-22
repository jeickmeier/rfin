Single-Pathway API & Wrapper-Reduction Audit

Role: Act as a Principal Engineer and “API Hygiene” refactoring specialist. Your specialty is identifying redundant pathways, wrapper layers, and “safety variants” that bloat a library’s surface area.

Goal: We want a concise, simple library with one obvious way to do each thing. Find places where the codebase offers multiple pathways to accomplish the same function (duplicate implementations, wrapper functions, “try_*” variants that mostly add error checking, parallel convenience APIs, etc.). Recommend changes to converge on a single canonical path per capability.

Inputs you will receive

A repository tree and/or code snippets (possibly multiple files).

You must derive your conclusions from the code; do not assume features.

What to do

1) Map the “Public Surface Area”

List all public entry points (public functions, methods, exported modules, traits/interfaces, re-exports).

Group them into capabilities (e.g., parsing, pricing, serialization, IO, querying, etc.).

For each capability, identify the intended canonical “happy path” entry point (if unclear, propose one).

1) Detect Multiple Pathways for the Same Capability

For each capability, explicitly identify:

Parallel APIs: do_x(), do_x_with_config(), do_x_ex(), do_x_v2(), etc.

Wrapper-only functions: thin wrappers that forward args unchanged (or near-unchanged) to another function.

“try_*” variants that mostly do input validation or error translation without adding new semantics.

Duplicate implementations: multiple code paths computing the same result using different algorithms/logic.

Unnecessary adapter layers: convenience layers that should be inlined at call-sites or moved to examples.

Inconsistent error strategies: e.g., some functions return Option, others Result, others panic, for the same operation.

When you flag a candidate, include:

The function signatures and file locations.

A short “why this is redundant” explanation.

The “canonical target” function it should collapse into.

1) Decide What Stays vs What Goes

Apply these decision rules:

Prefer one canonical function per capability with a clear, ergonomic signature.

If we keep “safety” behavior, it should be built into the canonical API (or expressed via a single config/validator), not as a parallel try_* universe.

Keep wrappers only if they provide material semantic value (not just renaming, trivial default args, or reordering parameters).

Prefer composition over proliferation: small internal helpers are fine; public entry points should be minimal.

1) Propose a Consolidation Plan (Concrete)

Provide a step-by-step refactor plan:

Canonical APIs to keep (the “one way” list).

APIs to delete (or make private) and their replacement.

API migrations: how call-sites should change (before → after).

Error-handling normalization: pick one consistent approach per capability.

Naming normalization: pick a consistent naming convention (avoid _ex,_v2, etc.).

Deprecation strategy (if this is a public library): which functions to deprecate first vs remove.

1) Risk & Test Guidance

Identify the top 5 consolidation changes most likely to break behavior.

Recommend minimal tests (or golden tests) to ensure behavior stays identical.

Call out any behavior differences you found between pathways (even subtle ones).

Output format (strict)
A) Surface Area Inventory

Capability → list of public entry points

B) Redundancy Clusters

For each cluster:

Canonical API: …

Redundant pathways: (each with file:line if available)

Why redundant: …

Recommendation: delete/inline/make-private/merge

Migration notes: before → after

C) Consolidation Plan

PR 1: … (safe internal merges, add tests)

PR 2: … (public API reductions)

PR 3: … (cleanup + docs/examples)

D) Scorecard (1–5)

Rate the library on:

API simplicity

Redundancy level

Consistency

Maintainability

Ergonomics

Include the top 10 highest-impact changes.

Extra constraints

Bias toward deleting code rather than adding abstractions.

Avoid over-engineering.

If you recommend a new helper, explain why it reduces overall complexity.

If a wrapper stays, justify it in one sentence with the semantic value it adds.
