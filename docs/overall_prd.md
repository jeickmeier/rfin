# Product Requirements Document — **RustFin v 1.0 (Consolidated)**

| **Doc ID**       | **RF‑PRD‑1.0‑FINAL‑ALT**                                                               |
| ---------------- | -------------------------------------------------------------------------------------- |
| **Status**       | Development Ready                                                                      |
| **Date**         | 29 June 2025                                                                           |
| **Authors**      | Core Architecture Group                                                                |
| **Stakeholders** | Quant Eng, Trading & Risk, Structured‑Credit Desk, Treasury, Alternatives Desk, IT Ops |

> **Scope note** – This document supersedes *RF‑PRD‑1.0‑FINAL*.
> It folds in support for **Private Equity (PE)** and **Private Real‑Estate (PRE)** instruments, collectively “Alternative Assets”, without reducing any previously‑committed functionality, performance or packaging targets.

---

\## 1 Executive Summary

RustFin is a modular, memory‑safe **Rust** analytics suite covering every major asset bucket in an institutional multi‑strategy book:

* **Spot & listed:** cash, equities, indices, FX, commodities, exchange‑traded futures.
* **Linear & vanilla derivatives:** bonds, FRNs, swaps, caps/floors, swaptions, FX & NDF forwards.
* **Private‑credit features:** PIK (incl. toggle), make‑whole, delayed‑draw revolvers, covenant step‑ups.
* **Structured‑credit:** pool modelling, waterfall execution, tranche analytics.
* **Alternative assets:** commitment/drawdown PE deals & levered/unlevered PRE properties with GP/LP waterfalls, exit value and illiquidity spread.
* **Scenario engine:** unified shocks for curves, vols, FX **and** CDR/CPR/L S/reinvestment price/yield, recovery lag, illiquidity spread, exit multiple / cap‑rate.
* **Greeks:** analytic, finite‑difference and **adjoint‑based first‑ & second‑order** sensitivities.
* **Risk aggregation:** VAR, **Expected Shortfall**, carry/roll, accrual vs MTM P\&L explain, multi‑currency translation.

Bindings ship for **Python** (wheels) and **JavaScript** (WASM), guaranteeing research/production parity.

---

\## 2 Workspace Layout

```
rustfin-workspace/
 ├─ rustfin-core            # curves, calendars, instruments, AD/AAD
 ├─ rustfin-structured      # ABS / CLO engine & tranche analytics
 ├─ rustfin-alternatives    # NEW – PE & PRE cash‑flow + waterfall (feature "alternatives")
 ├─ rustfin-mdc             # feeds, identifier map, snapshot + history
 ├─ rustfin-scenario        # stress generator incl. StructuredShock & IlliquidityShock
 └─ rustfin-portfolio       # position book, VAR/ES, P&L explain, reports
```

*`rustfin‑alternatives`* compiles only when the **`alternatives`** Cargo feature is enabled; the default wheel size target (≤ 12 MB) is preserved.

---

\## 3 Goals & Success Metrics (unchanged targets)

| #  | Goal                                                                     | Acceptance Metric                                                                |
| -- | ------------------------------------------------------------------------ | -------------------------------------------------------------------------------- |
| G1 | Represent ≥ **98 %** of positions held by hedge‑fund / multi‑asset desks | Sample portfolio loads with zero “unsupported” flags                             |
| G2 | High‑volume deterministic VAR & ES                                       | **100 k positions × 10 k scenarios < 90 s** (16 cores)                           |
| G3 | Full first‑order **and** analytic/AAD Γ                                  | Δ/Vega/DV01/CPR‑Δ/Alt‑Δ/Γ produced in ≤ 1.3 × valuation time                     |
| G4 | Precision & reproducibility                                              | 12‑digit match vs Bloomberg for bonds; identical PV with identical snapshot hash |
| G5 | Poly‑glot availability                                                   | PyPI wheels + npm WASM; runnable Jupyter & React demos                           |

---

\## 4 Scope

| **Included (v1.0)**        | **Deferred (v1.1+)**                                                                                                                                |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Everything in §§ 5–9 below | Monte‑Carlo engines; exotic path‑dependent pay‑offs; XVA & margin sim; GUI dashboards; corporate‑action engine; regulatory capital; hedge‑swap legs |

---

\## 5 Functional Requirements (by crate)

\### 5.1 **`rustfin-core`**

| ID             | Capability                                                                                                                           |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| C‑01‒C‑03      | Curves/Surfaces, Bootstrappers/Calibrators, Full instrument set.                                                                     |
| C‑04           | Precision layer (`f64` default, optional `decimal128`).                                                                              |
| C‑05a‑c        | Day‑count, frequency, schedule builder.                                                                                              |
| C‑06 & C‑06a‑b | Calendar engine (iCalendar, composite merge, sync CLI, packaged global calendars).                                                   |
| C‑07           | **Serde everywhere** (see § 6.2)                                                                                                     |
| C‑08           | Greeks API (Analytic, FD, AAD; 2nd‑order Γ).                                                                                         |
| C‑09‒C‑10      | `SpotAsset` (incl. Cash) & `FuturesContract`.                                                                                        |
| C‑11           | Carry/Roll helpers.                                                                                                                  |
| **C‑12‒C‑17**  | Private‑credit extensions: step‑ups, PIK, PIK toggle, make‑whole, delayed‑draw, covenant step triggers.                              |
| C‑18           | `ForwardIndex` mapping.                                                                                                              |
| C‑19           | `FactorKey` taxonomy incl. `ILIQ:<ccy>` bucket.                                                                                      |
| C‑20           | FX settlement‑lag metadata.                                                                                                          |
| C‑21           | Coupon reset‑lag field.                                                                                                              |
| **C‑22**       | `CashFlow::CFKind` extended under `alternatives` feature (`CapitalCall`, `Distribution`, `OperatingIn/Out`, `DebtService`, `CapEx`). |

\### 5.2 **`rustfin-structured`**

| ID       | Capability                                                            |
| -------- | --------------------------------------------------------------------- |
| S‑01     | Loan/Pool loader + cohort aggregation.                                |
| S‑02     | CPR/CDR/LS curves, recovery lag, loss‑timing.                         |
| S‑02b    | Clean‑up call trigger.                                                |
| S‑03     | **Waterfall engine** with tiered hurdles (re‑used by `alternatives`). |
| S‑04     | Tranche analytics (PV, IRR, WAL, DV01, Convexity, AAD sensitivities). |
| S‑05     | `From<StructuredShock>` conversion.                                   |
| S‑06     | JSON step trace (debug).                                              |
| **S‑07** | `WaterfallEngine::step` exposed `pub` for PE/PRE use.                 |

\### 5.3 **`rustfin-alternatives`** *(new)*

| ID   | Capability                                                                                                                                                                    |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A‑01 | `AlternativeAsset` marker trait `: Valuable + Serialize + Deserialize`.                                                                                                       |
| A‑02 | **`PrivateEquityDeal`** (commitment/drawdown model, realised + projected CFs).                                                                                                |
| A‑03 | **`PrivateRealEstate`** (property meta, debt stack, projected NOI & exit cap‑rate).                                                                                           |
| A‑04 | `DCFSpec` (discount‑curve ref, illiquidity spread, exit variables, terminal growth).                                                                                          |
| A‑05 | `DistWaterfall` – new‑type over `structured::WaterfallEngine<Hurdle>`.                                                                                                        |
| A‑06 | `FundVehicle` aggregation (vintage, currency, GP, vector of alt assets).                                                                                                      |
| A‑07 | Valuation: cash‑flow staging → structured waterfall → discounting via `core::CurveRef`; forward‑adjoint seeds for discount‑curve, illiquidity spread, exit multiple/cap‑rate. |
| A‑08 | Greeks: DV01‑style Δ to curve zeroes, Δ/Γ to exit inputs & illiquidity spread.                                                                                                |
| A‑09 | Fully derives `Serialize`, `Deserialize`, `Clone`, `Debug`, `#[repr(C)]`.                                                                                                     |
| A‑10 | Python (`pyo3`) & WASM (`wasm_bindgen`) bindings auto‑generated under feature flag.                                                                                           |

\### 5.4 **`rustfin-mdc`**

| ID      | Capability                                                                |
| ------- | ------------------------------------------------------------------------- |
| M‑01    | Feed adapters (BPIPE, Refinitiv, CSV).                                    |
| M‑02    | `MarketSnapshot`: curves, surfaces, FX, equities, futures; SHA‑256 + UTC. |
| M‑02b   | Historical time‑series store (Parquet).                                   |
| M‑03    | Snapshot persist / reload (`*.rfsnap`).                                   |
| M‑04    | Data‑quality validation (jump, stale, calendar drift).                    |
| M‑05    | Identifier normaliser (`IdMap`).                                          |
| Metrics | Prometheus feed‑lag, snapshot age, calibrator iterations.                 |

\### 5.5 **`rustfin-scenario`**

| ID        | Capability                                                                   |
| --------- | ---------------------------------------------------------------------------- |
| SC‑01     | Historic shock library.                                                      |
| SC‑02     | PCA/Student‑t stochastic VAR generator.                                      |
| SC‑03     | Correlation shock DSL.                                                       |
| SC‑04–05  | `StructuredShock` block → PoolScenario.                                      |
| **SC‑06** | `IlliquidityShock { spread_bp, exit_shift_pct }` with `impl Into<Scenario>`. |

\### 5.6 **`rustfin-portfolio`**

| ID    | Capability                                                        |
| ----- | ----------------------------------------------------------------- |
| P‑01  | Position & hierarchy model incl. `Alt(AlternativeAsset)` variant. |
| P‑02  | Deterministic scenario loop (streaming for memory O(#positions)). |
| P‑03  | Incremental refresh.                                              |
| P‑03b | Expected Shortfall (CVaR).                                        |
| P‑04  | P\&L explain (Base, ΔMarket, ΔPosition).                          |
| P‑04b | Carry/Roll aggregation (realised vs projected CF).                |
| P‑05  | Tag‑based aggregation; CSV/Parquet; gRPC.                         |
| P‑06  | Observability (tracing + Prometheus).                             |
| P‑07  | Accrual vs clean price toggle.                                    |
| P‑08  | Settlement‑date field.                                            |
| P‑09  | Reporting‑currency FX translation.                                |

---

\## 6 Non‑Functional Requirements

\### 6.1 Performance, scalability, safety (unchanged)

* 100 k × 10 k scenario grid < 90 s; linear to 64 cores; memory ≤ 4 GB for 1 M sensitivity matrix; no `unsafe` in public API.

\### 6.2 **Serialisation & round‑trip stability**

|  ID         | Requirement                                                | Acceptance Criteria                                                                                   |
| ----------- | ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
|  N‑SERDE‑01 | **All public types derive/impl `Serialize + Deserialize`** | `cargo insta` snapshot: serialise → deserialise → equal                                               |
|  N‑SERDE‑02 | Backwards‑compatible JSON/CBOR schema                      | New optional fields must use `#[serde(default)]`; enums tagged or `serde_repr` for integer stability. |
|  N‑SERDE‑03 | Zero‑copy DB‑friendliness                                  | JSONB→Postgres & Arrow→Parquet tests in CI; binary identical across Rust/Python/WASM.                 |

\### 6.3 Packaging (unchanged)

* Default wheel < 12 MB; `[feeds]` extra pulls SDKs; `alternatives` feature adds ≤ 300 kB stripped.

---

\## 7 System Architecture

```
  Feeds ─▶ rustfin-mdc ──► MarketSnapshot(Arc) ─┐
                                                │
                        rustfin-scenario        │
      StructuredShock   ▲       IlliqShock      ▼
                        │                      rustfin-portfolio
 rustfin-core ◄─────────┼──── rustfin-structured   ▲
  (curves, cf, AAD)     │ (waterfall engine)       │
                        │                          │
      rustfin-alternatives (PE/PRE orchestrator)   │
                        │                          │
                 WASM/Python bindings ◄────────────┘
```

---

\## 8 Milestones (30 weeks total, 4 weeks slack consumed)

| Phase                         | Δ      | New Deliverables                                             |
| ----------------------------- | ------ | ------------------------------------------------------------ |
| **M2 – Structured Engine**    | +1 wk  | `FactorKey::ILIQ`, `WaterfallEngine::step` public            |
| **M3 – Scenario + Portfolio** | +2 wks | `rustfin‑alternatives` crate; IlliquidityShock; bindings     |
| **M4 – Docs & Hardening**     | +1 wk  | mdBook chapter “Alternative Assets”; Serde snapshot tests    |
| **GA**                        | –      | Tag v1.0; publish PyPI & npm (with & without `alternatives`) |

Total duration remains **≤ 30 weeks** by overlapping M2/M3 staffing.

---

\## 9 Risks & Mitigation (updated)

| Risk                                       | Mitigation                                                         |
| ------------------------------------------ | ------------------------------------------------------------------ |
| Illiquidity spread market data sparse      | Allow manual override; MDC export alert if stale > 7 d             |
| Waterfall semantics drift between PE funds | DistWaterfall new‑type locks schema; golden‑test per fund template |
| Serialisation bloat                        | CBOR default, JSON behind CLI flag; lint for blob > 256 kB         |
| Decimal mode perf hit                      | Off by default; bench thresholds in CI                             |

---

\## 10 Approval

| Role                   | Name                 | Decision |
| ---------------------- | -------------------- | -------- |
| Quant Eng Lead         | \_\_\_\_\_\_\_\_\_\_ | ✅        |
| Trading & Risk Head    | \_\_\_\_\_\_\_\_\_\_ | ✅        |
| Structured Credit Lead | \_\_\_\_\_\_\_\_\_\_ | ✅        |
| Alternatives Desk Lead | \_\_\_\_\_\_\_\_\_\_ | ✅        |
| CTO / Exec Sponsor     | \_\_\_\_\_\_\_\_\_\_ | ✅        |

---

\### One‑line summary
RustFin v 1.0 now **natively prices, shocks and risk‑aggregates Private Equity & Private Real‑Estate positions** via a thin `rustfin‑alternatives` layer that re‑uses existing cash‑flow and waterfall engines, keeps full Serde round‑trip, sustains all performance targets and ships in the original 30‑week timeline.
