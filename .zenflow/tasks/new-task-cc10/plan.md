# Spec and build

## Configuration
- **Artifacts Path**: {@artifacts_path} → `.zenflow/tasks/{task_id}`

---

## Agent Instructions

Ask the user questions when anything is unclear or needs their input. This includes:
- Ambiguous or incomplete requirements
- Technical decisions that affect architecture or user experience
- Trade-offs that require business context

Do not make assumptions on important decisions — get clarification first.

---

## Workflow Steps

### [x] Step: Technical Specification
<!-- chat-id: 211536c9-8823-4a17-a219-46d25ef431d9 -->

Assess the task's difficulty, as underestimating it leads to poor outcomes.
- easy: Straightforward implementation, trivial bug fix or feature
- medium: Moderate complexity, some edge cases or caveats to consider
- hard: Complex logic, many caveats, architectural considerations, or high-risk changes

Create a technical specification for the task that is appropriate for the complexity level:
- Review the existing codebase architecture and identify reusable components.
- Define the implementation approach based on established patterns in the project.
- Identify all source code files that will be created or modified.
- Define any necessary data model, API, or interface changes.
- Describe verification steps using the project's test and lint commands.

Save the output to `{@artifacts_path}/spec.md` with:
- Technical context (language, dependencies)
- Implementation approach
- Source code structure changes
- Data model / API / interface changes
- Verification approach

If the task is complex enough, create a detailed implementation plan based on `{@artifacts_path}/spec.md`:
- Break down the work into concrete tasks (incrementable, testable milestones)
- Each task should reference relevant contracts and include verification steps
- Replace the Implementation step below with the planned tasks

Rule of thumb for step size: each step should represent a coherent unit of work (e.g., implement a component, add an API endpoint, write tests for a module). Avoid steps that are too granular (single function).

Save to `{@artifacts_path}/plan.md`. If the feature is trivial and doesn't warrant this breakdown, keep the Implementation step below as is.

---

### [x] Step: Phase 1 - Remove `freeze_all_market`
<!-- chat-id: ca45de5f-47a8-465c-a3c9-9372dba85508 -->

1. Replace call site in `attribution/parallel.rs:126` with direct `market_t0.clone()` ✅
2. Remove function definition from `attribution/factors.rs` (lines 521-537) ✅
3. Remove test `test_freeze_all_market` from `attribution/factors.rs` (lines 577-588) ✅
4. Verify no remaining imports or references ✅

---

### [x] Step: Phase 2 - Inline `compute_forward_rate` Stubs
<!-- chat-id: 3069cfe4-479f-4392-a78d-a3fdf912e9d9 -->

1. Update `CapPayoff::on_event` at line 119 to inline the forward rate logic ✅
2. Update `FloorPayoff::on_event` at line 208 to inline the forward rate logic ✅
3. Remove `CapPayoff::compute_forward_rate` method (lines 93-102) ✅
4. Remove `FloorPayoff::compute_forward_rate` method (lines 191-193) ✅
5. Add TODO comments indicating where Hull-White implementation would go ✅

---

### [ ] Step: Phase 3 - Audit Other Unused Parameters

1. Investigate remaining files with unused `_idx` parameters:
   - `calibration/solver/global.rs`
   - `instruments/structured_credit/pricing/stochastic/tree/tree.rs`
   - `instruments/swaption/pricing/tree_valuator.rs`
   - `instruments/common/models/trees/hull_white_tree.rs`
2. Determine if parameters are trait implementations or can be removed
3. Document findings and remove where safe or add `#[allow(unused)]` with explanation

---

### [ ] Step: Verification and Testing

1. Run valuations tests: `make test-rust`
2. Run linting: `make lint-rust`
3. Spot check specific tests:
   - `cargo test --package finstack-valuations attribution`
   - `cargo test --package finstack-valuations rates_payoff`
4. Verify code compiles without errors or warnings
5. Write report to `{@artifacts_path}/report.md` describing:
   - Lines of code removed
   - Tests updated
   - Test results
   - Any discovered edge cases or challenges
