"""Main entry point for PIK deep-dive presentation generation."""

from __future__ import annotations

from pathlib import Path

from pptx import Presentation
from pptx.util import Inches

from .slides_closing import (
    slide_13_toggle_mechanics,
    slide_14_convergence,
    slide_15_parameters,
    slide_16_model_comparison,
)
from .slides_intro import (
    slide_01_title,
    slide_02_result_to_explain,
    slide_03_bond_setup,
    slide_04_hr_model,
    slide_05_hr_sensitivity,
    slide_06_calibration_gap,
)
from .slides_structural import (
    slide_07_merton_model,
    slide_08_barrier_calibration,
    slide_09_endogenous_hazard,
    slide_10_dynamic_recovery,
    slide_11_mc_paths,
    slide_12_feedback_spiral,
)


def main():
    prs = Presentation()
    prs.slide_width = Inches(10)
    prs.slide_height = Inches(7.5)

    slide_01_title(prs)
    slide_02_result_to_explain(prs)
    slide_03_bond_setup(prs)
    slide_04_hr_model(prs)
    slide_05_hr_sensitivity(prs)
    slide_06_calibration_gap(prs)
    slide_07_merton_model(prs)
    slide_08_barrier_calibration(prs)
    slide_09_endogenous_hazard(prs)
    slide_10_dynamic_recovery(prs)
    slide_11_mc_paths(prs)
    slide_12_feedback_spiral(prs)
    slide_13_toggle_mechanics(prs)
    slide_14_convergence(prs)
    slide_15_parameters(prs)
    slide_16_model_comparison(prs)

    out = Path(__file__).parent.parent / "pik_deep_dive.pptx"
    prs.save(str(out))
    print(f"Saved: {out}")
    print(f"Slides: {len(prs.slides)}")
