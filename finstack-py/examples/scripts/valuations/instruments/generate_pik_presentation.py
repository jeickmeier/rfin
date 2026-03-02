#!/usr/bin/env python3
"""Generate a PowerPoint deck on PIK coupon breakeven modelling.

Audience: credit team familiar with coupon types.
Condensed 6-slide version: question → HR results → structural results →
premium sweep → takeaways.

Usage:
    python generate_pik_presentation.py
    # => writes pik_coupon_pricing.pptx
"""

from __future__ import annotations

from pathlib import Path

from pptx import Presentation
from pptx.util import Inches, Pt
from pptx.dml.color import RGBColor
from pptx.enum.text import PP_ALIGN, MSO_ANCHOR
from pptx.enum.shapes import MSO_SHAPE

# ── Colour palette ────────────────────────────────────────────────────────

DARK_BG = RGBColor(0x1A, 0x1A, 0x2E)
MID_BG = RGBColor(0x16, 0x21, 0x3E)
WHITE = RGBColor(0xFF, 0xFF, 0xFF)
LIGHT_GREY = RGBColor(0xCC, 0xCC, 0xCC)
ACCENT_BLUE = RGBColor(0x4E, 0xC9, 0xB0)
ACCENT_ORANGE = RGBColor(0xE8, 0x8D, 0x3F)
ACCENT_RED = RGBColor(0xE0, 0x4F, 0x4F)
ACCENT_GREEN = RGBColor(0x5C, 0xB8, 0x5C)
BODY_BG = RGBColor(0xF5, 0xF5, 0xFA)
TEXT_DARK = RGBColor(0x2D, 0x2D, 0x3D)
TEXT_MID = RGBColor(0x55, 0x55, 0x70)
HEADER_BLUE = RGBColor(0x2C, 0x3E, 0x6B)
TABLE_HEADER_BG = RGBColor(0x2C, 0x3E, 0x6B)
TABLE_ALT_BG = RGBColor(0xE8, 0xEB, 0xF5)
TABLE_WHITE_BG = RGBColor(0xFF, 0xFF, 0xFF)

TOTAL_SLIDES = 6


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
                    font_size=16, color=TEXT_DARK, spacing_after=Pt(8)):
    txBox = slide.shapes.add_textbox(left, top, width, height)
    tf = txBox.text_frame
    tf.word_wrap = True
    for i, item in enumerate(items):
        p = tf.paragraphs[0] if i == 0 else tf.add_paragraph()
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
    tf.margin_left = tf.margin_right = tf.margin_top = tf.margin_bottom = Pt(0)
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
    tbl_shape = slide.shapes.add_table(
        n_rows, n_cols, left, top, width, Inches(0.35 * n_rows))
    table = tbl_shape.table
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
                    paragraph.alignment = PP_ALIGN.LEFT if c == 0 else PP_ALIGN.RIGHT
            if r == 0 and header_row:
                cell.fill.solid()
                cell.fill.fore_color.rgb = TABLE_HEADER_BG
            elif r % 2 == 0:
                cell.fill.solid()
                cell.fill.fore_color.rgb = TABLE_ALT_BG
            else:
                cell.fill.solid()
                cell.fill.fore_color.rgb = TABLE_WHITE_BG
    return tbl_shape


def add_slide_number(slide, num: int):
    add_textbox(slide, Inches(8.8), Inches(7.1), Inches(1.2), Inches(0.3),
                f"{num}/{TOTAL_SLIDES}", font_size=10, color=TEXT_MID,
                alignment=PP_ALIGN.RIGHT)


def add_slide_title(slide, title: str, subtitle: str = ""):
    add_textbox(slide, Inches(1.1), Inches(0.2), Inches(8), Inches(0.5),
                title, font_size=26, bold=True, color=HEADER_BLUE)
    if subtitle:
        add_textbox(slide, Inches(1.1), Inches(0.65), Inches(8), Inches(0.4),
                    subtitle, font_size=14, color=TEXT_MID)


# ── Slide 1: Title ───────────────────────────────────────────────────────

def slide_01_title(prs):
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, DARK_BG)
    add_textbox(slide, Inches(1), Inches(2), Inches(8), Inches(1.2),
                "PIK Coupon Pricing", font_size=40, bold=True, color=WHITE)
    add_textbox(slide, Inches(1), Inches(3.1), Inches(8), Inches(0.8),
                "How Much Extra Spread Is Enough?", font_size=24, color=ACCENT_BLUE)
    add_textbox(slide, Inches(1), Inches(4.2), Inches(8), Inches(0.6),
                "Modelling breakeven Z-spreads for Cash, PIK, and Toggle coupons\n"
                "across issuer credit quality",
                font_size=16, color=LIGHT_GREY)
    bar = slide.shapes.add_shape(
        MSO_SHAPE.RECTANGLE, Inches(1), Inches(3.95), Inches(3), Pt(3))
    bar.fill.solid()
    bar.fill.fore_color.rgb = ACCENT_BLUE
    bar.line.fill.background()


# ── Slide 2: The Question ────────────────────────────────────────────────

def slide_02_the_question(prs):
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "1")
    add_slide_title(slide, "The Question")
    add_slide_number(slide, 2)

    add_textbox(slide, Inches(0.8), Inches(1.2), Inches(8.5), Inches(0.7),
                "At what spread should a PIK bond trade versus a\n"
                "cash-pay bond for the same issuer?",
                font_size=20, bold=True, color=HEADER_BLUE,
                alignment=PP_ALIGN.CENTER)

    add_callout_box(slide, Inches(0.5), Inches(2.2), Inches(4.2), Inches(0.45),
                    "Naive View", bg_color=TEXT_MID)
    add_bullet_list(slide, Inches(0.5), Inches(2.8), Inches(4.2), Inches(2.5), [
        '"PIK just defers coupons \u2014 add 50bp"',
        '"The coupon rate is the same either way"',
        '"Compounding offsets the deferral"',
    ], font_size=14, color=TEXT_DARK)

    add_callout_box(slide, Inches(5.3), Inches(2.2), Inches(4.2), Inches(0.45),
                    "What the Models Show", bg_color=ACCENT_BLUE)
    add_bullet_list(slide, Inches(5.3), Inches(2.8), Inches(4.2), Inches(2.5), [
        "For strong credits (BB+): PIK premium is small (\u00b160bp) "
        "and sign is model-dependent",
        "For stressed credits (CCC): PIK premium > +260bp, "
        "robust across models",
        "The non-linearity means flat spread bumps are dangerous",
    ], font_size=14, color=TEXT_DARK)

    add_callout_box(slide, Inches(1.5), Inches(5.3), Inches(7), Inches(0.7),
                    "The answer depends on credit quality in a way that is impossible "
                    "to guess without a model. The premium is non-linear and, for "
                    "strong credits, depends on calibration assumptions.",
                    bg_color=ACCENT_ORANGE, font_size=14)


# ── Slide 3: HR Model — PIK Always Costs ─────────────────────────────────

def slide_03_hr_results(prs):
    """Merge of old slides 6 (HR prices) + 7 (calibration gap)."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "2")
    add_slide_title(slide, "Hazard Rate Model: PIK Always Costs",
                    "Risk-neutral pricing with hazard rates from market spreads")
    add_slide_number(slide, 3)

    # ── HR results table (from old slide 6) ──────────────────────────
    hr_rows = [
        ["Issuer", "\u03bb (bp)", "Cash Price", "PIK Price", "\u0394Z (bp)"],
        ["BB+ (Solid HY)", "143", "113.35", "111.46", "+40"],
        ["BB\u2212 (Mid HY)", "334", "107.56", "104.88", "+60"],
        ["B (Weak HY)", "591", "99.76", "96.05", "+91"],
        ["B\u2212 (Stressed)", "911", "90.33", "85.40", "+137"],
        ["CCC (Deeply Stressed)", "1468", "76.13", "69.59", "+224"],
    ]
    add_table(slide, Inches(0.5), Inches(1.1), Inches(5.5), hr_rows,
              col_widths=[Inches(1.8), Inches(0.7), Inches(1.0),
                          Inches(1.0), Inches(1.0)],
              font_size=11)

    # ── Calibration gap table (from old slide 7, compressed) ─────────
    add_textbox(slide, Inches(6.3), Inches(1.1), Inches(3.5), Inches(0.3),
                "The Calibration Gap", font_size=14, bold=True,
                color=HEADER_BLUE)
    gap_rows = [
        ["Issuer", "HR 5Y PD", "Hist 5Y PD", "Ratio"],
        ["BB+", "6.9%", "1.0%", "6.9\u00d7"],
        ["B", "25.6%", "11.8%", "2.2\u00d7"],
        ["CCC", "52.0%", "39.3%", "1.3\u00d7"],
    ]
    add_table(slide, Inches(6.3), Inches(1.45), Inches(3.5), gap_rows,
              col_widths=[Inches(0.7), Inches(0.9), Inches(0.9),
                          Inches(0.7)],
              font_size=10)

    # ── Bullets ──────────────────────────────────────────────────────
    add_bullet_list(slide, Inches(0.5), Inches(3.8), Inches(9), Inches(1.5), [
        "Under risk-neutral hazard rates, PIK always trades wider: "
        "+40bp (BB+) to +224bp (CCC). The concentrated maturity "
        "exposure penalises PIK at every credit level",
        "But market spreads embed a risk premium above expected "
        "losses. For BB+ the spread implies 7\u00d7 the historical "
        "default rate \u2014 the PIK penalty may be overstated for "
        "strong credits",
    ], font_size=13, color=TEXT_DARK)

    add_callout_box(slide, Inches(1.0), Inches(5.5), Inches(8), Inches(0.65),
                    "Is the PIK penalty real, or is it driven by overstated "
                    "default risk? The structural model \u2014 which uses "
                    "historical PDs \u2014 tests this.",
                    bg_color=HEADER_BLUE, font_size=12)


# ── Slide 4: Structural Model — Feedback Loop + MC Results ───────────────

def slide_04_structural_results(prs):
    """Merge of old slides 9 (endo hazard) + 10 (dyn recovery)
    + 11 (feedback loop) + 13 (MC breakeven table)."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "3")
    add_slide_title(slide, "Structural Model: The Feedback Loop",
                    "Merton MC with endogenous hazard + dynamic recovery")
    add_slide_number(slide, 4)

    # ── Two key formulas (compact) ───────────────────────────────────
    add_textbox(slide, Inches(0.5), Inches(1.1), Inches(4.5), Inches(0.35),
                "\u03bb(L) = \u03bb\u2080 \u00d7 (L / L\u2080)\u00b2",
                font_size=16, bold=True, color=HEADER_BLUE)
    add_textbox(slide, Inches(0.5), Inches(1.45), Inches(4.5), Inches(0.3),
                "PIK raises leverage \u2192 hazard rate rises non-linearly",
                font_size=11, color=TEXT_MID)

    add_textbox(slide, Inches(0.5), Inches(1.85), Inches(4.5), Inches(0.35),
                "R(N) = max(floor,  R\u2080 \u00d7 N\u2080 / N)",
                font_size=16, bold=True, color=HEADER_BLUE)
    add_textbox(slide, Inches(0.5), Inches(2.2), Inches(4.5), Inches(0.3),
                "PIK grows debt claim \u2192 recovery per dollar diluted",
                font_size=11, color=TEXT_MID)

    # ── Mini feedback spiral (right side, simplified) ────────────────
    spiral_items = [
        (Inches(5.8), Inches(1.1), "PIK Accrual\n\u2193 Higher Notional"),
        (Inches(7.5), Inches(1.1), "Higher Leverage\n\u2193 Higher \u03bb"),
        (Inches(5.8), Inches(2.0), "Lower Recovery\nper dollar"),
        (Inches(7.5), Inches(2.0), "More Defaults\nBefore Maturity"),
    ]
    colors = [ACCENT_ORANGE, ACCENT_RED, ACCENT_RED, ACCENT_RED]
    for (x, y, label), col in zip(spiral_items, colors):
        shape = slide.shapes.add_shape(
            MSO_SHAPE.ROUNDED_RECTANGLE, x, y, Inches(1.5), Inches(0.65))
        shape.fill.solid()
        shape.fill.fore_color.rgb = col
        shape.line.fill.background()
        tf = shape.text_frame
        tf.word_wrap = True
        tf.margin_left = tf.margin_right = Pt(4)
        tf.margin_top = tf.margin_bottom = Pt(2)
        p = tf.paragraphs[0]
        p.text = label
        p.font.size = Pt(9)
        p.font.bold = True
        p.font.color.rgb = WHITE
        p.font.name = "Calibri"
        p.alignment = PP_ALIGN.CENTER
        tf.vertical_anchor = MSO_ANCHOR.MIDDLE

    # Arrows connecting the 4 boxes: right, down, left, up
    arrows = [
        (Inches(7.3), Inches(1.25), "\u2192"),
        (Inches(8.3), Inches(1.8), "\u2193"),
        (Inches(7.3), Inches(2.15), "\u2190"),
        (Inches(5.6), Inches(1.8), "\u2191"),
    ]
    for x, y, arrow in arrows:
        add_textbox(slide, x, y, Inches(0.4), Inches(0.35),
                    arrow, font_size=14, bold=True, color=HEADER_BLUE,
                    alignment=PP_ALIGN.CENTER)

    # ── MC breakeven Z-spread table (from old slide 13) ──────────────
    add_textbox(slide, Inches(0.5), Inches(2.7), Inches(9), Inches(0.3),
                "MC Breakeven Z-Spreads (25,000 paths, barriers from "
                "historical PDs)",
                font_size=12, bold=True, color=HEADER_BLUE)

    mc_rows = [
        ["Issuer", "Cash", "PIK", "Toggle", "PIK\u2212Cash", "Tog\u2212Cash"],
        ["BB+ (Solid HY)", "20bp", "\u221239bp", "22bp", "\u221259", "+1"],
        ["BB\u2212 (Mid HY)", "110bp", "88bp", "130bp", "\u221222", "+20"],
        ["B (Weak HY)", "292bp", "329bp", "346bp", "+37", "+54"],
        ["B\u2212 (Stressed)", "710bp", "851bp", "862bp", "+141", "+151"],
        ["CCC (Deeply Stressed)", "1,497bp", "1,759bp", "1,763bp", "+262", "+266"],
    ]
    add_table(slide, Inches(0.5), Inches(3.05), Inches(9), mc_rows,
              col_widths=[Inches(2.2), Inches(1.1), Inches(1.1),
                          Inches(1.1), Inches(1.2), Inches(1.2)],
              font_size=11)

    # ── Bullets ──────────────────────────────────────────────────────
    add_bullet_list(slide, Inches(0.5), Inches(5.3), Inches(9), Inches(1.0), [
        "BB+/BB\u2212: MC shows PIK tighter (\u221259/\u221222bp) \u2014 "
        "under historical PDs, PIK compounding survives on ~98% of paths",
        "B\u2212 to CCC: PIK premium +141 to +262bp \u2014 the "
        "feedback loop dominates and both models agree",
    ], font_size=13, color=TEXT_DARK)

    add_callout_box(slide, Inches(1.0), Inches(6.3), Inches(8), Inches(0.55),
                    "For weak credits, the feedback loop is unambiguous. "
                    "For strong credits, the sign depends on whether you "
                    "calibrate to market or historical defaults.",
                    bg_color=ACCENT_ORANGE, font_size=12)


# ── Slide 5: PIK Premium Across Credit Quality ──────────────────────────

def slide_05_premium_sweep(prs):
    """Merge of old slides 14 (hockey stick) + 15 (toggle) + 16 (model gap)."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, BODY_BG)
    add_section_number(slide, "4")
    add_slide_title(slide, "PIK Premium Across Credit Quality",
                    "The hockey-stick relationship")
    add_slide_number(slide, 5)

    # ── Premium table (left, from old slide 14) ──────────────────────
    prem_rows = [
        ["Mkt Spread", "Cash Z", "PIK Z", "Premium"],
        ["50bp", "10bp", "\u221255bp", "\u221265bp"],
        ["100bp", "31bp", "\u221224bp", "\u221254bp"],
        ["200bp", "87bp", "56bp", "\u221231bp"],
        ["300bp", "168bp", "166bp", "\u22122bp"],
        ["400bp", "292bp", "329bp", "+37bp"],
        ["600bp", "552bp", "659bp", "+107bp"],
        ["850bp", "1,119bp", "1,335bp", "+216bp"],
        ["1,200bp", "1,702bp", "1,990bp", "+287bp"],
    ]
    add_table(slide, Inches(0.3), Inches(1.1), Inches(4.5), prem_rows,
              col_widths=[Inches(1.0), Inches(1.0), Inches(1.0),
                          Inches(1.0)],
              font_size=10)

    # ── Hockey-stick bar chart (right, from old slide 14) ────────────
    add_textbox(slide, Inches(5.1), Inches(1.1), Inches(2), Inches(0.3),
                "PIK Z-Spread Premium", font_size=12, bold=True,
                color=HEADER_BLUE)

    premiums = [
        ("50bp", -65), ("100bp", -54), ("200bp", -31), ("300bp", -2),
        ("400bp", 37), ("600bp", 107), ("850bp", 216), ("1200bp", 287),
    ]
    max_abs = 287

    for i, (label, prem) in enumerate(premiums):
        y = Inches(1.45) + Inches(i * 0.42)
        add_textbox(slide, Inches(5.0), y, Inches(0.6), Inches(0.3),
                    label, font_size=8, color=TEXT_MID)
        zero_x = Inches(6.7)
        if prem >= 0:
            bar_w = Inches(1.3) * prem / max_abs
            color = ACCENT_RED if prem > 80 else (
                ACCENT_ORANGE if prem > 30 else ACCENT_GREEN)
            bar = slide.shapes.add_shape(
                MSO_SHAPE.RECTANGLE, int(zero_x), y + Pt(2),
                int(bar_w), Pt(12))
            bar.fill.solid()
            bar.fill.fore_color.rgb = color
            bar.line.fill.background()
            add_textbox(slide, int(zero_x) + int(bar_w) + Pt(4),
                        y - Pt(1), Inches(0.7), Inches(0.3),
                        f"+{prem}", font_size=8, bold=True, color=color)
        else:
            bar_w = Inches(1.3) * abs(prem) / max_abs
            bar = slide.shapes.add_shape(
                MSO_SHAPE.RECTANGLE, int(zero_x) - int(bar_w),
                y + Pt(2), int(bar_w), Pt(12))
            bar.fill.solid()
            bar.fill.fore_color.rgb = ACCENT_GREEN
            bar.line.fill.background()
            add_textbox(slide, int(zero_x) - int(bar_w) - Inches(0.5),
                        y - Pt(1), Inches(0.5), Inches(0.3),
                        str(prem), font_size=8, bold=True,
                        color=ACCENT_GREEN, alignment=PP_ALIGN.RIGHT)

    # Zero line
    zero_line = slide.shapes.add_shape(
        MSO_SHAPE.RECTANGLE, int(Inches(6.7)), Inches(1.45),
        Pt(2), Inches(3.4))
    zero_line.fill.solid()
    zero_line.fill.fore_color.rgb = TEXT_MID
    zero_line.line.fill.background()

    # ── 3-zone boxes (below chart) ───────────────────────────────────
    zone_y = Inches(4.85)
    add_callout_box(slide, Inches(0.3), zone_y, Inches(2.8), Inches(0.4),
                    "LOW IMPACT ZONE", bg_color=ACCENT_GREEN, font_size=10)
    add_textbox(slide, Inches(0.3), zone_y + Inches(0.45),
                Inches(2.8), Inches(0.4),
                "Mkt spread < 300bp\nPremium small, sign model-dep.",
                font_size=10, color=TEXT_DARK)

    add_callout_box(slide, Inches(3.4), zone_y, Inches(2.8), Inches(0.4),
                    "CROSSOVER ZONE", bg_color=ACCENT_ORANGE, font_size=10)
    add_textbox(slide, Inches(3.4), zone_y + Inches(0.45),
                Inches(2.8), Inches(0.4),
                "300\u2013500bp market spread\nPIK premium 0\u201375bp",
                font_size=10, color=TEXT_DARK)

    add_callout_box(slide, Inches(6.5), zone_y, Inches(2.8), Inches(0.4),
                    "STRUCTURAL RISK", bg_color=ACCENT_RED, font_size=10)
    add_textbox(slide, Inches(6.5), zone_y + Inches(0.45),
                Inches(2.8), Inches(0.4),
                "Mkt spread > 500bp\nPIK premium 75\u2013290+ bp",
                font_size=10, color=TEXT_DARK)

    # ── Model comparison row (from old slide 16) ─────────────────────
    cmp_rows = [
        ["", "BB+", "BB\u2212", "B", "B\u2212", "CCC"],
        ["HR \u0394Z", "+40", "+60", "+91", "+137", "+224"],
        ["MC \u0394Z", "\u221259", "\u221222", "+37", "+141", "+262"],
    ]
    add_table(slide, Inches(0.3), Inches(5.95), Inches(6.0), cmp_rows,
              col_widths=[Inches(0.8), Inches(1.0), Inches(1.0),
                          Inches(1.0), Inches(1.0), Inches(1.0)],
              font_size=10)

    add_textbox(slide, Inches(0.3), Inches(6.95), Inches(4.5), Inches(0.3),
                "\u0394Z = PIK Z-spread minus Cash Z-spread (bp)",
                font_size=9, color=TEXT_MID)

    # ── Toggle callout (from old slide 15) ───────────────────────────
    add_callout_box(slide, Inches(6.5), Inches(5.95), Inches(3.2), Inches(0.85),
                    "Toggle \u2248 full PIK or worse \u2014 "
                    "borrowers PIK on the worst paths "
                    "(adverse selection)",
                    bg_color=ACCENT_RED, font_size=11)


# ── Slide 6: Key Takeaways & Decision Framework ─────────────────────────

def slide_06_takeaways(prs):
    """Merge of old slides 17 (framework) + 18 (takeaways)."""
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    set_slide_bg(slide, MID_BG)
    add_slide_number(slide, 6)

    add_textbox(slide, Inches(0.8), Inches(0.4), Inches(8), Inches(0.5),
                "Key Takeaways", font_size=32, bold=True, color=WHITE)

    bar = slide.shapes.add_shape(
        MSO_SHAPE.RECTANGLE, Inches(0.8), Inches(1.0), Inches(2), Pt(3))
    bar.fill.solid()
    bar.fill.fore_color.rgb = ACCENT_BLUE
    bar.line.fill.background()

    # ── 3 takeaways ──────────────────────────────────────────────────
    takeaways = [
        ("1", "PIK premium is non-linear and credit-quality-dependent",
         "For strong credits (BB+/BB\u2212), the premium is small "
         "(\u00b160bp) and model-dependent. For stressed credits "
         "(CCC), it exceeds +260bp and is robust across models."),
        ("2", "The toggle option does not protect the investor",
         "Borrowers PIK when credit deteriorates \u2014 adverse "
         "selection. Toggle Z-spreads can exceed full PIK because "
         "the spiral concentrates on the worst paths."),
        ("3", "Above ~400bp market spread, structural modelling is essential",
         "In the structural risk zone, the feedback loop dominates "
         "pricing regardless of calibration. A flat spread bump "
         "approach will materially misprice PIK risk."),
    ]

    for i, (num, title, detail) in enumerate(takeaways):
        y = Inches(1.3) + Inches(i * 1.15)
        shape = slide.shapes.add_shape(
            MSO_SHAPE.OVAL, Inches(0.8), y, Inches(0.4), Inches(0.4))
        shape.fill.solid()
        shape.fill.fore_color.rgb = ACCENT_BLUE
        shape.line.fill.background()
        tf = shape.text_frame
        tf.margin_left = tf.margin_right = Pt(0)
        tf.margin_top = tf.margin_bottom = Pt(0)
        p = tf.paragraphs[0]
        p.text = num
        p.font.size = Pt(14)
        p.font.bold = True
        p.font.color.rgb = WHITE
        p.alignment = PP_ALIGN.CENTER
        tf.vertical_anchor = MSO_ANCHOR.MIDDLE

        add_textbox(slide, Inches(1.4), y - Pt(2), Inches(7.5), Inches(0.35),
                    title, font_size=16, bold=True, color=WHITE)
        add_textbox(slide, Inches(1.4), y + Inches(0.3), Inches(7.5),
                    Inches(0.55),
                    detail, font_size=12, color=LIGHT_GREY)

    # ── Decision guide table (from old slide 17) ─────────────────────
    add_textbox(slide, Inches(0.8), Inches(4.7), Inches(8), Inches(0.3),
                "Decision Guide", font_size=14, bold=True, color=ACCENT_BLUE)

    guide_rows = [
        ["Question", "Low Impact", "Crossover", "Structural Risk"],
        ["PIK Z-spread premium", "\u00b160bp (model-dep.)", "0 to +75bp",
         "+100 to +290bp"],
        ["Model needed?", "Either model adequate",
         "Structural recommended", "Structural required"],
        ["Toggle vs full PIK", "Minimal difference",
         "Toggle may exceed PIK", "Toggle \u2248 PIK or worse"],
        ["Key risk factor", "Calibration assumption",
         "Leverage trajectory", "Feedback loop intensity"],
    ]
    tbl = add_table(slide, Inches(0.5), Inches(5.05), Inches(9), guide_rows,
                    col_widths=[Inches(2.2), Inches(2.3),
                                Inches(2.3), Inches(2.3)],
                    font_size=10)

    # Override table colours to work on dark background
    table = tbl.table
    for r in range(len(guide_rows)):
        for c in range(len(guide_rows[0])):
            cell = table.cell(r, c)
            if r == 0:
                cell.fill.solid()
                cell.fill.fore_color.rgb = ACCENT_BLUE
            elif r % 2 == 0:
                cell.fill.solid()
                cell.fill.fore_color.rgb = RGBColor(0x1F, 0x2F, 0x50)
            else:
                cell.fill.solid()
                cell.fill.fore_color.rgb = RGBColor(0x25, 0x38, 0x5E)
            for paragraph in cell.text_frame.paragraphs:
                paragraph.font.color.rgb = WHITE if r == 0 else LIGHT_GREY


# ── Main ──────────────────────────────────────────────────────────────────

def main():
    prs = Presentation()
    prs.slide_width = Inches(10)
    prs.slide_height = Inches(7.5)

    slide_01_title(prs)
    slide_02_the_question(prs)
    slide_03_hr_results(prs)
    slide_04_structural_results(prs)
    slide_05_premium_sweep(prs)
    slide_06_takeaways(prs)

    out = Path(__file__).parent / "pik_coupon_pricing.pptx"
    prs.save(str(out))
    print(f"Saved: {out}")
    print(f"Slides: {len(prs.slides)}")


if __name__ == "__main__":
    main()
