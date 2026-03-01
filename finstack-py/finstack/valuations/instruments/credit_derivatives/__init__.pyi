"""Credit derivatives instrument wrappers."""

from __future__ import annotations
from .cds import CreditDefaultSwap as CreditDefaultSwap, CDSPayReceive as CDSPayReceive, CDSConvention as CDSConvention
from .cds_index import (
    CDSIndex as CDSIndex,
    CDSIndexBuilder as CDSIndexBuilder,
    CDSIndexConstituent as CDSIndexConstituent,
)
from .cds_option import CDSOption as CDSOption, CDSOptionBuilder as CDSOptionBuilder
from .cds_tranche import CDSTranche as CDSTranche, CDSTrancheBuilder as CDSTrancheBuilder, TrancheSide as TrancheSide

__all__ = [
    "CreditDefaultSwap",
    "CDSPayReceive",
    "CDSConvention",
    "CDSIndex",
    "CDSIndexBuilder",
    "CDSIndexConstituent",
    "CDSOption",
    "CDSOptionBuilder",
    "CDSTranche",
    "CDSTrancheBuilder",
    "TrancheSide",
]
