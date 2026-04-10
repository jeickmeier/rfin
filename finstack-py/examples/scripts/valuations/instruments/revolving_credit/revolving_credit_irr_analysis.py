"""Backwards-compatible wrapper — delegates to revolving_credit_irr package."""

from revolving_credit_irr.main import main

if __name__ == "__main__":
    raise SystemExit(main())
