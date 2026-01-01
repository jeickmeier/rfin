Installation
============

finstack-py requires Python 3.12 or later and is available via pip and from source.

Prerequisites
-------------

* Python >= 3.12
* pip >= 23.0 (recommended)
* For building from source: Rust >= 1.90 (MSRV)

Install from PyPI
-----------------

.. code-block:: bash

   pip install finstack

This will install pre-built wheels for most platforms (Linux, macOS, Windows).

Install from Source
-------------------

If you need to build from source (e.g., for development or unsupported platforms):

.. code-block:: bash

   # Clone the repository
   git clone https://github.com/your-org/finstack.git
   cd finstack

   # Install Rust (if not already installed)
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Build and install
   pip install maturin
   maturin develop --release

Development Installation
------------------------

For contributors and developers:

.. code-block:: bash

   # Clone the repository
   git clone https://github.com/your-org/finstack.git
   cd finstack

   # Install with development dependencies
   pip install -e '.[dev]'

   # Or use uv for faster dependency resolution
   pip install uv
   uv pip install -e '.[dev]'

Verify Installation
-------------------

Check that finstack is correctly installed:

.. code-block:: python

   import finstack
   print(finstack.__version__)

   # Test basic functionality
   from finstack import Currency, Money
   usd = Currency.from_code("USD")
   amount = Money.from_code(100.0, "USD")
   print(f"{amount.amount} {amount.currency.code}")  # 100.0 USD

Troubleshooting
---------------

macOS: Library Loading Issues
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

If you encounter library loading errors on macOS:

.. code-block:: bash

   export DYLD_FALLBACK_LIBRARY_PATH=/usr/local/lib:$DYLD_FALLBACK_LIBRARY_PATH

Linux: Missing GLIBC
~~~~~~~~~~~~~~~~~~~~

If you see GLIBC version errors:

.. code-block:: bash

   # Upgrade to a newer Linux distribution, or build from source
   pip install --no-binary :all: finstack

Windows: Visual C++ Redistributable
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Install Visual C++ Redistributable if you encounter DLL errors:

https://aka.ms/vs/17/release/vc_redist.x64.exe

Build from Source Issues
~~~~~~~~~~~~~~~~~~~~~~~~~

Ensure you have:

* Latest stable Rust: ``rustup update stable``
* Latest maturin: ``pip install --upgrade maturin``
* All build dependencies: ``apt-get install build-essential`` (Linux)

Getting Help
------------

If you encounter issues:

* Check the `GitHub Issues <https://github.com/your-org/finstack/issues>`_
* Ask on discussions: https://github.com/your-org/finstack/discussions
* Email: support@finstack.dev
