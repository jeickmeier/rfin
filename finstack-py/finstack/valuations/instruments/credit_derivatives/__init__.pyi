"""Credit derivatives instrument wrappers."""

from __future__ import annotations
from .cds import CreditDefaultSwap as CreditDefaultSwap, CDSPayReceive as CDSPayReceive
from .cds_index import CDSIndex as CdsIndex
from .cds_option import CdsOption as CdsOption
from .cds_tranche import CdsTranche as CdsTranche

__all__ = [
    "CreditDefaultSwap",
    "CDSPayReceive",
    "CdsIndex",
    "CdsOption",
    "CdsTranche",
]
