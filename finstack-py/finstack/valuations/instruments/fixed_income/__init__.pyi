"""Fixed income instrument wrappers."""

from __future__ import annotations
from .agency_mbs import (
    AgencyCmo as AgencyCmo,
    AgencyMbsPassthrough as AgencyMbsPassthrough,
    AgencyProgram as AgencyProgram,
    AgencyTba as AgencyTba,
    DollarRoll as DollarRoll,
    PoolType as PoolType,
    TbaTerm as TbaTerm,
)
from .bond import Bond as Bond, BondBuilder as BondBuilder
from .bond_future import (
    BondFuture as BondFuture,
    BondFutureBuilder as BondFutureBuilder,
    BondFutureSpecs as BondFutureSpecs,
)
from .convertible import ConvertibleBond as ConvertibleBond
from .inflation_linked_bond import (
    InflationLinkedBond as InflationLinkedBond,
    InflationLinkedBondBuilder as InflationLinkedBondBuilder,
)
from .revolving_credit import (
    RevolvingCredit as RevolvingCredit,
    EnhancedMonteCarloResult as EnhancedMonteCarloResult,
    PathResult as PathResult,
    ThreeFactorPathData as ThreeFactorPathData,
)
from .structured_credit import StructuredCredit as StructuredCredit
from .term_loan import TermLoan as TermLoan

__all__ = [
    "AgencyCmo",
    "AgencyMbsPassthrough",
    "AgencyProgram",
    "AgencyTba",
    "DollarRoll",
    "PoolType",
    "TbaTerm",
    "Bond",
    "BondBuilder",
    "BondFuture",
    "BondFutureBuilder",
    "BondFutureSpecs",
    "ConvertibleBond",
    "InflationLinkedBond",
    "InflationLinkedBondBuilder",
    "RevolvingCredit",
    "EnhancedMonteCarloResult",
    "PathResult",
    "ThreeFactorPathData",
    "StructuredCredit",
    "TermLoan",
]
