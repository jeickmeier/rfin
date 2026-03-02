#!/usr/bin/env python3
"""Generate a PowerPoint deck on PIK coupon breakeven modelling.

Audience: credit team familiar with coupon types but low on modelling knowledge.
Approach: "Building Blocks" — start simple, add complexity, show results.

Usage:
    python generate_pik_presentation.py
    # => writes pik_coupon_pricing.pptx
"""

from __future__ import annotations

import math
from pathlib import Path

from pptx import Presentation
from pptx.util import Inches, Pt, Emu
from pptx.dml.color import RGBColor
from pptx.enum.text import PP_ALIGN, MSO_ANCHOR
from pptx.enum.shapes import MSO_SHAPE

# ── Colour palette ────────────────────────────────────────────────────────

DARK_BG = RGBColor(0x1A, 0x1A, 0x2E)      # deep navy for title slides
MID_BG = RGBColor(0x16, 0x21, 0x3E)        # section headers
WHITE = RGBColor(0xFF, 0xFF, 0xFF)
LIGHT_GREY = RGBColor(0xCC, 0xCC, 0xCC)
ACCENT_BLUE = RGBColor(0x4E, 0xC9, 0xB0)   # teal accent
ACCENT_ORANGE = RGBColor(0xE8, 0x8D, 0x3F)  # orange for warnings
ACCENT_RED = RGBColor(0xE0, 0x4F, 0x4F)     # red for danger
ACCENT_GREEN = RGBColor(0x5C, 0xB8, 0x5C)   # green for safe
BODY_BG = RGBColor(0xF5, 0xF5, 0xFA)       # light content background
TEXT_DARK = RGBColor(0x2D, 0x2D, 0x3D)      # body text
TEXT_MID = RGBColor(0x55, 0x55, 0x70)       # secondary text
HEADER_BLUE = RGBColor(0x2C, 0x3E, 0x6B)   # section header text
TABLE_HEADER_BG = RGBColor(0x2C, 0x3E, 0x6B)
TABLE_ALT_BG = RGBColor(0xE8, 0xEB, 0xF5)
TABLE_WHITE_BG = RGBColor(0xFF, 0xFF, 0xFF)
HIGHLIGHT_YELLOW = RGBColor(0xFF, 0xD7, 0x00)


# ── Helpers ───────────────────────────────────────────────────────────────

def set_slide_bg(slide, color: RGBColor):
    bg = slide.background
    fill = bg.fill
    fill.solid()
    fill.fore_color.rgb = color


def add_textbox(slide, left, top, width, height, text: str, *,
                font_size=18, bold=False, color=TEXT_DARK,
                alignment=PP_ALIGN.LEFT, font_name="Calibri"):
    txBox = slide.shapes.add_textbox(left, top, width, height)
    tf = txBox.text_frame
    tf.word_wrap = True
    p = tf.paragraphs[0]
    p.text = text
    p.font.size = Pt(font_size)
    p.font.bold = bold
    p.font.color.rgb = color
    p.font.name = font_name
    p.alignment = alignment
    return txBox


def add_bullet_list(slide, left, top, width, height, items: list[str], *,
                    font_size=16, color=TEXT_DARK, bullet_color=ACCENT_BLUE,
                    spacing_after=Pt(8)):
    txBox = slide.shapes.add_textbox(left, top, width, height)
    tf = txBox.text_frame
    tf.word_wrap = True
    for i, item in enumerate(items):
        if i == 0:
            p = tf.paragraphs[0]
        else:
            p = tf.add_paragraph()
        p.text = item
        p.font.size = Pt(font_size)
        p.font.color.rgb = color
        p.font.name = "Calibri"
        p.level = 0
        p.space_after = spacing_after
    return txBox


def add_callout_box(slide, left, top, width, height, text: str, *,
                    bg_color=ACCENT_BLUE, text_color=WHITE, font_size=14):
    shape = slide.shapes.add_shape(
        MSO_SHAPE.ROUNDED_RECTANGLE, left, top, width, height)
    shape.fill.solid()
    shape.fill.fore_color.rgb = bg_color
    shape.line.fill.background()
    tf = shape.text_frame
    tf.word_wrap = True
    tf.margin_left = Pt(12)
    tf.margin_right = Pt(12)
    tf.margin_top = Pt(8)
    tf.margin_bottom = Pt(8)
    p = tf.paragraphs[0]
    p.text = text
    p.font.size = Pt(font_size)
    p.font.color.rgb = text_color
    p.font.name = "Calibri"
    p.font.bold = True
    p.alignment = PP_ALIGN.CENTER
    return shape


def add_section_number(slide, number: str, left=Inches(0.5), top=Inches(0.3)):
    shape = slide.shapes.add_shape(
        MSO_SHAPE.OVAL, left, top, Inches(0.5), Inches(0.5))
    shape.fill.solid()
    shape.fill.fore_color.rgb = ACCENT_BLUE
    shape.line.fill.background()
    tf = shape.text_frame
    tf.margin_left = Pt(0)
    tf.margin_right = Pt(0)
    tf.margin_top = Pt(0)
    tf.margin_bottom = Pt(0)
    p = tf.paragraphs[0]
    p.text = number
    p.font.size = Pt(16)
    p.font.bold = True
    p.font.color.rgb = WHITE
    p.font.name = "Calibri"
    p.alignment = PP_ALIGN.CENTER
    tf.vertical_anchor = MSO_ANCHOR.MIDDLE


def add_table(slide, left, top, width, rows_data: list[list[str]], *,
              col_widths=None, font_size=11, header_row=True):
    n_rows = len(rows_data)
    n_cols = len(rows_data[0])
    table_shape = slide.shapes.add_table(n_rows, n_cols, left, top, width, Inches(0.35 * n_rows))
    table = table_shape.table

    if col_widths:
        for i, w in enumerate(col_widths):
            table.columns[i].width = w

    for r, row in enumerate(rows_data):
        for c, cell_text in enumerate(row):
            cell = table.cell(r, c)
            cell.text = cell_text
            for paragraph in cell.text_frame.paragraphs:
                paragraph.font.size = Pt(font_size)
                paragraph.font.name = "Calibri"
                if r == 0 and header_row:
                    paragraph.font.bold = True
                    paragraph.font.color.rgb = WHITE
                    paragraph.alignment = PP_ALIGN.CENTER
                else:
                    paragraph.font.color.rgb = TEXT_DARK
                    if c == 0:
                        paragraph.alignment = PP_ALIGN.LEFT
                    else:
                        paragraph.alignment = PP_ALIGN.RIGHT

            if r == 0 and header_row:
                cell.fill.solid()
                cell.fill.fore_color.rgb = TABLE_HEADER_BG
            elif r % 2 == 0:
                cell.fill.solid()
                cell.fill.fore_color.rgb = TABLE_ALT_BG
            else:
                cell.fill.solid()
                cell.fill.fore_color.rgb = TABLE_WHITE_BG

    return table_shape


def add_slide_number(slide, num: int, total: int):
    add_textbox(slide, Inches(8.8), Inches(7.1), Inches(1.2), Inches(0.3),
                f"{num}/{total}", font_size=10, color=TEXT_MID,
                alignment=PP_ALIGN.RIGHT)


def add_slide_title(slide, title: str, subtitle: str = ""):
    add_textbox(slide, Inches(1.1), Inches(0.2), Inches(8), Inches(0.5),
                title, font_size=26, bold=True, color=HEADER_BLUE)
    if subtitle:
        add_textbox(slide, Inches(1.1), Inches(0.65), Inches(8), Inches(0.4),
                    subtitle, font_size=14, color=TEXT_MID)


# ── Slide builders ────────────────────────────────────────────────────────

def slide_01_title(prs):
    """Title slide."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])  # blank
    set_slide_bg(slide, DARK_BG)
    add_textbox(slide, Inches(1), Inches(2), Inches(8), Inches(1.2),
                "PIK Coupon Pricing", font_size=40, bold=True, color=WHITE)
    add_textbox(slide, Inches(1), Inches(3.1), Inches(8), Inches(0.8),
                "How Much Extra Spread Is Enough?", font_size=24, color=ACCENT_BLUE)
    add_textbox(slide, Inches(1), Inches(4.2), Inches(8), Inches(0.6),
                "Modelling breakeven spreads for Cash, PIK, and Toggle coupons\nacross issuer credit quality",
                font_size=16, color=LIGHT_GREY)
    # thin accent line
    shape = slide.shapes.add_shape(
        MSO_SHAPE.RECTANGLE, Inches(1), Inches(3.95), Inches(3), Pt(3))
    shape.fill.solid()
    shape.fill.fore_color.rgb = ACCENT_BLUE
    shape.line.fill.background()


def slide_02_the_question(prs):
    """The core question."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "1")
    add_slide_title(slide, "The Question")
    add_slide_number(slide, 2, 18)

    add_textbox(slide, Inches(0.8), Inches(1.2), Inches(8.5), Inches(0.7),
                "At what spread should a PIK bond trade versus a\ncash-pay bond for the same issuer?",
                font_size=20, bold=True, color=HEADER_BLUE, alignment=PP_ALIGN.CENTER)

    # Two columns
    # Left: naive view
    add_callout_box(slide, Inches(0.5), Inches(2.2), Inches(4.2), Inches(0.45),
                    "Naive View", bg_color=TEXT_MID)
    add_bullet_list(slide, Inches(0.5), Inches(2.8), Inches(4.2), Inches(2.5), [
        '"PIK just defers coupons — add 50bp"',
        '"The coupon rate is the same"',
        '"Compounding offsets the delay"',
    ], font_size=14, color=TEXT_DARK)

    # Right: model view
    add_callout_box(slide, Inches(5.3), Inches(2.2), Inches(4.2), Inches(0.45),
                    "What the Models Show", bg_color=ACCENT_BLUE)
    add_bullet_list(slide, Inches(5.3), Inches(2.8), Inches(4.2), Inches(2.5), [
        "For strong credits: PIK premium ≈ 0bp",
        "For stressed credits: PIK premium > 500bp",
        "The relationship is highly non-linear",
    ], font_size=14, color=TEXT_DARK)

    add_callout_box(slide, Inches(1.5), Inches(5.3), Inches(7), Inches(0.7),
                    "The answer depends on credit quality in a way that is impossible to guess without a model.",
                    bg_color=ACCENT_ORANGE, font_size=14)


def slide_03_coupon_recap(prs):
    """Coupon type recap with timeline visual."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "1")
    add_slide_title(slide, "Coupon Structures: Cash, PIK, and Toggle",
                    "What the investor receives and when")
    add_slide_number(slide, 3, 18)

    # Cash-pay timeline
    y_cash = Inches(1.5)
    add_textbox(slide, Inches(0.5), y_cash, Inches(2), Inches(0.3),
                "Cash-Pay", font_size=16, bold=True, color=ACCENT_GREEN)
    # timeline bar
    bar = slide.shapes.add_shape(MSO_SHAPE.RECTANGLE,
                                 Inches(2.5), y_cash + Pt(8), Inches(6.5), Pt(4))
    bar.fill.solid()
    bar.fill.fore_color.rgb = ACCENT_GREEN
    bar.line.fill.background()
    add_textbox(slide, Inches(2.5), y_cash + Inches(0.3), Inches(6.5), Inches(0.5),
                "4.25    4.25    4.25    4.25    4.25    4.25    4.25    4.25    4.25   104.25",
                font_size=10, color=TEXT_MID, alignment=PP_ALIGN.CENTER)
    add_textbox(slide, Inches(2.5), y_cash + Inches(0.6), Inches(6.5), Inches(0.3),
                "Regular coupon payments + par at maturity. Cash flows spread across time.",
                font_size=12, color=TEXT_DARK)

    # PIK timeline
    y_pik = Inches(3.0)
    add_textbox(slide, Inches(0.5), y_pik, Inches(2), Inches(0.3),
                "Full PIK", font_size=16, bold=True, color=ACCENT_RED)
    bar2 = slide.shapes.add_shape(MSO_SHAPE.RECTANGLE,
                                  Inches(2.5), y_pik + Pt(8), Inches(6.5), Pt(4))
    bar2.fill.solid()
    bar2.fill.fore_color.rgb = ACCENT_RED
    bar2.line.fill.background()
    add_textbox(slide, Inches(2.5), y_pik + Inches(0.3), Inches(6.5), Inches(0.5),
                "  0         0         0         0         0         0         0         0         0       151.76",
                font_size=10, color=TEXT_MID, alignment=PP_ALIGN.CENTER)
    add_textbox(slide, Inches(2.5), y_pik + Inches(0.6), Inches(6.5), Inches(0.3),
                "No coupons. Entire value at maturity: N × (1 + c/2)^10 = 151.76. Concentrated exposure.",
                font_size=12, color=TEXT_DARK)

    # Toggle timeline
    y_tog = Inches(4.5)
    add_textbox(slide, Inches(0.5), y_tog, Inches(2), Inches(0.3),
                "PIK Toggle", font_size=16, bold=True, color=ACCENT_ORANGE)
    bar3 = slide.shapes.add_shape(MSO_SHAPE.RECTANGLE,
                                  Inches(2.5), y_tog + Pt(8), Inches(6.5), Pt(4))
    bar3.fill.solid()
    bar3.fill.fore_color.rgb = ACCENT_ORANGE
    bar3.line.fill.background()
    add_textbox(slide, Inches(2.5), y_tog + Inches(0.3), Inches(6.5), Inches(0.5),
                "4.25    4.25    4.25      0       0      4.25    4.25    4.25    4.25   108.33",
                font_size=10, color=TEXT_MID, alignment=PP_ALIGN.CENTER)
    add_textbox(slide, Inches(2.5), y_tog + Inches(0.6), Inches(6.5), Inches(0.3),
                "Borrower chooses cash or PIK each period. Typically PIKs when credit deteriorates.",
                font_size=12, color=TEXT_DARK)

    add_callout_box(slide, Inches(1.5), Inches(6.0), Inches(7), Inches(0.6),
                    "Key: PIK concentrates all value at the longest maturity with a larger notional.",
                    bg_color=HEADER_BLUE, font_size=13)


def slide_04_hazard_rate(prs):
    """What's a hazard rate."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "2")
    add_slide_title(slide, "The Hazard Rate: Measuring Default Risk",
                    "The simplest credit model")
    add_slide_number(slide, 4, 18)

    add_bullet_list(slide, Inches(0.5), Inches(1.2), Inches(5), Inches(1.5), [
        "The hazard rate λ is the instantaneous rate of default",
        "Think of it as: 'each year, there is a λ chance of default'",
        'Survival probability decays over time: S(t) = e^(−λ × t)',
    ], font_size=15, color=TEXT_DARK)

    # Survival table
    add_textbox(slide, Inches(0.5), Inches(3.0), Inches(3), Inches(0.3),
                "Survival Probability S(t)", font_size=14, bold=True, color=HEADER_BLUE)

    rows = [
        ["Hazard Rate λ", "1Y", "3Y", "5Y", "7Y"],
        ["150bp (BB+)", "98.5%", "95.6%", "92.8%", "90.0%"],
        ["600bp (B)", "94.2%", "83.5%", "74.1%", "65.7%"],
        ["1400bp (CCC)", "86.9%", "65.7%", "49.7%", "37.5%"],
    ]
    add_table(slide, Inches(0.5), Inches(3.4), Inches(5.5), rows,
              font_size=12)

    # Visual annotation
    add_callout_box(slide, Inches(6.2), Inches(1.5), Inches(3.3), Inches(2.5),
                    "A CCC issuer at λ=1400bp has only a 50% chance of surviving to year 5.\n\n"
                    "Every cashflow scheduled beyond that point has ≤50% probability of being received.",
                    bg_color=HEADER_BLUE, font_size=12)

    add_textbox(slide, Inches(0.5), Inches(5.5), Inches(9), Inches(0.8),
                "The hazard rate is a single number that captures the market's view of default risk. "
                "Higher λ → faster decay of survival → lower present value of future cashflows.",
                font_size=14, color=TEXT_DARK)


def slide_05_pricing_with_hazard(prs):
    """Pricing formula with hazard rates."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "2")
    add_slide_title(slide, "Pricing Bonds with Hazard Rates",
                    "Survival-weighted present value")
    add_slide_number(slide, 5, 18)

    add_textbox(slide, Inches(0.5), Inches(1.2), Inches(9), Inches(0.5),
                "PV  =  Σ  Cashflow(t)  ×  S(t)  ×  D(t)    +    Recovery Leg",
                font_size=20, bold=True, color=HEADER_BLUE, alignment=PP_ALIGN.CENTER)

    add_textbox(slide, Inches(1), Inches(1.8), Inches(8), Inches(0.3),
                "where S(t) = survival probability, D(t) = risk-free discount factor",
                font_size=13, color=TEXT_MID, alignment=PP_ALIGN.CENTER)

    # Explanation
    add_bullet_list(slide, Inches(0.5), Inches(2.3), Inches(9), Inches(2), [
        "Each future cashflow is weighted by the probability it gets paid (survival) and time value (discount)",
        "Later cashflows are penalised more: both S(t) and D(t) are smaller at longer horizons",
        "The recovery leg adds back value for the scenario where default occurs: R × par × default_prob(t₁, t₂) × D(t)",
    ], font_size=14, color=TEXT_DARK)

    # Two-column comparison
    add_callout_box(slide, Inches(0.5), Inches(4.2), Inches(4.2), Inches(0.4),
                    "Cash-Pay Bond", bg_color=ACCENT_GREEN, font_size=13)
    add_bullet_list(slide, Inches(0.5), Inches(4.7), Inches(4.2), Inches(1.8), [
        "10 semi-annual cashflows of 4.25",
        "Plus 100 at maturity",
        "Survival exposure: spread across 10 dates",
        "Early coupons have high S(t) ≈ 1.0",
    ], font_size=12, color=TEXT_DARK, spacing_after=Pt(4))

    add_callout_box(slide, Inches(5.3), Inches(4.2), Inches(4.2), Inches(0.4),
                    "Full PIK Bond", bg_color=ACCENT_RED, font_size=13)
    add_bullet_list(slide, Inches(5.3), Inches(4.7), Inches(4.2), Inches(1.8), [
        "Zero interim cashflows",
        "Single payment of 151.76 at maturity",
        "Survival exposure: 100% at year 5",
        "Entirely dependent on S(5Y)",
    ], font_size=12, color=TEXT_DARK, spacing_after=Pt(4))

    add_callout_box(slide, Inches(1.5), Inches(6.5), Inches(7), Inches(0.5),
                    "PIK concentrates all value at the worst survival point with the largest notional.",
                    bg_color=ACCENT_ORANGE, font_size=13)


def slide_06_pik_in_hazard_model(prs):
    """PIK pricing results under flat hazard."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "2")
    add_slide_title(slide, "PIK in the Hazard Rate Model",
                    "Timing and notional effects only — no feedback")
    add_slide_number(slide, 6, 18)

    rows = [
        ["Issuer", "λ (bp)", "Cash Price", "PIK Price", "Δ Price"],
        ["BB+ (Solid HY)", "150", "113.13", "115.94", "+2.80"],
        ["BB- (Mid HY)", "350", "107.07", "108.76", "+1.69"],
        ["B (Weak HY)", "600", "99.44", "99.74", "+0.30"],
        ["B- (Stressed)", "900", "90.50", "89.21", "−1.29"],
        ["CCC (Deeply Stressed)", "1400", "77.41", "73.97", "−3.43"],
    ]
    add_table(slide, Inches(0.8), Inches(1.4), Inches(8.4), rows,
              col_widths=[Inches(2.5), Inches(1.2), Inches(1.5), Inches(1.5), Inches(1.5)],
              font_size=13)

    add_bullet_list(slide, Inches(0.5), Inches(4.2), Inches(9), Inches(1.5), [
        "At low hazard rates (BB+): PIK trades above cash — compounding offsets the survival penalty",
        "Crossover near B: the survival penalty starts to dominate the compounding benefit",
        "At high hazard rates (CCC): PIK trades 3.4 pts below cash — concentrated maturity exposure hurts",
    ], font_size=14, color=TEXT_DARK)

    add_textbox(slide, Inches(0.5), Inches(5.8), Inches(9), Inches(0.5),
                "Δ Price = PIK price minus Cash price. Negative means PIK is cheaper (investor demands more spread).",
                font_size=12, color=TEXT_MID)

    add_callout_box(slide, Inches(1.5), Inches(6.3), Inches(7), Inches(0.5),
                    "This model captures timing + notional effects. But it treats λ as fixed — it doesn't "
                    "know PIK makes the firm riskier.",
                    bg_color=HEADER_BLUE, font_size=12)


def slide_07_what_simple_misses(prs):
    """What the simple model misses."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "2")
    add_slide_title(slide, "What the Simple Model Misses",
                    "The gap between hazard-rate and structural pricing")
    add_slide_number(slide, 7, 18)

    rows = [
        ["Issuer", "HR Δ Price", "MC Δ Price", "Gap"],
        ["BB+ (Solid HY)", "+2.80", "+0.29", "2.5 pts"],
        ["BB- (Mid HY)", "+1.69", "+0.00", "1.7 pts"],
        ["B (Weak HY)", "+0.30", "−2.60", "2.9 pts"],
        ["B- (Stressed)", "−1.29", "−7.08", "5.8 pts"],
        ["CCC (Deeply Stressed)", "−3.43", "−12.32", "8.9 pts"],
    ]
    add_table(slide, Inches(1.5), Inches(1.4), Inches(7), rows,
              col_widths=[Inches(2.5), Inches(1.2), Inches(1.2), Inches(1.2)],
              font_size=13)

    add_textbox(slide, Inches(0.5), Inches(3.8), Inches(9), Inches(0.5),
                'HR = hazard rate model (fixed λ).  MC = Merton Monte Carlo (structural model with feedback).',
                font_size=12, color=TEXT_MID)

    add_bullet_list(slide, Inches(0.5), Inches(4.2), Inches(9), Inches(1.5), [
        "For CCC credits: the simple model says PIK is 3.4 pts cheaper. The full model says 12.3 pts cheaper.",
        "The 8.9 point gap is the feedback loop — PIK accrual makes the firm riskier, which the flat hazard rate ignores.",
        "The gap grows dramatically with credit risk. For BB+ it barely matters; for CCC it is the dominant effect.",
    ], font_size=14, color=TEXT_DARK)

    add_callout_box(slide, Inches(1.5), Inches(6.2), Inches(7), Inches(0.6),
                    "To price PIK accurately for stressed credits, we need a model where the hazard rate "
                    "responds to the growing PIK notional.",
                    bg_color=ACCENT_RED, font_size=13)


def slide_08_merton_model(prs):
    """Merton model: firm as an option."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "3")
    add_slide_title(slide, "The Merton Model: Default as a Balance Sheet Event",
                    "From fixed hazard rates to structural credit")
    add_slide_number(slide, 8, 18)

    add_bullet_list(slide, Inches(0.5), Inches(1.3), Inches(5.5), Inches(2.5), [
        "Firm has assets that fluctuate randomly (geometric Brownian motion)",
        "Firm has a debt barrier — if assets fall below it, the firm defaults",
        "The distance-to-default (DD) measures how far assets are from the barrier",
        "Higher asset volatility → more chance of hitting the barrier → higher default probability",
    ], font_size=14, color=TEXT_DARK)

    # Key inputs box
    add_callout_box(slide, Inches(6.5), Inches(1.3), Inches(3), Inches(0.35),
                    "Key Inputs", bg_color=TABLE_HEADER_BG, font_size=12)
    inputs = [
        ["Input", "Meaning"],
        ["Asset value (V)", "Market value of firm"],
        ["Asset vol (σ)", "How volatile is V?"],
        ["Debt barrier (B)", "Default trigger level"],
        ["Coverage (V/B)", "Asset cushion above default"],
    ]
    add_table(slide, Inches(6.5), Inches(1.75), Inches(3), inputs, font_size=10)

    # Issuer profiles
    add_textbox(slide, Inches(0.5), Inches(3.9), Inches(5), Inches(0.3),
                "Our Five Test Issuers:", font_size=14, bold=True, color=HEADER_BLUE)

    profile_rows = [
        ["Issuer", "Assets", "Vol", "Coverage", "DD", "PD(5Y)"],
        ["BB+ (Solid HY)", "200", "20%", "2.00x", "1.83", "3.4%"],
        ["BB- (Mid HY)", "165", "25%", "1.65x", "1.02", "15.4%"],
        ["B (Weak HY)", "140", "30%", "1.40x", "0.50", "30.9%"],
        ["B- (Stressed)", "125", "35%", "1.25x", "0.18", "42.7%"],
        ["CCC (Deeply Stressed)", "115", "40%", "1.15x", "−0.04", "51.5%"],
    ]
    add_table(slide, Inches(0.5), Inches(4.3), Inches(8), profile_rows,
              font_size=11)

    add_textbox(slide, Inches(0.5), Inches(6.4), Inches(9), Inches(0.5),
                "Debt barrier = 100 for all issuers (matching bond notional). Coverage = asset value / barrier.",
                font_size=12, color=TEXT_MID)


def slide_09_endogenous_hazard(prs):
    """Endogenous hazard: PIK raises leverage raises hazard."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "3")
    add_slide_title(slide, "Endogenous Hazard: PIK Raises Default Risk",
                    "Leverage-dependent hazard rate")
    add_slide_number(slide, 9, 18)

    add_textbox(slide, Inches(0.5), Inches(1.3), Inches(9), Inches(0.5),
                "λ(L) = λ₀ × (L / L₀)²",
                font_size=22, bold=True, color=HEADER_BLUE, alignment=PP_ALIGN.CENTER)
    add_textbox(slide, Inches(0.5), Inches(1.85), Inches(9), Inches(0.3),
                "Hazard rate at current leverage = base hazard × (current leverage / base leverage)²",
                font_size=13, color=TEXT_MID, alignment=PP_ALIGN.CENTER)

    add_bullet_list(slide, Inches(0.5), Inches(2.4), Inches(5), Inches(1.5), [
        "PIK accrual grows the debt (notional increases)",
        "Higher debt → higher leverage ratio L = B/V",
        "Higher leverage → higher hazard rate λ(L)",
        "The exponent (²) makes it non-linear: doubling leverage quadruples hazard",
    ], font_size=14, color=TEXT_DARK)

    # Table showing the effect
    haz_rows = [
        ["Leverage", "Hazard Rate", "vs Base"],
        ["0.60", "5.1%", "−44%"],
        ["0.70", "6.9%", "−23%"],
        ["0.80 (base)", "9.0%", "—"],
        ["0.90", "11.4%", "+27%"],
        ["1.00", "14.1%", "+56%"],
        ["1.10", "17.0%", "+89%"],
        ["1.20", "20.3%", "+125%"],
    ]
    add_table(slide, Inches(5.5), Inches(2.4), Inches(4), haz_rows, font_size=11)

    add_textbox(slide, Inches(5.5), Inches(5.3), Inches(4), Inches(0.3),
                "Example: B- issuer (base leverage 0.80)", font_size=11, color=TEXT_MID)

    add_callout_box(slide, Inches(1), Inches(5.8), Inches(8), Inches(0.6),
                    "When PIK accrual pushes leverage from 0.80 to 1.20, the hazard rate more than doubles. "
                    "This is the core mechanism that makes PIK dangerous for stressed credits.",
                    bg_color=ACCENT_ORANGE, font_size=12)


def slide_10_dynamic_recovery(prs):
    """Dynamic recovery: higher notional lowers recovery."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "3")
    add_slide_title(slide, "Dynamic Recovery: PIK Dilutes Recovery Value",
                    "More debt competing for the same assets")
    add_slide_number(slide, 10, 18)

    add_textbox(slide, Inches(0.5), Inches(1.3), Inches(9), Inches(0.5),
                "R(N) = max( floor,  R₀ × N₀ / N )",
                font_size=22, bold=True, color=HEADER_BLUE, alignment=PP_ALIGN.CENTER)
    add_textbox(slide, Inches(0.5), Inches(1.85), Inches(9), Inches(0.3),
                "Recovery at current notional = base recovery × (base notional / current notional), floored at 10%",
                font_size=13, color=TEXT_MID, alignment=PP_ALIGN.CENTER)

    add_bullet_list(slide, Inches(0.5), Inches(2.4), Inches(5), Inches(1.5), [
        "In default, creditors split the remaining assets",
        "PIK grows the total debt claim (notional rises)",
        "Same assets ÷ more debt = lower recovery per dollar",
        "Floor at 10% prevents recovery from reaching zero",
    ], font_size=14, color=TEXT_DARK)

    rec_rows = [
        ["Notional", "Recovery", "vs Base"],
        ["75", "30.0%", "—"],
        ["100 (base)", "30.0%", "—"],
        ["112.5", "26.7%", "−11%"],
        ["125", "24.0%", "−20%"],
        ["137.5", "21.8%", "−27%"],
        ["150", "20.0%", "−33%"],
    ]
    add_table(slide, Inches(5.5), Inches(2.4), Inches(4), rec_rows, font_size=11)

    add_textbox(slide, Inches(5.5), Inches(4.8), Inches(4), Inches(0.3),
                "Example: B- issuer (base recovery 30%)", font_size=11, color=TEXT_MID)

    add_callout_box(slide, Inches(1), Inches(5.5), Inches(8), Inches(0.7),
                    "Double penalty: PIK simultaneously raises the probability of default (via endogenous hazard) "
                    "AND lowers the amount recovered in default (via diluted recovery).",
                    bg_color=ACCENT_RED, font_size=12)


def slide_11_feedback_loop(prs):
    """The feedback loop — key conceptual slide."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "3")
    add_slide_title(slide, "The Feedback Loop",
                    "Why PIK risk is self-reinforcing")
    add_slide_number(slide, 11, 18)

    # Central diagram using shapes and text
    cx, cy = Inches(5), Inches(3.3)

    # Circle positions (clock positions)
    positions = [
        (Inches(3.6), Inches(1.5), "PIK Accrual"),        # top
        (Inches(6.5), Inches(2.2), "Higher\nNotional"),    # right-top
        (Inches(7.0), Inches(3.8), "Higher\nLeverage"),    # right-bottom
        (Inches(5.0), Inches(4.8), "Higher\nHazard Rate"), # bottom
        (Inches(2.5), Inches(3.8), "More Defaults\nBefore Maturity"), # left-bottom
        (Inches(1.5), Inches(2.2), "Lower\nRecovery"),     # left-top
    ]

    colors = [ACCENT_ORANGE, ACCENT_ORANGE, ACCENT_RED, ACCENT_RED, ACCENT_RED, ACCENT_RED]

    for (x, y, label), color in zip(positions, colors):
        shape = slide.shapes.add_shape(
            MSO_SHAPE.ROUNDED_RECTANGLE, x, y, Inches(1.6), Inches(0.7))
        shape.fill.solid()
        shape.fill.fore_color.rgb = color
        shape.line.fill.background()
        tf = shape.text_frame
        tf.word_wrap = True
        tf.margin_left = Pt(4)
        tf.margin_right = Pt(4)
        tf.margin_top = Pt(2)
        tf.margin_bottom = Pt(2)
        p = tf.paragraphs[0]
        p.text = label
        p.font.size = Pt(11)
        p.font.bold = True
        p.font.color.rgb = WHITE
        p.font.name = "Calibri"
        p.alignment = PP_ALIGN.CENTER
        tf.vertical_anchor = MSO_ANCHOR.MIDDLE

    # Arrows (using right-pointing arrows as text between boxes)
    arrow_positions = [
        (Inches(5.15), Inches(1.6), "→", 0),
        (Inches(7.5), Inches(2.9), "↓", 90),
        (Inches(6.4), Inches(4.7), "←", 0),
        (Inches(3.8), Inches(5.0), "←", 0),
        (Inches(2.0), Inches(3.0), "↑", 0),
        (Inches(3.0), Inches(1.5), "→", 0),
    ]
    for x, y, arrow, _ in arrow_positions:
        add_textbox(slide, x, y, Inches(0.5), Inches(0.5),
                    arrow, font_size=20, bold=True, color=HEADER_BLUE,
                    alignment=PP_ALIGN.CENTER)

    add_callout_box(slide, Inches(0.5), Inches(6.0), Inches(9), Inches(0.7),
                    "This is why the PIK premium is non-linear: it is a self-reinforcing spiral. "
                    "Each cycle amplifies the next. For stressed credits, the spiral dominates pricing.",
                    bg_color=HEADER_BLUE, font_size=13)


def slide_12_monte_carlo(prs):
    """Monte Carlo simulation overview."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "3")
    add_slide_title(slide, "Monte Carlo Simulation",
                    "Bringing the feedback loop to life")
    add_slide_number(slide, 12, 18)

    add_textbox(slide, Inches(0.5), Inches(1.2), Inches(9), Inches(0.6),
                "We simulate 25,000 paths of firm asset values over 5 years.\nAt each time step, the model updates hazard and recovery based on current leverage.",
                font_size=15, color=TEXT_DARK)

    # Steps
    steps = [
        ("1", "Simulate asset paths", "Assets follow geometric Brownian motion with calibrated volatility"),
        ("2", "Check for default", "First-passage: has the asset path hit or crossed the debt barrier?"),
        ("3", "Update PIK notional", "If PIK coupon date: accrete the notional by the coupon amount"),
        ("4", "Recalculate hazard", "λ(L) = λ₀ × (L/L₀)²  — leverage has changed due to PIK accrual"),
        ("5", "Recalculate recovery", "R(N) = R₀ × N₀/N — more debt dilutes recovery"),
        ("6", "Discount and average", "PV across all paths → price, spread, expected loss"),
    ]

    for i, (num, title, desc) in enumerate(steps):
        y = Inches(2.2) + Inches(i * 0.7)
        # number circle
        shape = slide.shapes.add_shape(MSO_SHAPE.OVAL, Inches(0.5), y, Inches(0.35), Inches(0.35))
        shape.fill.solid()
        shape.fill.fore_color.rgb = ACCENT_BLUE
        shape.line.fill.background()
        tf = shape.text_frame
        tf.margin_left = Pt(0)
        tf.margin_right = Pt(0)
        tf.margin_top = Pt(0)
        tf.margin_bottom = Pt(0)
        p = tf.paragraphs[0]
        p.text = num
        p.font.size = Pt(12)
        p.font.bold = True
        p.font.color.rgb = WHITE
        p.alignment = PP_ALIGN.CENTER
        tf.vertical_anchor = MSO_ANCHOR.MIDDLE

        add_textbox(slide, Inches(1.0), y - Pt(2), Inches(3), Inches(0.35),
                    title, font_size=14, bold=True, color=TEXT_DARK)
        add_textbox(slide, Inches(4.0), y - Pt(2), Inches(5.5), Inches(0.35),
                    desc, font_size=12, color=TEXT_MID)

    add_textbox(slide, Inches(0.5), Inches(6.5), Inches(9), Inches(0.4),
                "Steps 3-5 create the feedback loop: PIK → higher notional → recalculated hazard and recovery → different default/survival outcomes.",
                font_size=13, bold=True, color=HEADER_BLUE)


def slide_13_breakeven_table(prs):
    """Breakeven spread results table."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "4")
    add_slide_title(slide, "Breakeven Spreads: The Answer",
                    "Merton MC effective spreads by structure and credit quality")
    add_slide_number(slide, 13, 18)

    rows = [
        ["Issuer", "Cash (bp)", "PIK (bp)", "Toggle (bp)", "PIK − Cash", "Toggle − Cash"],
        ["BB+ (Solid HY)", "—*", "—*", "—*", "−5", "−5"],
        ["BB- (Mid HY)", "—*", "—*", "—*", "+0", "+0"],
        ["B (Weak HY)", "22", "89", "106", "+67", "+84"],
        ["B- (Stressed)", "453", "689", "743", "+236", "+290"],
        ["CCC (Deeply Stressed)", "773", "1,288", "1,388", "+515", "+615"],
    ]
    add_table(slide, Inches(0.5), Inches(1.4), Inches(9), rows,
              col_widths=[Inches(2.5), Inches(1.1), Inches(1.1), Inches(1.3), Inches(1.2), Inches(1.5)],
              font_size=12)

    add_textbox(slide, Inches(0.5), Inches(4.0), Inches(9), Inches(0.3),
                "* BB+ and BB- bonds price above par (negative spread to risk-free). PIK premium still applies to the relative difference.",
                font_size=11, color=TEXT_MID)

    add_bullet_list(slide, Inches(0.5), Inches(4.5), Inches(9), Inches(2), [
        "BB+ to BB-: PIK premium is negligible (0-5bp). Defaults are too rare for the feedback loop to matter.",
        "B: PIK premium of ~67bp. The feedback loop starts to bite.",
        "B- : PIK premium of ~236bp. The spiral is significant — PIK spread is 50% higher than cash.",
        "CCC: PIK premium of ~515bp. The feedback loop dominates — PIK spread is 67% higher than cash.",
    ], font_size=14, color=TEXT_DARK)

    add_callout_box(slide, Inches(1.5), Inches(6.5), Inches(7), Inches(0.5),
                    'The PIK premium is NOT a flat "add 50bp." It ranges from zero to 500+ bp.',
                    bg_color=ACCENT_ORANGE, font_size=13)


def slide_14_pik_premium_chart(prs):
    """PIK premium vs asset coverage — described as a data visual."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "4")
    add_slide_title(slide, "PIK Premium vs Asset Coverage",
                    "The hockey-stick relationship")
    add_slide_number(slide, 14, 18)

    # Since we can't embed matplotlib easily, represent as a table + visual description
    rows = [
        ["Coverage", "Cash (bp)", "PIK (bp)", "Premium"],
        ["1.10x", "716", "864", "+149bp"],
        ["1.25x", "296", "392", "+96bp"],
        ["1.40x", "22", "89", "+67bp"],
        ["1.55x", "−160", "−110", "+50bp"],
        ["1.70x", "−296", "−259", "+38bp"],
        ["1.85x", "−395", "−366", "+29bp"],
        ["2.00x", "−465", "−442", "+24bp"],
        ["2.15x", "−520", "−501", "+19bp"],
    ]
    add_table(slide, Inches(0.5), Inches(1.3), Inches(5.5), rows,
              col_widths=[Inches(1.2), Inches(1.2), Inches(1.2), Inches(1.2)],
              font_size=11)

    # Visual bar representation of premium
    add_textbox(slide, Inches(6.3), Inches(1.3), Inches(3.2), Inches(0.3),
                "PIK Premium (bp)", font_size=13, bold=True, color=HEADER_BLUE)

    premiums = [149, 96, 67, 50, 38, 29, 24, 19]
    labels = ["1.10x", "1.25x", "1.40x", "1.55x", "1.70x", "1.85x", "2.00x", "2.15x"]
    max_prem = 149

    for i, (prem, label) in enumerate(zip(premiums, labels)):
        y = Inches(1.7) + Inches(i * 0.45)
        bar_width = Inches(2.5) * prem / max_prem
        # label
        add_textbox(slide, Inches(6.3), y, Inches(0.5), Inches(0.3),
                    label, font_size=10, color=TEXT_MID)
        # bar
        color = ACCENT_RED if prem > 80 else (ACCENT_ORANGE if prem > 40 else ACCENT_GREEN)
        bar = slide.shapes.add_shape(
            MSO_SHAPE.RECTANGLE, Inches(6.8), y + Pt(2), int(bar_width), Pt(14))
        bar.fill.solid()
        bar.fill.fore_color.rgb = color
        bar.line.fill.background()
        # value
        add_textbox(slide, Inches(6.8) + int(bar_width) + Pt(4), y - Pt(1), Inches(0.8), Inches(0.3),
                    f"+{prem}", font_size=10, bold=True, color=color)

    # Zones
    add_callout_box(slide, Inches(0.5), Inches(5.5), Inches(2.8), Inches(0.5),
                    "DANGER ZONE", bg_color=ACCENT_RED, font_size=12)
    add_textbox(slide, Inches(0.5), Inches(6.1), Inches(2.8), Inches(0.4),
                "Below 1.3x coverage:\nPIK premium 80-150+ bp", font_size=11, color=TEXT_DARK)

    add_callout_box(slide, Inches(3.5), Inches(5.5), Inches(2.8), Inches(0.5),
                    "CAUTION ZONE", bg_color=ACCENT_ORANGE, font_size=12)
    add_textbox(slide, Inches(3.5), Inches(6.1), Inches(2.8), Inches(0.4),
                "1.3x - 1.6x coverage:\nPIK premium 40-80bp", font_size=11, color=TEXT_DARK)

    add_callout_box(slide, Inches(6.5), Inches(5.5), Inches(2.8), Inches(0.5),
                    "SAFE ZONE", bg_color=ACCENT_GREEN, font_size=12)
    add_textbox(slide, Inches(6.5), Inches(6.1), Inches(2.8), Inches(0.4),
                "Above 1.6x coverage:\nPIK premium < 40bp", font_size=11, color=TEXT_DARK)


def slide_15_toggle_not_free(prs):
    """Toggle is not free."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "4")
    add_slide_title(slide, "The Toggle Is Not Free",
                    "B- issuer: comparing toggle strategies")
    add_slide_number(slide, 15, 18)

    rows = [
        ["Strategy", "Price", "Spread", "E[Loss]", "PIK %", "Term. Notional"],
        ["Cash-Pay", "63.67", "453bp", "45.8%", "0%", "100.0"],
        ["Full PIK", "56.59", "689bp", "51.8%", "100%", "151.8"],
        ["Threshold (h > 10%)", "55.08", "743bp", "53.1%", "13%", "102.1"],
        ["Stochastic (sigmoid)", "55.08", "743bp", "53.1%", "21%", "106.8"],
        ["Optimal (nested MC)", "55.48", "728bp", "52.8%", "17%", "106.2"],
    ]
    add_table(slide, Inches(0.5), Inches(1.4), Inches(9), rows,
              col_widths=[Inches(2.2), Inches(1.0), Inches(1.0), Inches(1.2), Inches(0.9), Inches(1.5)],
              font_size=11)

    add_bullet_list(slide, Inches(0.5), Inches(3.9), Inches(9), Inches(2), [
        "Counterintuitive: toggle spreads can exceed full PIK (743 vs 689bp)",
        "The borrower PIKs precisely when credit is deteriorating — adverse selection against the investor",
        "Even with only 13% of coupons PIK'd, the spread impact is significant (+290bp over cash)",
        "The optimal exercise model (nested MC) finds the best strategy for the borrower — worst for investors",
    ], font_size=14, color=TEXT_DARK)

    add_callout_box(slide, Inches(1.5), Inches(6.0), Inches(7), Inches(0.7),
                    "A toggle option does not protect the investor. It gives the borrower the ability "
                    "to accelerate leverage precisely when it does the most damage.",
                    bg_color=ACCENT_RED, font_size=12)


def slide_16_implied_hazard(prs):
    """Implied hazard premium."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "4")
    add_slide_title(slide, "Implied Hazard Premium",
                    "How much extra default risk does the market charge for PIK?")
    add_slide_number(slide, 16, 18)

    add_textbox(slide, Inches(0.5), Inches(1.2), Inches(9), Inches(0.6),
                "Question: Given the MC model price, what flat hazard rate λ would reproduce it?\n"
                "The difference between PIK-implied λ and Cash-implied λ is the structural hazard premium.",
                font_size=14, color=TEXT_DARK)

    rows = [
        ["Issuer", "Base λ", "λ Cash", "λ PIK", "Δλ (PIK premium)"],
        ["BB+ (Solid HY)", "150", "136", "215", "+79bp"],
        ["BB- (Mid HY)", "350", "665", "677", "+12bp"],
        ["B (Weak HY)", "600", "1,569", "1,558", "−11bp"],
        ["B- (Stressed)", "900", "2,435", "2,617", "+182bp"],
        ["CCC (Deeply Stressed)", "1,400", "2,976", "3,685", "+708bp"],
    ]
    add_table(slide, Inches(0.5), Inches(2.2), Inches(9), rows,
              col_widths=[Inches(2.5), Inches(1.0), Inches(1.0), Inches(1.0), Inches(2)],
              font_size=12)

    add_bullet_list(slide, Inches(0.5), Inches(4.5), Inches(9), Inches(1.5), [
        "For CCC credits, the PIK-implied hazard is ~708bp above the cash-implied hazard",
        "This is the structural premium — the extra default risk created by the feedback loop",
        "Simple models (flat λ) cannot capture this because they treat λ as fixed",
    ], font_size=14, color=TEXT_DARK)

    add_callout_box(slide, Inches(1.5), Inches(6.2), Inches(7), Inches(0.5),
                    "The implied hazard approach converts the structural model's insight into the hazard-rate "
                    "language that flat-curve models understand.",
                    bg_color=HEADER_BLUE, font_size=12)


def slide_17_when_does_it_matter(prs):
    """Three-zone framework."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "5")
    add_slide_title(slide, "When Does the PIK Premium Matter?",
                    "A practical framework")
    add_slide_number(slide, 17, 18)

    # Green zone
    add_callout_box(slide, Inches(0.5), Inches(1.3), Inches(2.8), Inches(0.5),
                    "LOW RISK", bg_color=ACCENT_GREEN, font_size=14)
    add_textbox(slide, Inches(0.5), Inches(1.9), Inches(2.8), Inches(0.3),
                "BB+ to BB  |  Coverage > 1.6x", font_size=12, bold=True, color=TEXT_DARK)
    add_bullet_list(slide, Inches(0.5), Inches(2.3), Inches(2.8), Inches(1.5), [
        "PIK premium < 50bp",
        "Feedback loop is negligible",
        "Simple spread adjustment is fine",
        "Focus on relative value, not model",
    ], font_size=12, color=TEXT_DARK, spacing_after=Pt(4))

    # Yellow zone
    add_callout_box(slide, Inches(3.6), Inches(1.3), Inches(2.8), Inches(0.5),
                    "MODERATE RISK", bg_color=ACCENT_ORANGE, font_size=14)
    add_textbox(slide, Inches(3.6), Inches(1.9), Inches(2.8), Inches(0.3),
                "B  |  Coverage 1.3x - 1.6x", font_size=12, bold=True, color=TEXT_DARK)
    add_bullet_list(slide, Inches(3.6), Inches(2.3), Inches(2.8), Inches(1.5), [
        "PIK premium 50 - 200bp",
        "Simple models understate risk",
        "Structural model adds material value",
        "Scrutinise leverage trajectory",
    ], font_size=12, color=TEXT_DARK, spacing_after=Pt(4))

    # Red zone
    add_callout_box(slide, Inches(6.7), Inches(1.3), Inches(2.8), Inches(0.5),
                    "HIGH RISK", bg_color=ACCENT_RED, font_size=14)
    add_textbox(slide, Inches(6.7), Inches(1.9), Inches(2.8), Inches(0.3),
                "B- to CCC  |  Coverage < 1.3x", font_size=12, bold=True, color=TEXT_DARK)
    add_bullet_list(slide, Inches(6.7), Inches(2.3), Inches(2.8), Inches(1.5), [
        "PIK premium 200 - 500+ bp",
        "Feedback loop dominates",
        "Structural model is essential",
        "Simple spread bump is dangerous",
    ], font_size=12, color=TEXT_DARK, spacing_after=Pt(4))

    # Summary table
    add_textbox(slide, Inches(0.5), Inches(4.2), Inches(9), Inches(0.3),
                "Practical Decision Guide:", font_size=14, bold=True, color=HEADER_BLUE)

    guide_rows = [
        ["Question", "Low Risk", "Moderate Risk", "High Risk"],
        ["PIK premium estimate", "~25-50bp", "~50-200bp", "200-500+bp"],
        ["Model needed?", "Spread bump OK", "Structural recommended", "Structural required"],
        ["Toggle vs full PIK", "Minimal difference", "Toggle may exceed PIK", "Toggle can exceed PIK"],
        ["Key sensitivity", "Duration, coupon", "Leverage trajectory", "Feedback loop intensity"],
    ]
    add_table(slide, Inches(0.5), Inches(4.6), Inches(9), guide_rows,
              col_widths=[Inches(2.5), Inches(2.2), Inches(2.2), Inches(2.2)],
              font_size=11)


def slide_18_takeaways(prs):
    """Key takeaways."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, MID_BG)
    add_slide_number(slide, 18, 18)

    add_textbox(slide, Inches(0.8), Inches(0.5), Inches(8), Inches(0.6),
                "Key Takeaways", font_size=32, bold=True, color=WHITE)

    # Thin accent line
    shape = slide.shapes.add_shape(
        MSO_SHAPE.RECTANGLE, Inches(0.8), Inches(1.15), Inches(2), Pt(3))
    shape.fill.solid()
    shape.fill.fore_color.rgb = ACCENT_BLUE
    shape.line.fill.background()

    takeaways = [
        ("1", "PIK premium is non-linear in credit quality",
         "Negligible for strong credits (BB), massive for weak ones (CCC: 500+bp). "
         "A flat spread bump across the portfolio is wrong."),
        ("2", "Simple hazard models capture timing but miss the feedback loop",
         "The flat-λ model underestimates the PIK discount by up to 9 points for stressed credits. "
         "The gap is the endogenous leverage-hazard spiral."),
        ("3", "The toggle option does not protect the investor",
         "Borrowers PIK when credit deteriorates — adverse selection. Toggle spreads can exceed full PIK. "
         "Price toggles as 'PIK plus optionality cost.'"),
        ("4", "Below ~1.3x coverage, structural modelling is essential",
         "In the danger zone, the feedback loop dominates pricing. "
         "A spread bump approach will materially misprice PIK risk."),
    ]

    for i, (num, title, detail) in enumerate(takeaways):
        y = Inches(1.5) + Inches(i * 1.3)
        # number
        shape = slide.shapes.add_shape(MSO_SHAPE.OVAL, Inches(0.8), y, Inches(0.45), Inches(0.45))
        shape.fill.solid()
        shape.fill.fore_color.rgb = ACCENT_BLUE
        shape.line.fill.background()
        tf = shape.text_frame
        tf.margin_left = tf.margin_right = tf.margin_top = tf.margin_bottom = Pt(0)
        p = tf.paragraphs[0]
        p.text = num
        p.font.size = Pt(16)
        p.font.bold = True
        p.font.color.rgb = WHITE
        p.alignment = PP_ALIGN.CENTER
        tf.vertical_anchor = MSO_ANCHOR.MIDDLE

        add_textbox(slide, Inches(1.5), y - Pt(2), Inches(7.5), Inches(0.35),
                    title, font_size=17, bold=True, color=WHITE)
        add_textbox(slide, Inches(1.5), y + Inches(0.35), Inches(7.5), Inches(0.6),
                    detail, font_size=13, color=LIGHT_GREY)


# ── Main ──────────────────────────────────────────────────────────────────

def main():
    prs = Presentation()
    prs.slide_width = Inches(10)
    prs.slide_height = Inches(7.5)

    slide_01_title(prs)
    slide_02_the_question(prs)
    slide_03_coupon_recap(prs)
    slide_04_hazard_rate(prs)
    slide_05_pricing_with_hazard(prs)
    slide_06_pik_in_hazard_model(prs)
    slide_07_what_simple_misses(prs)
    slide_08_merton_model(prs)
    slide_09_endogenous_hazard(prs)
    slide_10_dynamic_recovery(prs)
    slide_11_feedback_loop(prs)
    slide_12_monte_carlo(prs)
    slide_13_breakeven_table(prs)
    slide_14_pik_premium_chart(prs)
    slide_15_toggle_not_free(prs)
    slide_16_implied_hazard(prs)
    slide_17_when_does_it_matter(prs)
    slide_18_takeaways(prs)

    out = Path(__file__).parent / "pik_coupon_pricing.pptx"
    prs.save(str(out))
    print(f"Saved: {out}")
    print(f"Slides: {len(prs.slides)}")


if __name__ == "__main__":
    main()
