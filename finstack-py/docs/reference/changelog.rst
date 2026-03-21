Changelog
=========

This page links to the main project changelog.

For the full changelog, see: `CHANGELOG.md <https://github.com/your-org/finstack/blob/main/CHANGELOG.md>`_

Recent Releases
---------------

Version 0.5.0 (Simplicity Rollout)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

This release is a simplicity-focused refactor. All changes are **backward-compatible**:
deprecated APIs remain functional and emit compiler warnings. They will be removed in a
future major version.

**Deprecated APIs and Migration Guide**:

.. rubric:: 1. Thin ``GenericInstrumentPricer`` type aliases

The following type aliases are deprecated. They are thin wrappers around
``GenericInstrumentPricer<T>`` and add no value.

Before::

    use finstack_valuations::instruments::rates::deposit::SimpleDepositDiscountingPricer;
    // ...similar aliases for FRA, IRFuture, FxSwap, BasisSwap, etc.

After::

    use finstack_valuations::instruments::common::pricing::GenericInstrumentPricer;
    use finstack_valuations::pricer::InstrumentType;

    let pricer = GenericInstrumentPricer::<Deposit>::discounting(InstrumentType::Deposit);

.. rubric:: 2. ``create_*_registry()`` factory functions

The following functions are deprecated in favour of
``PricerRegistry::new() + register_*_pricers()``:

- ``create_rates_registry()``
- ``create_credit_registry()``
- ``create_equity_registry()``
- ``create_fx_registry()``

Before::

    use finstack_valuations::pricer::{create_rates_registry, standard_registry};

    let registry = create_rates_registry();

After::

    use finstack_valuations::pricer::{PricerRegistry, register_rates_pricers};

    let mut registry = PricerRegistry::new();
    register_rates_pricers(&mut registry);
    // or simply:
    let registry = standard_registry(); // all asset classes

.. rubric:: 3. ``BondFuture`` high-arity convenience constructors

The following constructors are deprecated. They take 10-11 positional arguments
and are replaced by the fluent builder with explicit ``contract_specs``:

- ``BondFuture::ust_10y(...)``
- ``BondFuture::ust_10y_with_ctd_bond(...)``
- ``BondFuture::ust_5y(...)``
- ``BondFuture::ust_2y(...)``
- ``BondFuture::bund(...)``
- ``BondFuture::gilt(...)``

Before::

    let future = BondFuture::ust_10y(
        id, notional, expiry, delivery_start, delivery_end,
        quoted_price, position, basket, ctd_id, curve_id,
    )?;

After::

    use finstack_valuations::instruments::fixed_income::bond_future::{
        BondFuture, BondFutureSpecs,
    };

    let future = BondFuture::builder()
        .id(id)
        .notional(notional)
        .expiry(expiry)
        .delivery_start(delivery_start)
        .delivery_end(delivery_end)
        .quoted_price(quoted_price)
        .position(position)
        .contract_specs(BondFutureSpecs::ust_10y())   // ← explicit specs
        .deliverable_basket(basket)
        .ctd_bond_id(ctd_id)
        .discount_curve_id(curve_id)
        .attributes(Attributes::new())
        .build_validated()?;

For other contract types, substitute the appropriate ``BondFutureSpecs`` preset:
``BondFutureSpecs::ust_5y()``, ``BondFutureSpecs::ust_2y()``,
``BondFutureSpecs::bund()``, or ``BondFutureSpecs::gilt()``.

.. rubric:: 4. ``instruments::legs`` and ``instruments::market`` compat modules

The backward-compat re-export modules are deprecated. Use the canonical
``instruments::common::parameters`` path instead:

Before::

    use finstack_valuations::instruments::legs::PayReceive;
    use finstack_valuations::instruments::market::ExerciseStyle;

After::

    use finstack_valuations::instruments::common::parameters::PayReceive;
    use finstack_valuations::instruments::common::parameters::ExerciseStyle;

    // Or via the top-level re-export:
    use finstack_valuations::instruments::PayReceive;
    use finstack_valuations::instruments::ExerciseStyle;

**Deprecation Timeline**:

- ``0.5.0``: Deprecation warnings added; all deprecated APIs still work.
- ``0.6.0``: (Planned) Remove deprecated APIs. Update any call sites flagged by ``rustc``.

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

Upgrading from 0.4.x to 0.5.0
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

All deprecated APIs in 0.5.0 have migration paths documented above. To find
all call sites that need updating, run::

    cargo check 2>&1 | grep "use of deprecated"

Each warning includes the recommended replacement in its note.

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
