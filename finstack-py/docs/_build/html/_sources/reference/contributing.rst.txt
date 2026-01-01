Contributing Guide
==================

We welcome contributions to finstack-py!

Getting Started
---------------

1. **Fork the repository** on GitHub
2. **Clone your fork**:

   .. code-block:: bash

      git clone https://github.com/your-username/finstack.git
      cd finstack

3. **Install development dependencies**:

   .. code-block:: bash

      # Using pip
      pip install -e '.[dev]'

      # Or using uv (faster)
      pip install uv
      uv pip install -e '.[dev]'

4. **Build the Python bindings**:

   .. code-block:: bash

      maturin develop --release

Development Workflow
--------------------

1. Create a Feature Branch
~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: bash

   git checkout -b feature/my-new-feature

2. Make Changes
~~~~~~~~~~~~~~~

* **Rust code**: ``finstack-py/src/``
* **Python code**: ``finstack-py/python/finstack/``
* **Tests**: ``finstack-py/tests/``
* **Examples**: ``finstack-py/examples/``
* **Docs**: ``finstack-py/docs/``

3. Write Tests
~~~~~~~~~~~~~~

All new features must include tests:

.. code-block:: bash

   # Run tests
   pytest finstack-py/tests/

   # Run specific test file
   pytest finstack-py/tests/test_bond.py -v

   # Run with coverage
   pytest --cov=finstack --cov-report=html

4. Lint and Format
~~~~~~~~~~~~~~~~~~

.. code-block:: bash

   # Rust linting
   make lint-rust

   # Python linting
   make lint-python

   # Auto-fix (where possible)
   make lint-rust-fix
   ruff check --fix .

5. Build Documentation
~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: bash

   cd finstack-py/docs
   make html

   # View locally
   open _build/html/index.html

6. Commit and Push
~~~~~~~~~~~~~~~~~~

.. code-block:: bash

   git add .
   git commit -m "Add feature: description"
   git push origin feature/my-new-feature

7. Open a Pull Request
~~~~~~~~~~~~~~~~~~~~~~

* Go to GitHub and open a PR against ``main``
* Fill out the PR template
* Wait for CI checks to pass
* Address review comments

Code Standards
--------------

Rust Code
~~~~~~~~~

* Follow `.cursor/rules/rust/` standards
* Use ``#![deny(unsafe_code)]`` (no unsafe)
* All public APIs must have doc comments
* No implicit cross-currency math
* Stable serde field names

Python Code
~~~~~~~~~~~

* Follow PEP 8 and Google-style docstrings
* Type hints for all public APIs
* NumPy-style docstrings with Examples sections
* Import from public API surface (not internal paths)

Example:

.. code-block:: python

   """Bond pricing utilities.

   This module provides functions for pricing fixed-income securities.
   """

   from typing import Optional
   from finstack import Bond, Money, Date

   def price_bond(
       bond: Bond,
       market: MarketContext,
       config: Optional[FinstackConfig] = None
   ) -> Money:
       """Price a bond using the discounting model.

       Parameters
       ----------
       bond : Bond
           The bond to price.
       market : MarketContext
           Market data (discount curve).
       config : FinstackConfig, optional
           Configuration for rounding, by default None.

       Returns
       -------
       Money
           Present value of the bond.

       Examples
       --------
       >>> bond = Bond.fixed_semiannual(...)
       >>> pv = price_bond(bond, market)
       >>> print(f"PV: {pv.amount:,.2f}")
       PV: 1,024,567.89
       """
       registry = PricerRegistry.create_standard()
       result = registry.price_bond(bond, "discounting", market)
       return result.present_value

Testing Standards
-----------------

Unit Tests
~~~~~~~~~~

* Test file: ``test_<module>.py``
* Test function: ``test_<feature>_<scenario>()``
* Use pytest fixtures for reusable setup
* Avoid external dependencies (mock where needed)

Integration Tests
~~~~~~~~~~~~~~~~~

* Test multi-component workflows
* Use realistic market data
* Verify end-to-end behavior

Parity Tests
~~~~~~~~~~~~

* Verify Python ≡ Rust for all public APIs
* Tolerance: deterministic operations < 1e-10
* See ``finstack-py/tests/parity/``

Property Tests
~~~~~~~~~~~~~~

* Use Hypothesis for property-based testing
* Test invariants (e.g., currency safety, monotonicity)
* See ``finstack-py/tests/properties/``

Documentation Standards
-----------------------

All Public APIs
~~~~~~~~~~~~~~~

* **Doc comments** in Rust source (``///`` or ``/**``)
* **Examples section** showing typical usage
* **Parameters and returns** clearly documented

Tutorials
~~~~~~~~~

* Step-by-step with executable code
* Explain the "why" not just the "how"
* Include practice exercises

Examples
~~~~~~~~

* Runnable scripts in ``examples/``
* Realistic market data
* Error handling demonstrated
* Output formatting shown

Pull Request Checklist
-----------------------

Before submitting a PR, ensure:

- [ ] Tests pass: ``pytest finstack-py/tests/``
- [ ] Linting passes: ``make lint-rust && make lint-python``
- [ ] Documentation added for new features
- [ ] Examples added for new workflows
- [ ] CHANGELOG.md updated (if user-facing change)
- [ ] PR description explains what and why

CI/CD Checks
------------

Our GitHub Actions CI runs:

* Rust compilation (debug and release)
* Rust tests (unit, integration, doc tests)
* Rust linting (clippy)
* Python tests (pytest)
* Python linting (ruff)
* Documentation build (Sphinx)
* Benchmark regression (pytest-benchmark)

All checks must pass before merge.

Release Process
---------------

(For maintainers only)

1. Update version in ``Cargo.toml``
2. Update ``CHANGELOG.md``
3. Tag release: ``git tag v1.0.0``
4. Push tag: ``git push origin v1.0.0``
5. CI builds wheels and publishes to PyPI

Getting Help
------------

* **Questions**: GitHub Discussions
* **Bugs**: GitHub Issues
* **Security**: security@finstack.dev (private)
* **Chat**: Discord (link in README)

Code of Conduct
---------------

We follow the Contributor Covenant Code of Conduct.
Be respectful, inclusive, and constructive.

License
-------

By contributing, you agree to license your contributions under:

* Apache License 2.0 OR
* MIT License

(dual-licensed, same as finstack)
