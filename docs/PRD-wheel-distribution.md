# PRD: Multi-Platform Wheel Distribution

**Status:** Draft
**Author:** Jon Eickmeier
**Created:** 2026-02-27

## Problem

The `finstack` Python package (PyO3/maturin bindings for rfin) has no pre-built wheel pipeline. Consumers must compile from Rust source, which requires the full Rust toolchain, takes 10-30 minutes, and creates friction for:

- **enterprise-finstack analytics-service Docker builds** — currently runs a full Rust compilation inside a multi-stage Dockerfile on every image rebuild
- **Local development on different machines** — only macOS arm64 wheels exist in `target/wheels/`, and only for CPython 3.12
- **Windows users** — no path to install without manual Rust setup
- **CI pipelines** — any downstream project that depends on `finstack` must have Rust in its CI environment

## Goal

Build and publish pre-built wheels for all supported platforms, Python versions, and feature sets so that consumers can `pip install` or `uv add` without a Rust toolchain.

## Platform Matrix

### Operating Systems & Architectures

| Platform | Architecture | Wheel Tag | Priority |
|----------|-------------|-----------|----------|
| Linux | x86_64 | `manylinux_2_28_x86_64` | **P0** — Docker, servers, CI |
| Linux | arm64/aarch64 | `manylinux_2_28_aarch64` | **P0** — Docker on Apple Silicon, ARM servers |
| macOS | arm64 | `macosx_11_0_arm64` | **P0** — local dev (Apple Silicon) |
| macOS | x86_64 | `macosx_10_12_x86_64` | **P1** — legacy Intel Macs |
| Windows | x86_64 | `win_amd64` | **P1** — Windows dev/analytics users |

### Python Versions

| Version | Status | Priority |
|---------|--------|----------|
| CPython 3.12 | Stable, current rfin dev target | **P0** |
| CPython 3.13 | Stable | **P0** |
| CPython 3.14 | Used by enterprise-finstack | **P0** |

### Feature Variants

The `finstack-py` crate has Cargo features that control which I/O backends are compiled in:

| Variant | Cargo Features | Use Case |
|---------|---------------|----------|
| `default` | `scenarios`, `sqlite` | General use, local analytics, notebooks |
| `postgres` | `scenarios`, `sqlite`, `postgres` | Server-side, enterprise-finstack Docker |

**Decision needed:** Ship one wheel with all features compiled in (simplest), or separate wheels per feature set (smaller binaries, but more complex distribution). **Recommendation: single wheel with all features** (`--features scenarios,sqlite,postgres`) — the binary size increase from postgres is minimal (just adds `tokio-postgres` client), and it eliminates the "wrong variant" footgun.

## Build Matrix Summary

Full matrix: **3 Python versions x 5 platforms = 15 wheels** per release.

P0 subset (ship first): **3 Python versions x 3 platforms = 9 wheels**.

## Implementation

### 1. GitHub Actions Workflow: `release-wheels.yml`

Trigger on:
- Git tag push matching `v*` (release)
- Manual `workflow_dispatch` (ad-hoc builds)
- Nightly schedule (optional, for pre-release testing)

Use the [`PyO3/maturin-action`](https://github.com/PyO3/maturin-action) GitHub Action, which handles cross-compilation and manylinux containers.

```yaml
# .github/workflows/release-wheels.yml
name: Build & Publish Wheels

on:
  push:
    tags: ["v*"]
  workflow_dispatch:
    inputs:
      publish:
        description: "Publish to GitHub Release"
        type: boolean
        default: false

jobs:
  build-wheels:
    name: Build ${{ matrix.os }} / ${{ matrix.python }}
    runs-on: ${{ matrix.runner }}
    strategy:
      fail-fast: false
      matrix:
        include:
          # ── Linux x86_64 ──
          - { os: linux-x64, runner: ubuntu-latest, target: x86_64, python: "3.12" }
          - { os: linux-x64, runner: ubuntu-latest, target: x86_64, python: "3.13" }
          - { os: linux-x64, runner: ubuntu-latest, target: x86_64, python: "3.14" }
          # ── Linux arm64 ──
          - { os: linux-arm64, runner: ubuntu-latest, target: aarch64, python: "3.12" }
          - { os: linux-arm64, runner: ubuntu-latest, target: aarch64, python: "3.13" }
          - { os: linux-arm64, runner: ubuntu-latest, target: aarch64, python: "3.14" }
          # ── macOS arm64 (Apple Silicon) ──
          - { os: macos-arm64, runner: macos-14, target: aarch64, python: "3.12" }
          - { os: macos-arm64, runner: macos-14, target: aarch64, python: "3.13" }
          - { os: macos-arm64, runner: macos-14, target: aarch64, python: "3.14" }
          # ── macOS x86_64 (P1) ──
          - { os: macos-x64, runner: macos-13, target: x86_64, python: "3.12" }
          - { os: macos-x64, runner: macos-13, target: x86_64, python: "3.13" }
          - { os: macos-x64, runner: macos-13, target: x86_64, python: "3.14" }
          # ── Windows x86_64 (P1) ──
          - { os: windows-x64, runner: windows-latest, target: x64, python: "3.12" }
          - { os: windows-x64, runner: windows-latest, target: x64, python: "3.13" }
          - { os: windows-x64, runner: windows-latest, target: x64, python: "3.14" }

    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-python@v5
        with:
          python-version: ${{ matrix.python }}

      - uses: PyO3/maturin-action@v1
        with:
          target: ${{ matrix.target }}
          args: >-
            --release
            --out dist
            --manifest-path finstack-py/Cargo.toml
            --features scenarios,sqlite,postgres
            --interpreter python${{ matrix.python }}
          manylinux: auto
          # ARM Linux cross-compilation needs QEMU
          docker-options: ${{ matrix.target == 'aarch64' && '-e JEMALLOC_SYS_WITH_LG_PAGE=16' || '' }}

      - uses: actions/upload-artifact@v4
        with:
          name: wheel-${{ matrix.os }}-py${{ matrix.python }}
          path: dist/*.whl

  # Build source distribution (for fallback pip install from source)
  build-sdist:
    name: Build sdist
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: PyO3/maturin-action@v1
        with:
          command: sdist
          args: --out dist --manifest-path finstack-py/Cargo.toml
      - uses: actions/upload-artifact@v4
        with:
          name: sdist
          path: dist/*.tar.gz

  # Publish to GitHub Release
  publish:
    name: Publish Release
    needs: [build-wheels, build-sdist]
    runs-on: ubuntu-latest
    if: startsWith(github.ref, 'refs/tags/v') || github.event.inputs.publish == 'true'
    permissions:
      contents: write
    steps:
      - uses: actions/download-artifact@v4
        with:
          path: dist
          merge-multiple: true

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: dist/*
          generate_release_notes: true
```

### 2. Local Build Targets (Makefile)

Add targets to the existing Makefile for developers to build wheels locally:

```makefile
# --- Wheel Building ---

MATURIN_FEATURES := scenarios,sqlite,postgres
WHEEL_DIR := target/wheels

.PHONY: wheels wheel-local wheel-docker wheel-all

wheel-local: ## Build wheel for current platform + Python
 @printf "Building wheel for local platform...\n"
 @$(call py_run,maturin build --release \
  --manifest-path finstack-py/Cargo.toml \
  --features $(MATURIN_FEATURES) \
  -o $(WHEEL_DIR))
 @printf "Wheel(s) written to $(WHEEL_DIR)/\n"
 @ls -lh $(WHEEL_DIR)/finstack-*.whl

wheel-docker: ## Build manylinux wheels for Docker (current arch)
 @printf "Building manylinux wheel via Docker...\n"
 docker run --rm \
  -v $(CURDIR):/io \
  -w /io \
  ghcr.io/pyo3/maturin:v1.10 \
  build --release \
  --manifest-path finstack-py/Cargo.toml \
  --features $(MATURIN_FEATURES) \
  -o /io/$(WHEEL_DIR)
 @printf "Wheel(s) written to $(WHEEL_DIR)/\n"
 @ls -lh $(WHEEL_DIR)/finstack-*.whl

wheel-all: ## Build wheels for all locally-available Python versions
 @printf "Building wheels for all available Python interpreters...\n"
 @$(call py_run,maturin build --release \
  --manifest-path finstack-py/Cargo.toml \
  --features $(MATURIN_FEATURES) \
  --find-interpreter \
  -o $(WHEEL_DIR))
 @printf "Wheel(s) written to $(WHEEL_DIR)/\n"
 @ls -lh $(WHEEL_DIR)/finstack-*.whl

wheels: wheel-local ## Alias for wheel-local
```

### 3. Wheel Storage & Distribution

#### Phase 1: GitHub Releases (immediate)

Wheels are attached to GitHub Releases as assets. Consumers install via:

```bash
# Direct URL install
uv pip install "https://github.com/rustfin/rfin/releases/download/v0.4.0/finstack-0.4.0-cp314-cp314-manylinux_2_28_aarch64.whl"

# Or download + install
gh release download v0.4.0 --pattern "finstack-*cp314*manylinux*aarch64*"
uv pip install ./finstack-*.whl
```

For `pyproject.toml` consumers (like enterprise-finstack), reference via URL source:

```toml
[tool.uv.sources]
finstack = { url = "https://github.com/rustfin/rfin/releases/download/v0.4.0/finstack-0.4.0-cp314-cp314-manylinux_2_28_aarch64.whl" }
```

#### Phase 2: Private PyPI Index (future)

When the team needs version resolution, dependency metadata, and multi-version support:

- Host on **GitHub Packages**, **AWS CodeArtifact**, **Cloudsmith**, or **Garage S3 + `dumb-pypi`**
- Consumers add the index: `uv pip install finstack --extra-index-url https://pypi.internal/simple`
- No URL pinning needed — standard version resolution works

#### Phase 3: Public PyPI (eventual)

When finstack is ready for public consumption:

- Publish via `maturin publish` in CI
- Standard `pip install finstack` works everywhere

### 4. enterprise-finstack Docker Integration

With pre-built wheels available from GitHub Releases, the analytics-service Dockerfile simplifies from a multi-stage Rust build to:

```dockerfile
FROM finstack-python-base:latest

ARG FINSTACK_WHEEL_URL

COPY pyproject.toml uv.lock ./

RUN uv sync --frozen --no-dev --no-install-project

# Install pre-built finstack wheel (no Rust toolchain needed)
RUN if [ -n "$FINSTACK_WHEEL_URL" ]; then \
      uv pip install "$FINSTACK_WHEEL_URL"; \
    fi

COPY app/ ./app/
RUN uv sync --frozen --no-dev

EXPOSE 8001
CMD ["uv", "run", "--no-sync", "uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8001"]
```

Or with local wheels from `make wheel-docker`:

```dockerfile
COPY wheels/ /tmp/wheels/
RUN uv pip install /tmp/wheels/finstack-*.whl && rm -rf /tmp/wheels
```

## Wheel Naming Convention

Maturin produces wheels following PEP 427:

```
finstack-{version}-{python_tag}-{abi_tag}-{platform_tag}.whl
```

Examples:

```
finstack-0.4.0-cp312-cp312-manylinux_2_28_x86_64.whl
finstack-0.4.0-cp314-cp314-manylinux_2_28_aarch64.whl
finstack-0.4.0-cp312-cp312-macosx_11_0_arm64.whl
finstack-0.4.0-cp313-cp313-win_amd64.whl
```

## Testing Requirements

### Per-wheel validation (in CI, after build)

Each wheel is installed into a clean virtualenv and smoke-tested:

```bash
uv venv --python $PYTHON_VERSION
uv pip install dist/finstack-*.whl
python -c "
from finstack import Money, Currency, DiscountCurve
m = Money(100.0, Currency.USD)
print(f'OK: {m}')
"
```

### Cross-platform integration test

After all wheels are built, a separate job installs and runs the Python test suite (`finstack-py/tests/`) against each platform's wheel.

## Versioning & Release Flow

1. Bump version in root `Cargo.toml` workspace (`workspace.package.version`)
2. Tag: `git tag v0.4.1 && git push --tags`
3. CI builds all wheels, runs tests, creates GitHub Release with assets
4. Downstream consumers update their version pin

## Open Questions

1. **Feature variants:** Single "all features" wheel vs. separate `finstack` / `finstack[postgres]` extras? Recommendation is single wheel, but worth confirming binary size delta.

2. **Nightly builds:** Should `main` branch pushes publish nightly/dev wheels (e.g., `finstack-0.5.0.dev20260227`)? Useful for enterprise-finstack CI but adds storage overhead.

3. **WASM distribution:** The `finstack-wasm` package has a similar distribution problem. Should this PRD cover npm publishing, or is that a separate effort?

4. **Minimum manylinux version:** `manylinux_2_28` (AlmaLinux 8 / Debian 11+) is the modern default. Any need for older `manylinux_2_17` (CentOS 7)?

## Success Criteria

- [ ] `make wheel-local` produces a working wheel in `target/wheels/` in < 5 min
- [ ] `make wheel-docker` produces a manylinux wheel usable in Docker without Rust
- [ ] CI workflow builds 9+ wheels (P0 matrix) on tag push, all pass smoke tests
- [ ] GitHub Release has all wheels attached and is installable via URL
- [ ] enterprise-finstack Dockerfile builds in < 2 min (vs. ~30 min with Rust compilation)
- [ ] A developer on a fresh machine can `uv pip install <release-url>` and import finstack
