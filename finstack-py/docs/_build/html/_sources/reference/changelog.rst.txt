Changelog
=========

This page links to the main project changelog.

For the full changelog, see: `CHANGELOG.md <https://github.com/your-org/finstack/blob/main/CHANGELOG.md>`_

Recent Releases
---------------

Version 0.1.0 (TBD)
~~~~~~~~~~~~~~~~~~~

**Initial Beta Release**

**Features**:

* 40+ financial instruments (bonds, swaps, options, credit, structured products)
* Analytical and Monte Carlo pricing models
* Risk metrics (DV01, CS01, Greeks)
* Curve calibration from market quotes
* Scenario analysis with DSL and builder API
* Portfolio aggregation and optimization
* Statement modeling with forecasts and extensions
* Polars/Pandas DataFrame integration

**Python API**:

* Full parity with Rust core (100% coverage)
* NumPy-style docstrings
* Type hints for all public APIs
* Comprehensive examples and tutorials

**Documentation**:

* Beginner, intermediate, and advanced tutorials
* 20+ cookbook examples
* Complete API reference
* Glossary and contributing guide

**Performance**:

* Rust core (10-100x faster than pure Python)
* GIL release for parallelism
* Batch pricing support

**Testing**:

* 300+ unit tests
* Parity tests (Python ≡ Rust)
* Property tests (Hypothesis)
* Benchmarks (pytest-benchmark)

**Known Limitations**:

* Bucketed risk metrics require Rust core changes (see Task 1.5)
* Some advanced Monte Carlo features not yet exposed
* WASM bindings are separate package

Migration Guide
---------------

(For future releases with breaking changes)

Upgrading from 0.0.x to 0.1.0
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

No breaking changes (initial release).

Versioning Policy
-----------------

finstack follows `Semantic Versioning 2.0.0 <https://semver.org/>`_:

* **Major** (1.0.0): Breaking API changes
* **Minor** (0.1.0): New features, backward-compatible
* **Patch** (0.0.1): Bug fixes, backward-compatible

Breaking changes will be:

* Announced in CHANGELOG.md
* Deprecated for at least one minor version before removal
* Accompanied by migration guide

See Also
--------

* `Full CHANGELOG <https://github.com/your-org/finstack/blob/main/CHANGELOG.md>`_
* :doc:`contributing`
* `GitHub Releases <https://github.com/your-org/finstack/releases>`_
