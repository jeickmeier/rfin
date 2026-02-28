# AGENTS.md

## Workflows

- Build core Rust crates: `make build`
- Build core Rust crates (release, no debuginfo): `make build-prod`
- Run the full test suite: `make test`
- Run all checks exactly as CI does (wasm-build + pre-commit + test): `make ci-test`
- Lint all components (fast, core crates only): `make lint`
- Lint all components including bindings + all features (slow): `make lint-full`
- Format all codebases: `make fmt`
- Run Rust examples: `make examples`
- Run all tests and auto-fix issues: `make test-and-fix`
- Python dev setup (uv + maturin): `make python-dev`
- Initialize Python env with uv: `make setup-python`
- Run Python examples (scripts + notebooks): `make examples-python`
- Build Python wheel (local platform): `make wheel-local`
- Build manylinux wheel via Docker: `make wheel-docker`
- Build wheels for all local Python versions: `make wheel-all`
- Build WASM npm package (web + node): `make wasm-pkg`
- Dry-run npm publish: `make wasm-publish-dry`
- WASM build (dev): `make wasm-build`
- WASM examples dev server: `make wasm-examples-dev`
- Generate TypeScript bindings: `make generate-bindings`
- Generate rustdoc: `make doc`
- Serve mdBook docs: `make book-serve`
- Run coverage for all components: `make coverage`
- Generate API parity report: `make list`
- Analyze binary sizes: `make size-all`
- Security audits across components: `make audit`
- Update Rust/Python dependencies: `make update`
- Verify local toolchain setup: `make check-env`
- Run pre-commit hooks: `make pre-commit-run`

## Component Commands

- Rust tests (nextest): `make test-rust`
- Rust tests (slow): `make test-rust-slow`
- Rust doctests: `make test-rust-doc`
- Python tests: `make test-python`
- Python typecheck: `make typecheck-python`
- Python stub verification (CI-grade): `make verifytypes-python`
- WASM tests: `make test-wasm`
- Rust formatting: `make fmt-rust`
- Python formatting: `make fmt-python`
- WASM formatting: `make fmt-wasm`
- Rust lints (fast, core only): `make lint-rust`
- Rust lints (full workspace + all features): `make lint-rust-full`
- Python lints: `make lint-python`
- WASM lints: `make lint-wasm`

## Release

- Release workflow: `.github/workflows/release.yml` (triggers on `v*` tag push or manual dispatch)
- Tag a release: bump version in root `Cargo.toml`, then `git tag v0.X.Y && git push --tags`
- CI builds 15 Python wheels (5 platforms x 3 Python versions) + sdist + npm tarball
- Python wheels: attached to GitHub Release; features `scenarios,sqlite,postgres`
- npm package: `finstack-wasm` published to npm registry (requires `NPM_TOKEN` secret)
- Wheel features: `scenarios,sqlite,postgres` (no `turso` in distribution)
- See `docs/RELEASING.md` for full release checklist
