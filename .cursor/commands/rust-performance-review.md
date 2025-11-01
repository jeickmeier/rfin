# ⚡ Rust Performance Code Review — Latency, Throughput, Allocs, Concurrency

**Role:** You are a senior Rust performance engineer. Identify and fix **CPU hotspots, excessive allocations, cache misses, lock contention, async overhead**, and build-config bottlenecks—**without changing external behavior**.

**Scope Inputs (fill these):**

* **Repo/Path:** `{{repo_or_path}}`
* **Critical paths / workloads:** `{{binaries|APIs|hot modules}}`
* **Perf targets:** `{{p50/p95/p99 latency, QPS/throughput, mem cap}}`
* **Platforms/targets:** `{{x86_64-avx2, aarch64-neon, Linux}}`
* **Workload generator:** `{{bench cmd or repro steps}}`

---

## What to Deliver (in order)

1. **Executive Summary (≤10 bullets):** Biggest wins, estimated gains (latency/throughput/allocs), risk level.
2. **Hotspots Table (Markdown)**
   `area | location | evidence | cost | root cause | fix | est gain | risk`
   *Evidence = flamegraph frame %, criterion delta, alloc counts, lock wait time, etc.*
3. **Patch Plan:** Small, ordered commits grouped by theme (allocs, algo, concurrency, build).
4. **Bench Results (before→after):** p50/p95/p99, throughput, peak RSS, total allocs, top frames.
5. **Safety Net:** Tests/bench coverage to prevent perf regressions; include micro + scenario benches.

---

## Investigation Checklist & Tools (run and summarize key outputs)

### Profiling & Tracing

* **CPU:** `cargo flamegraph --bench {{bench}}` or `--bin {{bin}}` (Linux `perf`)
* **Allocations:** `dhat-rs` or `heaptrack` (summarize top alloc sites, total bytes, live bytes)
* **Async:** `tokio-console` for task wait time, wakers, long polls
* **Locks:** `parking_lot_deadlock`, `perf lock`, or tracing spans around mutexes/RwLocks
* **I/O:** `strace -c`/`dtrace` syscall hotspots; read/write sizes & batching

### Bench & Repro

* **Micro:** `criterion` (report regression thresholds, slope, variance)
* **Scenario:** end-to-end workload that reflects real use; export markdown tables
* **Size/Codegen:** `cargo bloat --release --crates` and `cargo llvm-lines` (heavy monomorphization)

### Lints & Static Hints

* `cargo clippy --all-targets -- -W clippy::perf -W clippy::redundant_clone -W clippy::needless_collect -W clippy::or_fun_call -W clippy::large_enum_variant`
* Grep hotspots: `rg -n "(clone\(\)|to_owned\(\)|collect\::<|unwrap\(\)|Mutex|Arc<Mutex)" src`

### Build & Codegen

* Verify and report:

  * `opt-level=3`, `lto=thin`, `codegen-units=1..8`, `strip = "symbols"`, `panic="abort"` (bins), `target-cpu=native`
  * PGO/BOLT if relevant (note steps), `RUSTFLAGS` sanity, incremental off for release

---

## Heuristics & Fix Patterns

### Algorithms & Data Structures

* Replace `O(n^2)` scans in hot paths with indexed/hashed structures (`hashbrown::HashMap`, `FxHash`/`ahash` where safe).
* Reduce monomorphization bloat (over-generic APIs with many instantiations).
* Prefer **iterators + in-place transforms**; avoid intermediate `collect`.

### Allocation & Copies

* Remove needless `clone()/to_owned()`, prefer borrows (`&str`, `Cow`, `SmallVec`, `arrayvec`).
* **Pre-allocate** with `Vec::with_capacity`, `reserve()`, avoid `shrink_to_fit()` in hot paths.
* Use `Bytes`, `&[u8]`, `io::Buf*` for zero-copy I/O.

### Concurrency & Async

* Minimize **lock granularity** and contention; prefer `parking_lot::{Mutex,RwLock}`, or lock-free (`crossbeam`, atomics) when justified.
* Bound queues; batch work; avoid tiny tasks & ping-pong across executors.
* Pin big tasks to one executor; reduce `Arc` churn; consider `slab`/pooling.

### CPU & Cache

* Improve locality (SoA over AoS where appropriate); avoid large enums in tight loops.
* Consider **SIMD** (`std::simd`, `wide`, `packed_simd` compat) for numeric/text kernels.
* Use `#[inline]` (sparingly) on tiny hot fns; `#[cold]` on errors; avoid panics in hot path.

### I/O & Serialization

* Batch syscalls; read/write larger chunks; reuse buffers.
* Prefer `serde` zero-copy (`borrow` features); avoid repeated parse/format within loops.

---

## Output Formats

### A) Hotspots Table (example)

```
area | location | evidence | cost | root cause | fix | est gain | risk
alloc | src/parse.rs:120 | 18% time; 240M alloc bytes (dhat) | p95 +28ms | repeated to_owned() | borrow & reuse buffer | -18ms p95 | low
cpu   | foo::scan() | 25% frame (flamegraph) | p95 +40ms | O(n^2) dedupe | use hashbrown::HashSet | -30ms p95 | medium
lock  | cache.rs:62 | 12% lock wait | throughput -15% | global Mutex | striped RwLock + shard | +18% tput | medium
```

### B) PR Plan (bulleted, ordered)

* **Commit 1 (allocs):** remove redundant clones; add `with_capacity`; reuse buffers.
* **Commit 2 (algo):** replace scan with hashed index; add unit benches.
* **Commit 3 (concurrency):** shard cache; switch to `parking_lot::RwLock`.
* **Commit 4 (build):** enable `lto=thin`, `target-cpu=native`; document PGO steps.
* **Commit 5 (SIMD optional):** vectorize inner loop; guard with feature flag.

### C) Benchmark Report (Markdown)

```
bench                | before p50/p95/p99 | after p50/p95/p99 | Δ% | allocs (tot/live) | RSS peak | notes
parse_small_payload  | 0.9 / 2.1 / 3.7ms | 0.6 / 1.4 / 2.3ms | -36 | 3.2MB / 120KB     | -8MB     | reuse buf
e2e_ingest_10k_msgs  | 480/780/990ms      | 360/590/770ms     | -24 | 1.1GB / 80MB      | -220MB   | sharded cache
```

---

## Acceptance Criteria

* Meets stated **p50/p95/p99** and throughput targets or quantified improvements ≥ `{{X%}}`.
* No functional/semantic changes; tests pass.
* `criterion` benches added for each fixed hotspot with regression thresholds.
* `cargo clippy -D warnings` (perf lints) clean in optimized paths.
* Build config optimized and documented.

---

## Optional Snippets

**Criterion template**

```rust
use criterion::{criterion_group, criterion_main, Criterion, black_box};
fn bench_core(c: &mut Criterion) {
    c.bench_function("parse_small", |b| b.iter(|| crate::parse(black_box(INPUT))));
}
criterion_group!(name = benches; config = Criterion::default().sample_size(200).noise_threshold(0.02); targets = bench_core);
criterion_main!(benches);
```

**dhat-rs harness**

```rust
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    let _prof = dhat::Profiler::new_heap();
    // run scenario
}
```
