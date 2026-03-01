"""FX instrument wrappers."""

from __future__ import annotations
from .fx import (
    FxSpot as FxSpot,
    FxSpotBuilder as FxSpotBuilder,
    FxOption as FxOption,
    FxOptionBuilder as FxOptionBuilder,
    FxSwap as FxSwap,
    FxSwapBuilder as FxSwapBuilder,
)
from .fx_barrier_option import FxBarrierOption as FxBarrierOption, FxBarrierOptionBuilder as FxBarrierOptionBuilder
from .fx_digital_option import FxDigitalOption as FxDigitalOption, FxDigitalOptionBuilder as FxDigitalOptionBuilder
from .fx_forward import (
    FxForward as FxForward,
    FxForwardBuilder as FxForwardBuilder,
)
from .fx_touch_option import FxTouchOption as FxTouchOption, FxTouchOptionBuilder as FxTouchOptionBuilder
from .fx_variance_swap import (
    FxVarianceSwap as FxVarianceSwap,
    FxVarianceSwapBuilder as FxVarianceSwapBuilder,
    FxVarianceDirection as FxVarianceDirection,
    FxRealizedVarianceMethod as FxRealizedVarianceMethod,
)
from .ndf import Ndf as Ndf, NdfBuilder as NdfBuilder
from .quanto_option import QuantoOption as QuantoOption, QuantoOptionBuilder as QuantoOptionBuilder

__all__ = [
    "FxSpot",
    "FxSpotBuilder",
    "FxOption",
    "FxOptionBuilder",
    "FxSwap",
    "FxSwapBuilder",
    "FxBarrierOption",
    "FxBarrierOptionBuilder",
    "FxDigitalOption",
    "FxDigitalOptionBuilder",
    "FxForward",
    "FxForwardBuilder",
    "FxTouchOption",
    "FxTouchOptionBuilder",
    "FxVarianceSwap",
    "FxVarianceSwapBuilder",
    "FxVarianceDirection",
    "FxRealizedVarianceMethod",
    "Ndf",
    "NdfBuilder",
    "QuantoOption",
    "QuantoOptionBuilder",
]
