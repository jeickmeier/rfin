"""FX instrument wrappers."""

from __future__ import annotations
from .fx import FxSpot as FxSpot, FxOption as FxOption, FxSwap as FxSwap
from .fx_barrier_option import FxBarrierOption as FxBarrierOption
from .fx_digital_option import FxDigitalOption as FxDigitalOption
from .fx_forward import (
    FxForward as FxForward,
    FxForwardBuilder as FxForwardBuilder,
)
from .fx_touch_option import FxTouchOption as FxTouchOption
from .fx_variance_swap import (
    FxVarianceSwap as FxVarianceSwap,
    FxVarianceSwapBuilder as FxVarianceSwapBuilder,
    FxVarianceDirection as FxVarianceDirection,
    FxRealizedVarianceMethod as FxRealizedVarianceMethod,
)
from .ndf import Ndf as Ndf, NdfBuilder as NdfBuilder
from .quanto_option import QuantoOption as QuantoOption

__all__ = [
    "FxSpot",
    "FxOption",
    "FxSwap",
    "FxBarrierOption",
    "FxDigitalOption",
    "FxForward",
    "FxForwardBuilder",
    "FxTouchOption",
    "FxVarianceSwap",
    "FxVarianceSwapBuilder",
    "FxVarianceDirection",
    "FxRealizedVarianceMethod",
    "Ndf",
    "NdfBuilder",
    "QuantoOption",
]
