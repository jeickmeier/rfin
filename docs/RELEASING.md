# Releasing

## Prerequisites

- Push access to the repository
- `NPM_TOKEN` secret configured in GitHub repo settings (for npm publish)
  - Create at npmjs.com > Access Tokens > Granular, scoped to `finstack-wasm`
- GitHub Release publishing uses `permissions: contents: write` (no secret needed)

## Release Checklist

1. **Bump the workspace version** in the root `Cargo.toml`:

   ```toml
   [workspace.package]
   version = "0.X.Y"
   ```

2. **Commit the version bump**:

   ```bash
   git add Cargo.toml
   git commit -m "release: v0.X.Y"
   ```

3. **Tag and push**:

   ```bash
   git tag v0.X.Y
   git push && git push --tags
   ```

4. **CI handles the rest** — the `release.yml` workflow will:
   - Build 15 Python wheels (5 platforms x 3 Python versions)
   - Build a source distribution (sdist)
   - Build the `finstack-wasm` npm package (web + nodejs targets)
   - Create a GitHub Release with all artifacts attached
   - Publish `finstack-wasm` to npm

5. **Verify** the GitHub Release page has all artifacts and npm package is installable.

## What Gets Built

### Python Wheels

| Platform | Architecture | Tag |
|----------|-------------|-----|
| Linux | x86_64 | `manylinux_2_28_x86_64` |
| Linux | aarch64 | `manylinux_2_28_aarch64` |
| macOS | arm64 | `macosx_11_0_arm64` |
| macOS | x86_64 | `macosx_10_12_x86_64` |
| Windows | x86_64 | `win_amd64` |

Each platform builds for CPython 3.12, 3.13, and 3.14 (15 wheels total).

All wheels include features: `scenarios,sqlite,postgres`.

### npm Package

- `finstack-wasm` with web (ESM) and nodejs (CJS) targets
- Package version is synced from the git tag automatically

## Installing Pre-built Wheels

### From GitHub Release URL

```bash
uv pip install "https://github.com/rustfin/rfin/releases/download/v0.X.Y/finstack-0.X.Y-cp314-cp314-manylinux_2_28_aarch64.whl"
```

### In pyproject.toml (uv sources)

```toml
[project]
dependencies = ["finstack"]

[tool.uv.sources]
finstack = { url = "https://github.com/rustfin/rfin/releases/download/v0.X.Y/finstack-0.X.Y-cp314-cp314-manylinux_2_28_aarch64.whl" }
```

### npm Package

```bash
npm install finstack-wasm
```

## Local Build Targets

```bash
make wheel-local         # Build wheel for current platform + Python
make wheel-docker        # Build manylinux wheel via Docker
make wheel-all           # Build wheels for all local Python versions
make wasm-pkg            # Build WASM package (web + node targets)
make wasm-publish-dry    # Dry-run npm publish
```

## Manual Dispatch

The workflow can be triggered manually from the GitHub Actions UI with `workflow_dispatch`.
Set `publish: true` to publish artifacts, or leave it `false` for a dry-run that only builds.
