# PRD: Multi-Platform Package Distribution

**Status:** Approved
**Author:** Jon Eickmeier
**Created:** 2026-02-27

## Problem

The `finstack` Python package (PyO3/maturin) and `finstack-wasm` npm package have no pre-built distribution pipeline. Consumers must compile from Rust source, which requires the full Rust toolchain, takes 10-30 minutes, and creates friction for:

- **enterprise-finstack Docker builds** — runs a full Rust compilation inside a multi-stage Dockerfile on every image rebuild
- **Local development on different machines** — only a single macOS arm64 / CPython 3.12 wheel exists
- **Windows users** — no path to install without manual Rust setup
- **CI pipelines** — any downstream project that depends on `finstack` must have Rust in its CI
- **Frontend consumers** — `finstack-wasm` requires `wasm-pack` + Rust to build from source

## Goal

Build and publish pre-built Python wheels and npm packages for all supported platforms so that consumers can install without a Rust toolchain.

## Decisions

| Question | Decision |
|----------|----------|
| Feature variants | **Single wheel with all features** (`scenarios,sqlite,postgres`). Size delta is minimal; eliminates wrong-variant footgun. |
| Nightly builds | **No.** Not needed at this stage. Only build on tag push + manual dispatch. |
| WASM/npm | **Yes.** npm publish of `finstack-wasm` is in scope for this PRD. |
| Minimum manylinux | **`manylinux_2_28`** (glibc 2.28+: Debian 10+, Ubuntu 20.04+, RHEL 8+). No older targets. |

---

## Part 1: Python Wheels

### Platform Matrix

| Platform | Architecture | Wheel Platform Tag | Priority |
|----------|-------------|-----------|----------|
| Linux | x86_64 | `manylinux_2_28_x86_64` | **P0** — Docker, servers, CI |
| Linux | arm64/aarch64 | `manylinux_2_28_aarch64` | **P0** — Docker on Apple Silicon, ARM servers |
| macOS | arm64 | `macosx_11_0_arm64` | **P0** — local dev (Apple Silicon) |
| macOS | x86_64 | `macosx_10_12_x86_64` | **P1** — Intel Macs |
| Windows | x86_64 | `win_amd64` | **P1** — Windows dev/analytics users |

### Python Versions

| Version | Status | Priority |
|---------|--------|----------|
| CPython 3.12 | Stable, current rfin dev target | **P0** |
| CPython 3.13 | Stable | **P0** |
| CPython 3.14 | Used by enterprise-finstack | **P0** |

### Build Matrix

All wheels built with `--features scenarios,sqlite,postgres` (single variant).

**Full matrix:** 3 Python versions x 5 platforms = **15 wheels** per release.

**P0 subset (ship first):** 3 Python versions x 3 platforms = **9 wheels**.

### Wheel Naming Convention

Per PEP 427:

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

---

## Part 2: npm / WASM Package

### Package Details

| Field | Value |
|-------|-------|
| Package name | `finstack-wasm` |
| Registry | npm (public) |
| Build tool | `wasm-pack` |
| Targets | `web` (ESM, browser/bundler), `nodejs` (CJS, server-side) |

### Build Outputs

`wasm-pack build` produces a self-contained npm package in `pkg/` with:
- `finstack_wasm_bg.wasm` — compiled WASM binary
- `finstack_wasm.js` — JS glue code
- `finstack_wasm.d.ts` — TypeScript type definitions
- `package.json` — npm package metadata

Two targets are built:

| Target | Output Dir | Use Case |
|--------|-----------|----------|
| `--target web` | `pkg/` | Browser apps, bundlers (Vite, webpack, esbuild) |
| `--target nodejs` | `pkg-node/` | Node.js server-side usage |

The primary distribution is the `web` target (published to npm). The `nodejs` target is available as a secondary build artifact.

### Version Sync

The npm package version is currently `0.1.0` in `finstack-wasm/package.json` but the Rust workspace version is `0.4.0`. These must be synced before first publish. The release workflow will set `package.json` version from the git tag.

---

## Part 3: Implementation

### 1. GitHub Actions Workflow: `release.yml`

Single workflow that builds Python wheels, npm package, and publishes both on tag push.

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags: ["v*"]
  workflow_dispatch:
    inputs:
      publish:
        description: "Publish artifacts"
        type: boolean
        default: false

jobs:
  # ── Python Wheels ───────────────────────────────────────────────
  build-wheels:
    name: Wheel / ${{ matrix.os }} / py${{ matrix.python }}
    runs-on: ${{ matrix.runner }}
    strategy:
      fail-fast: false
      matrix:
        include:
          # ── Linux x86_64 ──
          - { os: linux-x64,   runner: ubuntu-latest,  target: x86_64,  python: "3.12" }
          - { os: linux-x64,   runner: ubuntu-latest,  target: x86_64,  python: "3.13" }
          - { os: linux-x64,   runner: ubuntu-latest,  target: x86_64,  python: "3.14" }
          # ── Linux arm64 ──
          - { os: linux-arm64, runner: ubuntu-latest,  target: aarch64, python: "3.12" }
          - { os: linux-arm64, runner: ubuntu-latest,  target: aarch64, python: "3.13" }
          - { os: linux-arm64, runner: ubuntu-latest,  target: aarch64, python: "3.14" }
          # ── macOS arm64 (Apple Silicon) ──
          - { os: macos-arm64, runner: macos-14,       target: aarch64, python: "3.12" }
          - { os: macos-arm64, runner: macos-14,       target: aarch64, python: "3.13" }
          - { os: macos-arm64, runner: macos-14,       target: aarch64, python: "3.14" }
          # ── macOS x86_64 ──
          - { os: macos-x64,   runner: macos-13,       target: x86_64,  python: "3.12" }
          - { os: macos-x64,   runner: macos-13,       target: x86_64,  python: "3.13" }
          - { os: macos-x64,   runner: macos-13,       target: x86_64,  python: "3.14" }
          # ── Windows x86_64 ──
          - { os: windows-x64, runner: windows-latest, target: x64,     python: "3.12" }
          - { os: windows-x64, runner: windows-latest, target: x64,     python: "3.13" }
          - { os: windows-x64, runner: windows-latest, target: x64,     python: "3.14" }

    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-python@v5
        with:
          python-version: ${{ matrix.python }}

      - name: Build wheel
        uses: PyO3/maturin-action@v1
        with:
          target: ${{ matrix.target }}
          args: >-
            --release
            --out dist
            --manifest-path finstack-py/Cargo.toml
            --features scenarios,sqlite,postgres
            --interpreter python${{ matrix.python }}
          manylinux: "2_28"

      - name: Smoke test wheel
        if: matrix.target != 'aarch64' || runner.os != 'Linux'
        run: |
          pip install dist/finstack-*.whl
          python -c "from finstack import Money, Currency; m = Money(100.0, Currency.USD); print(f'OK: {m}')"

      - uses: actions/upload-artifact@v4
        with:
          name: wheel-${{ matrix.os }}-py${{ matrix.python }}
          path: dist/*.whl

  # ── Source Distribution ─────────────────────────────────────────
  build-sdist:
    name: Source dist
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

  # ── WASM / npm Package ─────────────────────────────────────────
  build-wasm:
    name: WASM / npm
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Set up Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: 1.90.0
          target: wasm32-unknown-unknown

      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

      - name: Set up Node.js
        uses: actions/setup-node@v4
        with:
          node-version: "20"
          registry-url: "https://registry.npmjs.org"

      - name: Sync package.json version from tag
        run: |
          VERSION="${GITHUB_REF_NAME#v}"
          cd finstack-wasm
          npm version "$VERSION" --no-git-tag-version --allow-same-version

      - name: Build web target
        run: |
          cd finstack-wasm
          wasm-pack build --target web --release --out-dir pkg
          wasm-pack build --target nodejs --release --out-dir pkg-node

      - name: Package npm tarball
        run: |
          cd finstack-wasm
          npm pack
          mv finstack-wasm-*.tgz ../dist/

      - uses: actions/upload-artifact@v4
        with:
          name: npm-package
          path: dist/finstack-wasm-*.tgz

      - uses: actions/upload-artifact@v4
        with:
          name: wasm-pkg
          path: finstack-wasm/pkg/

  # ── Publish ─────────────────────────────────────────────────────
  publish:
    name: Publish Release
    needs: [build-wheels, build-sdist, build-wasm]
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

  publish-npm:
    name: Publish to npm
    needs: [build-wasm]
    runs-on: ubuntu-latest
    if: startsWith(github.ref, 'refs/tags/v') || github.event.inputs.publish == 'true'
    permissions:
      id-token: write
    steps:
      - uses: actions/download-artifact@v4
        with:
          name: npm-package
          path: dist

      - uses: actions/setup-node@v4
        with:
          node-version: "20"
          registry-url: "https://registry.npmjs.org"

      - name: Publish to npm
        run: npm publish dist/finstack-wasm-*.tgz --provenance --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

### 2. Local Build Targets (Makefile additions)

```makefile
# --- Package Building ---

MATURIN_FEATURES := scenarios,sqlite,postgres
WHEEL_DIR := target/wheels

.PHONY: wheels wheel-local wheel-docker wheel-all wasm-publish-dry

wheel-local: ## Build wheel for current platform + Python
 @printf "Building wheel for local platform...\n"
 @$(call py_run,maturin build --release \
  --manifest-path finstack-py/Cargo.toml \
  --features $(MATURIN_FEATURES) \
  -o $(WHEEL_DIR))
 @printf "Wheel(s) written to $(WHEEL_DIR)/\n"
 @ls -lh $(WHEEL_DIR)/finstack-*.whl

wheel-docker: ## Build manylinux wheel via Docker (current arch)
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

wasm-pack: ## Build WASM package (web + node targets)
 cd finstack-wasm && wasm-pack build --target web --release --out-dir pkg
 cd finstack-wasm && wasm-pack build --target nodejs --release --out-dir pkg-node

wasm-publish-dry: wasm-pack ## Dry-run npm publish (no upload)
 cd finstack-wasm && npm pack --dry-run
```

### 3. Distribution

#### Phase 1: GitHub Releases + npm (immediate)

**Python wheels** are attached to GitHub Releases. Consumers install via:

```bash
# Direct URL install
uv pip install "https://github.com/rustfin/rfin/releases/download/v0.4.0/finstack-0.4.0-cp314-cp314-manylinux_2_28_aarch64.whl"

# Or download + install
gh release download v0.4.0 --pattern "finstack-*cp314*manylinux*aarch64*"
uv pip install ./finstack-*.whl
```

For `pyproject.toml` consumers (like enterprise-finstack analytics-service):

```toml
[project]
dependencies = ["finstack"]

[tool.uv.sources]
finstack = { url = "https://github.com/rustfin/rfin/releases/download/v0.4.0/finstack-0.4.0-cp314-cp314-manylinux_2_28_aarch64.whl" }
```

**npm package** is published to the public npm registry:

```bash
npm install finstack-wasm
```

#### Phase 2: Private PyPI Index (future)

When the team needs version resolution and multi-version support:

- Host on **GitHub Packages**, **AWS CodeArtifact**, **Cloudsmith**, or **Garage S3 + `dumb-pypi`**
- Consumers add the index: `uv pip install finstack --extra-index-url https://pypi.internal/simple`
- No URL pinning needed — standard version resolution works

#### Phase 3: Public PyPI (eventual)

- Publish via `maturin publish` in CI
- Standard `pip install finstack` works everywhere

### 4. enterprise-finstack Docker Simplification

With pre-built wheels on GitHub Releases, the analytics-service Dockerfile drops the multi-stage Rust build entirely:

```dockerfile
FROM finstack-python-base:latest

ARG FINSTACK_WHEEL_URL

COPY pyproject.toml uv.lock ./

# Strip local path source (replaced by pre-built wheel)
RUN sed -i '/"finstack",/d' pyproject.toml \
    && sed -i '/^\[tool\.uv\.sources\]/,/^$/d' pyproject.toml

RUN uv sync --no-dev --no-install-project

# Install pre-built finstack wheel (no Rust toolchain needed)
RUN uv pip install "$FINSTACK_WHEEL_URL"

COPY app/ ./app/
RUN uv pip install --no-deps .

EXPOSE 8001
CMD ["uv", "run", "--no-sync", "uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8001"]
```

Build time drops from ~30 min to ~2 min.

---

## Testing Requirements

### Per-wheel smoke test (in CI, after build)

Each wheel is installed into a clean environment and validated:

```bash
pip install dist/finstack-*.whl
python -c "
from finstack import Money, Currency, DiscountCurve
m = Money(100.0, Currency.USD)
print(f'OK: {m}')
"
```

Note: Linux arm64 wheels are cross-compiled via QEMU and cannot be smoke-tested on x86_64 runners. These are validated via the integration test job.

### npm package smoke test (in CI, after build)

```bash
cd $(mktemp -d)
npm init -y
npm install ../dist/finstack-wasm-*.tgz
node -e "const fs = require('finstack-wasm'); console.log('OK:', Object.keys(fs).length, 'exports')"
```

### Cross-platform integration test

A separate job installs each platform's wheel/package and runs the full test suite (`finstack-py/tests/`, `finstack-wasm/` npm tests).

---

## Versioning & Release Flow

1. Bump version in root `Cargo.toml` workspace (`workspace.package.version`)
2. The CI workflow syncs `finstack-wasm/package.json` version from the git tag automatically
3. Tag: `git tag v0.4.1 && git push --tags`
4. CI builds all wheels + npm package, runs tests, creates GitHub Release, publishes to npm
5. Downstream consumers update their version pin

---

## Secrets & Permissions Required

| Secret | Purpose |
|--------|---------|
| `NPM_TOKEN` | npm publish token (create at npmjs.com > Access Tokens > Granular, scoped to `finstack-wasm`) |

GitHub Release publishing uses `permissions: contents: write` (no secret needed for same-repo).

npm provenance (`--provenance`) requires `permissions: id-token: write` for OIDC-based signing.

---

## Success Criteria

- [ ] `make wheel-local` produces a working wheel in `target/wheels/` in < 5 min
- [ ] `make wheel-docker` produces a manylinux wheel usable in Docker without Rust
- [ ] `make wasm-pack` produces a publishable npm package in `finstack-wasm/pkg/`
- [ ] CI workflow builds 15 wheels + 1 sdist + 1 npm tarball on tag push
- [ ] All smoke tests pass for every artifact
- [ ] GitHub Release has all Python wheels + npm tarball attached
- [ ] `finstack-wasm` is published to npm and installable via `npm install finstack-wasm`
- [ ] enterprise-finstack Dockerfile builds in < 2 min using a release wheel URL
- [ ] A developer on a fresh machine can `uv pip install <release-url>` and `import finstack`
