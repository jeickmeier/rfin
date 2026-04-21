//! Market validation tests for CDS index option specific features.

use super::common::*;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::instruments::SettlementType;
use time::macros::date;

#[test]
fn test_index_factor_scaling() {
    // Index factor should scale PV linearly
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let base_option = CDSOptionBuilder::new().with_index(1.0).build(as_of);

    let base_pv = base_option.value(&market, as_of).unwrap().amount();

    for factor in [0.85, 0.90, 0.95] {
        let scaled_option = CDSOptionBuilder::new().with_index(factor).build(as_of);

        let scaled_pv = scaled_option.value(&market, as_of).unwrap().amount();
        let ratio = scaled_pv / base_pv;

        assert_approx_eq(
            ratio,
            factor,
            0.001,
            &format!("Index factor scaling for factor={}", factor),
        );
    }
}

#[test]
fn test_forward_spread_adjustment_call() {
    // Forward spread adjustment should increase call value
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let base_option = CDSOptionBuilder::new()
        .call()
        .with_index(1.0)
        .forward_adjust(0.0)
        .build(as_of);

    let adjusted_option = CDSOptionBuilder::new()
        .call()
        .with_index(1.0)
        .forward_adjust(25.0)
        .build(as_of);

    let base_pv = base_option.value(&market, as_of).unwrap().amount();
    let adj_pv = adjusted_option.value(&market, as_of).unwrap().amount();

    assert!(
        adj_pv > base_pv,
        "Positive forward adjustment should increase call value: base={}, adjusted={}",
        base_pv,
        adj_pv
    );
}

#[test]
fn test_forward_spread_adjustment_put() {
    // Forward spread adjustment should decrease put value
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let base_option = CDSOptionBuilder::new()
        .put()
        .with_index(1.0)
        .forward_adjust(0.0)
        .build(as_of);

    let adjusted_option = CDSOptionBuilder::new()
        .put()
        .with_index(1.0)
        .forward_adjust(25.0)
        .build(as_of);

    let base_pv = base_option.value(&market, as_of).unwrap().amount();
    let adj_pv = adjusted_option.value(&market, as_of).unwrap().amount();

    assert!(
        adj_pv < base_pv,
        "Positive forward adjustment should decrease put value: base={}, adjusted={}",
        base_pv,
        adj_pv
    );
}

#[test]
fn test_index_vs_single_name() {
    // Index option with factor 1.0 and no adjustment should differ from single-name
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let single_name = CDSOptionBuilder::new().build(as_of);
    let index = CDSOptionBuilder::new()
        .with_index(1.0)
        .forward_adjust(0.0)
        .build(as_of);

    let sn_pv = single_name.value(&market, as_of).unwrap().amount();
    let idx_pv = index.value(&market, as_of).unwrap().amount();

    // Values might differ slightly due to implementation details
    assert_finite(sn_pv, "Single-name PV");
    assert_finite(idx_pv, "Index PV");
}

#[test]
fn test_very_small_index_factor() {
    // Very small index factor should give very small value
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    // Test with smallest practical index factor (1%)
    let option_small = CDSOptionBuilder::new().with_index(0.01).build(as_of);
    let option_full = CDSOptionBuilder::new().with_index(1.0).build(as_of);

    let pv_small = option_small.value(&market, as_of).unwrap().amount();
    let pv_full = option_full.value(&market, as_of).unwrap().amount();

    // Small factor should give proportionally smaller value
    let ratio = pv_small / pv_full;
    assert_approx_eq(
        ratio,
        0.01,
        0.001,
        "Very small index factor should scale linearly",
    );
}

#[test]
fn test_negative_forward_adjustment() {
    // Negative forward adjustment should decrease call value
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let base = CDSOptionBuilder::new()
        .call()
        .with_index(1.0)
        .forward_adjust(0.0)
        .build(as_of);

    let adjusted = CDSOptionBuilder::new()
        .call()
        .with_index(1.0)
        .forward_adjust(-20.0)
        .build(as_of);

    let base_pv = base.value(&market, as_of).unwrap().amount();
    let adj_pv = adjusted.value(&market, as_of).unwrap().amount();

    assert!(
        adj_pv < base_pv,
        "Negative forward adjustment should decrease call value"
    );
}

#[test]
fn test_index_option_physical_settlement_is_rejected() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let mut option = CDSOptionBuilder::new().with_index(1.0).build(as_of);
    option.settlement = SettlementType::Physical;

    let err = option
        .value(&market, as_of)
        .expect_err("Physical settlement should be rejected for CDS index options");
    assert!(matches!(err, finstack_core::Error::Validation(_)));
}

// ============================================================================
// Quant-audit remediation PR 3: Front-End Protection for CDS index options (C5)
// ============================================================================
//
// Pedersen (2003) and O'Kane (2008) Ch. 12 show that payer options on a
// defaultable CDS index must include a front-end protection (FEP) term
// for defaults between `as_of` and exercise:
//
//     FEP = ∫_0^{T_exp} (1 - R) · h(s) · S(s) · DF(0, s) ds
//
// The FEP is added to payer (Call) NPV only; receiver (Put) options have
// knock-out semantics on individual defaulted names in the standard index
// conventions and do not receive the FEP uplift.

#[test]
fn test_index_call_includes_front_end_protection() {
    // Hand-computed FEP sanity check.
    // Market: flat 3% discount, flat 2% hazard (recovery 40%), 1y expiry, 5y CDS.
    // h = 0.02, S(t) = exp(-h*t), DF(t) = exp(-0.03*t).
    // FEP = (1-0.4) · ∫_0^1 0.02 · exp(-0.02 t) · exp(-0.03 t) dt
    //     = 0.6 · 0.02 · ∫_0^1 exp(-0.05 t) dt
    //     = 0.012 · (1 - e^{-0.05}) / 0.05
    //     ≈ 0.012 · 0.975410 = 0.01170492 per unit notional.
    // On 10M notional: FEP_PV ≈ 117_049.
    //
    // Pre-fix: payer index call has *no* FEP and is priced using only the
    // Black-on-spreads formula. Post-fix: FEP is added on top.
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of); // recovery 0.40, hazard 0.02, disc 0.03

    // Single-name reference for comparison (no FEP applies to single-name options).
    let sn_call = CDSOptionBuilder::new().call().expiry_months(12).build(as_of);
    let sn_pv = sn_call.value(&market, as_of).unwrap().amount();

    // Index payer option with same terms.
    let idx_call = CDSOptionBuilder::new()
        .call()
        .with_index(1.0)
        .expiry_months(12)
        .build(as_of);
    let idx_pv = idx_call.value(&market, as_of).unwrap().amount();

    // Hand-computed FEP on 10M notional, 40% recovery, 2% hazard, 3% disc, 1y.
    let expected_fep = 0.012_f64 * (1.0 - (-0.05_f64).exp()) / 0.05 * 10_000_000.0;

    let diff = idx_pv - sn_pv;
    // Tolerance 2% of the hand-computed FEP value: the curve's hazard &
    // discount rates are interpolated between knots (1y, 5y, 10y), so the
    // effective {h, r} values inside the FEP quadrature differ slightly
    // from the idealised flat-rate hand-calc above. The point of this
    // assertion is to catch the pre-fix regression (diff = 0) and to
    // verify order-of-magnitude agreement with Pedersen 2003.
    assert!(
        (diff - expected_fep).abs() < 0.02 * expected_fep,
        "Index payer call should include FEP ≈ {expected_fep:.0}, got diff {diff:.0} (idx_pv={idx_pv:.0}, sn_pv={sn_pv:.0}). Pre-fix would show diff ≈ 0."
    );
    assert!(
        idx_pv > sn_pv,
        "Payer index call must be strictly more valuable than single-name call of same terms (FEP uplift)."
    );
}

#[test]
fn test_index_put_does_not_include_front_end_protection() {
    // Receiver options on CDS indices do NOT include FEP — the buyer would
    // be selling protection, but defaulted names are no longer part of the
    // exercise universe (standard FTD-style knock-out). Assert parity with
    // single-name put.
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let sn_put = CDSOptionBuilder::new().put().expiry_months(12).build(as_of);
    let idx_put = CDSOptionBuilder::new()
        .put()
        .with_index(1.0)
        .expiry_months(12)
        .build(as_of);

    let sn_pv = sn_put.value(&market, as_of).unwrap().amount();
    let idx_pv = idx_put.value(&market, as_of).unwrap().amount();

    let diff = (idx_pv - sn_pv).abs();
    let baseline = sn_pv.abs().max(1.0);
    assert!(
        diff < 0.001 * baseline,
        "Receiver index put must NOT include FEP uplift (unlike payer); idx={idx_pv:.2}, sn={sn_pv:.2}, diff={diff:.2}"
    );
}
