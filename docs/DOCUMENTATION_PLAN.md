# Finstack Documentation Plan

## Overview

Three documentation systems, unified into one navigable site:

| System | Tool | Content | Audience |
|--------|------|---------|----------|
| **Guide (Book)** | mdBook + mdbook-jupyter | Architecture, how-tos, cookbooks, extending | All |
| **Rust API Reference** | rustdoc (existing) | Auto-generated from doc comments | Rust devs |
| **Python API Reference** | mkdocs-material + mkdocstrings | Auto-generated from `.pyi` stubs | Python users |

All code examples appear in parallel: Rust and Python side-by-side, with
placeholders for future WASM/TypeScript equivalents.

---

## Phase 1 — mdBook Guide

### 1.1 Bootstrap

Create `book/` directory with `book.toml` and install the jupyter preprocessor.

```toml
# book/book.toml
[book]
title = "Finstack Guide"
authors = ["Finstack Contributors"]
language = "en"
multilingual = false
src = "src"

[build]
build-dir = "book"

[preprocessor.jupyter]
embed_images = true

[output.html]
default-theme = "light"
preferred-dark-theme = "ayu"
git-repository-url = "https://github.com/your-org/finstack"
additional-css = ["custom.css"]
```

Install prerequisites:

```sh
cargo install mdbook mdbook-jupyter
```

Update Makefile `install-mdbook` target to also install `mdbook-jupyter`.

### 1.2 Book Structure

```
book/src/
├── SUMMARY.md
├── introduction.md
│
├── getting-started/
│   ├── README.md                    # Overview of setup options
│   ├── installation.md              # Rust (Cargo), Python (uv/pip), WASM (npm)
│   ├── quickstart-python.md         # First pricing in 5 minutes
│   ├── quickstart-rust.md           # First pricing in 5 minutes
│   └── quickstart-wasm.md           # Placeholder — WASM quick start
│
├── architecture/
│   ├── README.md                    # Design philosophy, crate map diagram, feature flags
│   ├── core-primitives/
│   │   ├── README.md                # What core provides, module map
│   │   ├── currency-money.md        # Currency, Money, rounding, FX policy
│   │   ├── dates-calendars.md       # Date types, calendars, day-count conventions
│   │   ├── schedules-periods.md     # Schedule generation, fiscal periods
│   │   └── config.md                # FinstackConfig, global settings
│   ├── market-data/
│   │   ├── README.md                # Curve/surface taxonomy, interpolation overview
│   │   ├── discount-curves.md       # DiscountCurve, bootstrapping, multi-curve
│   │   ├── forward-curves.md        # ForwardCurve, projection
│   │   ├── hazard-curves.md         # HazardCurve, survival probabilities
│   │   ├── volatility-surfaces.md   # Vol surfaces, SABR, smile models
│   │   └── fx-rates.md              # FxRateSet, triangulation, cross rates
│   ├── instruments/
│   │   ├── README.md                # Instrument trait, pricer registry, dispatch
│   │   ├── rates.md                 # Bonds, swaps, basis swaps, xccy, caps/floors
│   │   ├── credit.md                # CDS, CDX, structured credit, private credit
│   │   ├── equity.md                # Equity options, exotics, variance swaps
│   │   ├── fx.md                    # FX forwards, options, NDFs
│   │   └── structured.md            # Autocallables, range accruals, convertibles
│   ├── risk/
│   │   ├── README.md                # Risk framework overview, bump vs AAD
│   │   ├── metrics.md               # Metric keys, DV01, CS01, vega, greeks
│   │   ├── attribution.md           # P&L attribution, daily/period decomposition
│   │   └── scenarios.md             # Scenario engine, shock types, stress testing
│   ├── portfolio/
│   │   ├── README.md                # Entity/position model, aggregation design
│   │   ├── valuation.md             # Portfolio valuation engine
│   │   ├── grouping.md              # Grouping, book structure, netting sets
│   │   └── optimization.md          # Portfolio optimization, constraints
│   ├── statements/
│   │   ├── README.md                # Statement modeling overview
│   │   ├── waterfalls.md            # Waterfall engine, node graphs
│   │   ├── covenants.md             # Covenant definitions, monitoring
│   │   └── forecasting.md           # Financial forecasting, adjustments
│   ├── analytics/
│   │   ├── README.md                # Expression engine, analytics overview
│   │   └── expressions.md           # DSL for computed metrics
│   ├── monte-carlo/
│   │   ├── README.md                # MC engine design
│   │   ├── path-generation.md       # SDE models, path generation, antithetic
│   │   └── pricing.md               # MC pricing, convergence, variance reduction
│   └── binding-layer/
│       ├── README.md                # Binding philosophy — logic stays in Rust
│       ├── python-bindings.md       # PyO3 wrapper pattern, error mapping, .pyi stubs
│       └── wasm-bindings.md         # wasm-bindgen pattern, TypeScript types
│
├── cookbooks/
│   ├── README.md                    # How to use cookbooks, conventions
│   ├── curve-building.md            # Bootstrap discount + forward curves
│   ├── bond-pricing.md              # Price bonds, compute DV01/CS01, z-spread
│   ├── swap-pricing.md              # IRS, basis swaps, xccy
│   ├── options-pricing.md           # Caps/floors, swaptions, equity options
│   ├── credit-analysis.md           # CDS pricing, hazard curves, CDX
│   ├── portfolio-valuation.md       # Build portfolio, run valuation, aggregate
│   ├── scenario-analysis.md         # Define shocks, run scenarios, compare
│   ├── statement-modeling.md        # Waterfalls, covenants, forecasting
│   ├── monte-carlo.md              # MC engine setup, path capture, visualization
│   ├── pnl-attribution.md          # Daily P&L decomposition
│   ├── exotic-options.md           # Barriers, autocallables, cliquets, Asian
│   └── margin-netting.md           # Margin calculations, netting sets
│
├── extending/
│   ├── README.md                    # How the extension points work
│   ├── add-instrument.md            # End-to-end: Rust trait → pricer → Python → test
│   ├── add-pricer.md                # Implement Pricer trait, register, test
│   ├── add-python-binding.md        # PyO3 wrapper, .pyi stub, parity test
│   ├── add-wasm-binding.md          # Placeholder — wasm-bindgen pattern
│   ├── add-metric.md               # New risk metric, key convention, aggregation
│   └── add-market-data.md          # New curve/surface type, calibration
│
├── conventions/
│   ├── README.md                    # Why conventions matter
│   ├── naming.md                    # get_*, metric keys, module layout
│   ├── error-handling.md            # Result types, map_error, binding errors
│   ├── testing.md                   # Unit, doctest, parity, integration, coverage gates
│   └── documentation.md            # Links to DOCUMENTATION_STANDARD.md
│
├── reference/
│   ├── README.md                    # Quick-reference index
│   ├── crate-index.md              # Table of all crates with links to rustdoc
│   ├── metric-keys.md              # Full catalog of metric key patterns
│   ├── market-conventions.md       # Day counts, business day rules, quote conventions
│   └── error-catalog.md            # All error types with causes and remedies
│
└── notebooks/                       # Jupyter notebooks rendered via mdbook-jupyter
    ├── README.md                    # How notebooks are organized
    ├── core/
    │   └── *.ipynb                  # Symlinked or copied from finstack-py/examples/notebooks/core/
    ├── valuations/
    │   └── *.ipynb
    ├── statements/
    │   └── *.ipynb
    ├── scenarios/
    │   └── *.ipynb
    └── portfolio/
        └── *.ipynb
```

### 1.3 Content Conventions

**Parallel code blocks.** Every cookbook and architecture page shows Rust and
Python side-by-side using mdBook tabs (or consecutive fenced blocks):

````markdown
**Rust**
```rust
use finstack_core::money::Money;
use finstack_core::currency::USD;

let price = Money::new(99.50, USD);
```

**Python**
```python
from finstack.core import money, currency

price = money.Money(99.50, "USD")
```

**TypeScript (WASM)** — *coming soon*
```typescript
// import { Money } from "finstack-wasm";
// const price = new Money(99.50, "USD");
```
````

**WASM placeholders.** Every section includes a commented-out WASM/TypeScript
block so future binding work has clear insertion points.

**Cross-references.** Architecture pages link to rustdoc and Python API docs:

```markdown
See [`DiscountCurve`](../api/rust/finstack_core/market_data/struct.DiscountCurve.html)
(Rust) or [`finstack.core.market_data.DiscountCurve`](../api/python/core/market_data/#DiscountCurve)
(Python) for the full API.
```

### 1.4 Notebook Integration

Notebooks from `finstack-py/examples/notebooks/` are referenced in `SUMMARY.md`
via relative symlinks into `book/src/notebooks/`. The `mdbook-jupyter`
preprocessor converts them to rendered markdown at build time.

```sh
# Create symlinks (in book/src/notebooks/)
ln -s ../../../../finstack-py/examples/notebooks/core core
ln -s ../../../../finstack-py/examples/notebooks/valuations valuations
ln -s ../../../../finstack-py/examples/notebooks/statements statements
ln -s ../../../../finstack-py/examples/notebooks/scenarios scenarios
ln -s ../../../../finstack-py/examples/notebooks/portfolio portfolio
```

`SUMMARY.md` entries:

```markdown
# Notebooks

- [Notebooks](notebooks/README.md)
  - [Core: Currency, Money & Config](notebooks/core/01_core_basics_currency_money_config.ipynb)
  - [Valuations: Intro & Pricer Registry](notebooks/valuations/01_valuations_intro_pricer_registry.ipynb)
  ...
```

---

## Phase 2 — Python API Reference (mkdocs)

### 2.1 Setup

Add to `pyproject.toml` dev dependencies:

```toml
[project.optional-dependencies]
docs = [
    "mkdocs-material>=9",
    "mkdocstrings[python]>=0.25",
    "mkdocs-gen-files>=0.5",
    "mkdocs-literate-nav>=0.6",
    "mkdocs-section-index>=0.3",
]
```

### 2.2 Configuration

Create `mkdocs.yml` at project root:

```yaml
site_name: Finstack Python API
site_url: https://your-org.github.io/finstack/api/python/
repo_url: https://github.com/your-org/finstack

theme:
  name: material
  palette:
    - scheme: default
      primary: indigo
      toggle:
        icon: material/brightness-7
        name: Dark mode
    - scheme: slate
      primary: indigo
      toggle:
        icon: material/brightness-4
        name: Light mode
  features:
    - navigation.sections
    - navigation.expand
    - search.suggest
    - content.code.copy
    - content.tabs.link

plugins:
  - search
  - mkdocstrings:
      handlers:
        python:
          paths: [finstack-py]
          options:
            docstring_style: google
            show_root_heading: true
            show_source: false
            members_order: source
            show_symbol_type_heading: true
            show_symbol_type_toc: true
  - gen-files:
      scripts:
        - docs/gen_ref_pages.py
  - literate-nav:
      nav_file: SUMMARY.md
  - section-index

nav:
  - Home: index.md
  - API Reference: reference/

markdown_extensions:
  - pymdownx.highlight
  - pymdownx.superfences
  - pymdownx.tabbed:
      alternate_style: true
  - admonitions
  - toc:
      permalink: true
```

### 2.3 Auto-generation Script

Create `docs/gen_ref_pages.py` to walk `finstack-py/finstack/` and generate
a reference page per module:

```python
"""Generate API reference pages from .pyi stubs."""
import mkdocs_gen_files
from pathlib import Path

nav = mkdocs_gen_files.Nav()
root = Path("finstack-py/finstack")

for path in sorted(root.rglob("*.pyi")):
    if path.name == "finstack.pyi":
        continue
    module_path = path.relative_to(root).with_suffix("")
    parts = tuple(module_path.parts)
    if parts[-1] == "__init__":
        parts = parts[:-1]
    if not parts:
        continue
    doc_path = Path("reference", *parts) / "index.md"
    full_module = "finstack." + ".".join(parts)

    with mkdocs_gen_files.open(doc_path, "w") as fd:
        fd.write(f"::: {full_module}\n")

    nav[parts] = str(doc_path)

with mkdocs_gen_files.open("reference/SUMMARY.md", "w") as nav_file:
    nav_file.writelines(nav.build_literate_nav())
```

### 2.4 Makefile Targets

```makefile
.PHONY: python-docs python-docs-serve
python-docs: ## Build Python API docs
 uv run mkdocs build -d site/api/python

python-docs-serve: ## Serve Python API docs with live reload
 uv run mkdocs serve
```

---

## Phase 3 — Unified Publishing

### 3.1 Combined Site Layout

```
site/
├── index.html          # Landing page with links to all three
├── guide/              # mdBook output
├── api/
│   ├── rust/           # rustdoc output
│   └── python/         # mkdocs output
```

### 3.2 Build Script

Create `scripts/build-docs.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

SITE_DIR="site"
rm -rf "$SITE_DIR"
mkdir -p "$SITE_DIR/api/rust" "$SITE_DIR/api/python" "$SITE_DIR/guide"

# 1. Rustdoc
CARGO_INCREMENTAL=1 cargo doc \
    --workspace --exclude finstack-py --exclude finstack-wasm \
    --no-deps --all-features
cp -r target/doc/* "$SITE_DIR/api/rust/"

# 2. mdBook
cd book && mdbook build && cd ..
cp -r book/book/* "$SITE_DIR/guide/"

# 3. mkdocs (Python)
uv run mkdocs build -d "$SITE_DIR/api/python"

# 4. Landing page
cat > "$SITE_DIR/index.html" << 'EOF'
<!DOCTYPE html>
<html>
<head><title>Finstack Documentation</title></head>
<body>
  <h1>Finstack Documentation</h1>
  <ul>
    <li><a href="guide/">Guide & Cookbooks</a></li>
    <li><a href="api/rust/finstack/">Rust API Reference</a></li>
    <li><a href="api/python/">Python API Reference</a></li>
  </ul>
</body>
</html>
EOF

echo "Documentation built in $SITE_DIR/"
```

### 3.3 GitHub Actions Workflow

Create `.github/workflows/docs.yml`:

```yaml
name: Documentation

on:
  push:
    branches: [main]
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: true

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: astral-sh/setup-uv@v4

      - name: Install mdBook
        run: cargo install mdbook mdbook-jupyter

      - name: Install Python doc deps
        run: uv pip install -e ".[docs]"

      - name: Build all docs
        run: bash scripts/build-docs.sh

      - name: Upload artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: site/

  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
```

### 3.4 Makefile Targets

```makefile
.PHONY: docs-all docs-serve-guide docs-serve-python
docs-all: ## Build complete documentation site
 bash scripts/build-docs.sh

docs-serve-guide: book-serve  ## Alias for book-serve

docs-serve-python: python-docs-serve  ## Alias for mkdocs serve
```

---

## Phase 4 — Content Authoring Order

Writing priority based on user value and existing material.

### Tier 1 — Foundation (can start immediately)

| # | Page | Source Material | Notes |
|---|------|----------------|-------|
| 1 | `introduction.md` | README.md | Adapt existing overview |
| 2 | `architecture/README.md` | README.md + lib.rs | Crate map, feature flags, design philosophy |
| 3 | `getting-started/installation.md` | README.md, finstack-py/README.md | Consolidate existing instructions |
| 4 | `getting-started/quickstart-python.md` | Notebook 01_core_basics | Condense to 5-minute walkthrough |
| 5 | `getting-started/quickstart-rust.md` | Rustdoc examples | New, mirror Python quickstart |
| 6 | `reference/crate-index.md` | Cargo.toml workspace | Table format with links |

### Tier 2 — Architecture Deep-Dives

| # | Page | Source Material |
|---|------|----------------|
| 7 | `architecture/core-primitives/` (all) | core crate docs + notebook 01-02 |
| 8 | `architecture/market-data/` (all) | core market_data module + notebook 03 |
| 9 | `architecture/instruments/` (all) | valuations crate docs + notebook 01-val |
| 10 | `architecture/risk/` (all) | valuations risk/attribution + notebook 04 |
| 11 | `architecture/portfolio/` (all) | portfolio crate + portfolio notebooks |
| 12 | `architecture/binding-layer/` (all) | finstack-py/src patterns, AGENTS.md |

### Tier 3 — Cookbooks (highest end-user value)

| # | Page | Source Material |
|---|------|----------------|
| 13 | `cookbooks/curve-building.md` | cookbook/06, 24 + calibration scripts |
| 14 | `cookbooks/bond-pricing.md` | cookbook/08 + bond_capabilities.py |
| 15 | `cookbooks/swap-pricing.md` | cookbook/19 + irs_capabilities.py |
| 16 | `cookbooks/portfolio-valuation.md` | cookbook/01, 21, 26 |
| 17 | `cookbooks/credit-analysis.md` | cookbook/09 + credit_capabilities.py |
| 18 | `cookbooks/scenario-analysis.md` | cookbook/02, 25 |
| 19 | `cookbooks/monte-carlo.md` | cookbook/12 + mc scripts |
| 20 | `cookbooks/options-pricing.md` | cookbook/11, 13 |
| 21 | `cookbooks/statement-modeling.md` | cookbook/22, 23, 27, 28 |
| 22 | `cookbooks/pnl-attribution.md` | cookbook/16 + attribution scripts |
| 23 | `cookbooks/exotic-options.md` | cookbook/11 + barrier/asian/cliquet scripts |
| 24 | `cookbooks/margin-netting.md` | cookbook/05, 30 |

### Tier 4 — Contributor Guides

| # | Page | Source Material |
|---|------|----------------|
| 25 | `extending/add-instrument.md` | AGENTS.md patterns + existing instruments |
| 26 | `extending/add-python-binding.md` | AGENTS.md + finstack-py/src patterns |
| 27 | `extending/add-pricer.md` | Pricer trait + registry pattern |
| 28 | `extending/add-wasm-binding.md` | finstack-wasm/src patterns (placeholder) |
| 29 | `conventions/` (all) | AGENTS.md, DOCUMENTATION_STANDARD.md |
| 30 | `reference/metric-keys.md` | Metric key patterns from valuations |
| 31 | `reference/market-conventions.md` | Day count, BDC from core |
| 32 | `reference/error-catalog.md` | Exception hierarchy from **init**.pyi |

### Tier 5 — Notebooks & Polish

| # | Page | Source Material |
|---|------|----------------|
| 33 | `notebooks/` integration | Symlink existing notebooks, verify mdbook-jupyter renders |
| 34 | Cross-reference audit | Ensure guide ↔ rustdoc ↔ pydoc links work |
| 35 | Landing page + navigation | Final site assembly and polish |

---

## Implementation Tasks

### Infrastructure (do first)

- [ ] Create `book/` directory with `book.toml` and `src/SUMMARY.md`
- [ ] Install `mdbook-jupyter`, update Makefile `install-mdbook` target
- [ ] Create symlinks for notebooks in `book/src/notebooks/`
- [ ] Add `mkdocs.yml` at project root
- [ ] Add `docs/gen_ref_pages.py` for Python API auto-generation
- [ ] Add doc dependencies to `pyproject.toml`
- [ ] Add Makefile targets: `python-docs`, `python-docs-serve`, `docs-all`
- [ ] Create `scripts/build-docs.sh`
- [ ] Verify `make book-serve` renders the skeleton
- [ ] Verify `make python-docs-serve` renders `.pyi` stubs

### Content — Tier 1

- [ ] Write `introduction.md`
- [ ] Write `architecture/README.md` with crate diagram
- [ ] Write `getting-started/installation.md`
- [ ] Write `getting-started/quickstart-python.md`
- [ ] Write `getting-started/quickstart-rust.md`
- [ ] Write `getting-started/quickstart-wasm.md` (placeholder)
- [ ] Write `reference/crate-index.md`

### Content — Tier 2

- [ ] Write `architecture/core-primitives/` (5 files)
- [ ] Write `architecture/market-data/` (6 files)
- [ ] Write `architecture/instruments/` (6 files)
- [ ] Write `architecture/risk/` (4 files)
- [ ] Write `architecture/portfolio/` (4 files)
- [ ] Write `architecture/statements/` (4 files)
- [ ] Write `architecture/analytics/` (2 files)
- [ ] Write `architecture/monte-carlo/` (3 files)
- [ ] Write `architecture/binding-layer/` (3 files)

### Content — Tier 3

- [ ] Write 12 cookbook pages (see Tier 3 table)

### Content — Tier 4

- [ ] Write 4 extending guides
- [ ] Write 4 convention pages
- [ ] Write 4 reference pages

### Publishing

- [ ] Create `.github/workflows/docs.yml`
- [ ] Verify full `scripts/build-docs.sh` pipeline
- [ ] Enable GitHub Pages in repo settings
- [ ] Add doc build to CI (warning on broken links)

---

## WASM Future Considerations

Every section is designed with WASM expansion in mind:

1. **Placeholder blocks**: Each cookbook and architecture page includes a
   commented `TypeScript (WASM)` code block ready to fill in.

2. **`quickstart-wasm.md`**: Exists as a stub from day one.

3. **`extending/add-wasm-binding.md`**: Dedicated guide for adding WASM bindings.

4. **`architecture/binding-layer/wasm-bindings.md`**: Documents the wasm-bindgen
   pattern, TypeScript type generation, and the `finstack-wasm` crate structure.

5. **mkdocs can expand**: When WASM TypeScript docs are needed, TypeDoc output
   can be added to `site/api/wasm/` following the same pattern as rustdoc.

6. **Parity model**: The existing Python ↔ Rust parity test pattern documented
   in `conventions/testing.md` naturally extends to WASM ↔ Rust parity.
