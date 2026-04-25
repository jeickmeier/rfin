"""CDS-family instrument bindings."""

from __future__ import annotations

from finstack.finstack import valuations as _valuations

CreditDefaultSwap = _valuations.credit_derivatives.CreditDefaultSwap
CDSIndex = _valuations.credit_derivatives.CDSIndex
CDSTranche = _valuations.credit_derivatives.CDSTranche
CDSOption = _valuations.credit_derivatives.CDSOption

__all__ = ["CDSIndex", "CDSOption", "CDSTranche", "CreditDefaultSwap"]
