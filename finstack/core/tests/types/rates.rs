//! Additional tests for rate types and conversions

use finstack_core::types::{Bps, Percentage, Rate};

#[test]
fn rate_conversions_roundtrip() {
    // Test decimal -> bps -> decimal
    let rate = Rate::from_decimal(0.0525);
    assert_eq!(rate.as_bps(), 525);
    let rate2 = Rate::from_bps(525);
    assert!((rate.as_decimal() - rate2.as_decimal()).abs() < 1e-10);

    // Test percent -> bps -> percent
    let rate = Rate::from_percent(5.25);
    assert_eq!(rate.as_bps(), 525);
    assert!((rate.as_percent() - 5.25).abs() < 1e-10);
}

#[test]
fn rate_arithmetic_operations() {
    let r1 = Rate::from_percent(3.0);
    let r2 = Rate::from_percent(1.5);

    // Addition
    let sum = r1 + r2;
    assert!((sum.as_percent() - 4.5).abs() < 1e-10);

    // Subtraction
    let diff = r1 - r2;
    assert!((diff.as_percent() - 1.5).abs() < 1e-10);

    // Multiplication
    let doubled = r1 * 2.0;
    assert!((doubled.as_percent() - 6.0).abs() < 1e-10);

    // Division
    let halved = r1 / 2.0;
    assert!((halved.as_percent() - 1.5).abs() < 1e-10);

    // Negation
    let neg = -r1;
    assert!((neg.as_percent() + 3.0).abs() < 1e-10);
}

#[test]
fn rate_predicates() {
    assert!(Rate::ZERO.is_zero());
    assert!(!Rate::ZERO.is_positive());
    assert!(!Rate::ZERO.is_negative());

    let positive = Rate::from_percent(2.5);
    assert!(!positive.is_zero());
    assert!(positive.is_positive());
    assert!(!positive.is_negative());

    let negative = Rate::from_percent(-1.5);
    assert!(!negative.is_zero());
    assert!(!negative.is_positive());
    assert!(negative.is_negative());
}

#[test]
fn rate_abs() {
    let negative = Rate::from_percent(-3.5);
    let abs_val = negative.abs();
    assert!((abs_val.as_percent() - 3.5).abs() < 1e-10);

    let positive = Rate::from_percent(2.5);
    let abs_val = positive.abs();
    assert_eq!(abs_val.as_percent(), 2.5);
}

#[test]
fn rate_display_formatting() {
    let rate = Rate::from_percent(2.5);
    assert_eq!(format!("{}", rate), "2.5000%");

    let rate = Rate::from_bps(150);
    assert_eq!(format!("{}", rate), "1.5000%");
}

#[test]
fn rate_from_f64_conversion() {
    let r: Rate = 0.035.into();
    assert!((r.as_decimal() - 0.035).abs() < 1e-10);

    let f: f64 = r.into();
    assert!((f - 0.035).abs() < 1e-10);
}

#[test]
fn bps_conversions_comprehensive() {
    let bps = Bps::new(250);

    assert_eq!(bps.as_bps(), 250);
    assert!((bps.as_decimal() - 0.025).abs() < 1e-10);
    assert!((bps.as_percent() - 2.5).abs() < 1e-10);

    let rate = bps.as_rate();
    assert_eq!(rate.as_bps(), 250);
}

#[test]
fn bps_arithmetic_operations() {
    let b1 = Bps::new(100);
    let b2 = Bps::new(50);

    assert_eq!(b1 + b2, Bps::new(150));
    assert_eq!(b1 - b2, Bps::new(50));
    assert_eq!(b1 * 3, Bps::new(300));
    assert_eq!(b1 / 2, Bps::new(50));
    assert_eq!(-b1, Bps::new(-100));
}

#[test]
fn bps_predicates() {
    assert!(Bps::ZERO.is_zero());
    assert!(!Bps::ZERO.is_positive());
    assert!(!Bps::ZERO.is_negative());

    let positive = Bps::new(100);
    assert!(positive.is_positive());
    assert!(!positive.is_negative());

    let negative = Bps::new(-50);
    assert!(negative.is_negative());
    assert!(!negative.is_positive());
}

#[test]
fn bps_abs() {
    let negative = Bps::new(-150);
    assert_eq!(negative.abs(), Bps::new(150));

    let positive = Bps::new(100);
    assert_eq!(positive.abs(), Bps::new(100));
}

#[test]
fn bps_display_formatting() {
    assert_eq!(format!("{}", Bps::new(250)), "250bp");
    assert_eq!(format!("{}", Bps::new(-50)), "-50bp");
}

#[test]
fn bps_from_conversions() {
    let b: Bps = 250.into();
    assert_eq!(b.as_bps(), 250);

    let i: i32 = b.into();
    assert_eq!(i, 250);

    let f: f64 = b.into();
    assert!((f - 0.025).abs() < 1e-10);
}

#[test]
fn percentage_conversions() {
    let pct = Percentage::new(12.5);

    assert_eq!(pct.as_percent(), 12.5);
    assert!((pct.as_decimal() - 0.125).abs() < 1e-10);
    assert_eq!(pct.as_bps(), 1250);

    let rate = pct.as_rate();
    assert!((rate.as_percent() - 12.5).abs() < 1e-10);
}

#[test]
fn percentage_arithmetic_operations() {
    let p1 = Percentage::new(10.0);
    let p2 = Percentage::new(5.0);

    assert_eq!(p1 + p2, Percentage::new(15.0));
    assert_eq!(p1 - p2, Percentage::new(5.0));
    assert_eq!(p1 * 2.0, Percentage::new(20.0));
    assert_eq!(p1 / 2.0, Percentage::new(5.0));
    assert_eq!(-p1, Percentage::new(-10.0));
}

#[test]
fn percentage_predicates() {
    assert!(Percentage::ZERO.is_zero());
    assert!(!Percentage::ZERO.is_positive());
    assert!(!Percentage::ZERO.is_negative());

    let positive = Percentage::new(15.5);
    assert!(positive.is_positive());
    assert!(!positive.is_negative());

    let negative = Percentage::new(-5.0);
    assert!(negative.is_negative());
    assert!(!negative.is_positive());
}

#[test]
fn percentage_abs() {
    let negative = Percentage::new(-7.5);
    assert_eq!(negative.abs(), Percentage::new(7.5));

    let positive = Percentage::new(3.5);
    assert_eq!(positive.abs(), Percentage::new(3.5));
}

#[test]
fn percentage_display_formatting() {
    let pct = Percentage::new(12.75);
    assert_eq!(format!("{}", pct), "12.75%");

    let pct = Percentage::new(-5.5);
    assert_eq!(format!("{}", pct), "-5.50%");
}

#[test]
fn percentage_from_conversions() {
    let p: Percentage = 15.0.into();
    assert_eq!(p.as_percent(), 15.0);

    let f: f64 = p.into();
    assert_eq!(f, 15.0);
}

#[test]
fn cross_type_conversions() {
    // Rate <-> Bps
    let rate = Rate::from_percent(2.5);
    let bps: Bps = rate.into();
    assert_eq!(bps.as_bps(), 250);
    let rate_back: Rate = bps.into();
    assert_eq!(rate.as_bps(), rate_back.as_bps());

    // Rate <-> Percentage
    let rate = Rate::from_percent(3.5);
    let pct: Percentage = rate.into();
    assert!((pct.as_percent() - 3.5).abs() < 1e-10);
    let rate_back: Rate = pct.into();
    assert!((rate.as_decimal() - rate_back.as_decimal()).abs() < 1e-10);

    // Bps <-> Percentage
    let bps = Bps::new(350);
    let pct: Percentage = bps.into();
    assert_eq!(pct.as_percent(), 3.5);
    let bps_back: Bps = pct.into();
    assert_eq!(bps.as_bps(), bps_back.as_bps());
}

#[test]
fn rate_edge_cases() {
    // Very small rates
    let tiny = Rate::from_bps(1);
    assert_eq!(tiny.as_bps(), 1);
    assert!((tiny.as_decimal() - 0.0001).abs() < 1e-10);

    // Large rates
    let large = Rate::from_percent(100.0);
    assert_eq!(large.as_bps(), 10000);
    assert!((large.as_decimal() - 1.0).abs() < 1e-10);

    // Negative rates
    let negative = Rate::from_bps(-50);
    assert_eq!(negative.as_bps(), -50);
    assert!(negative.is_negative());
}

#[test]
fn bps_ordering() {
    let b1 = Bps::new(100);
    let b2 = Bps::new(200);
    let b3 = Bps::new(100);

    assert!(b1 < b2);
    assert!(b2 > b1);
    assert_eq!(b1, b3);
    assert!(b1 <= b3);
    assert!(b1 >= b3);
}

#[test]
fn percentage_ordering() {
    let p1 = Percentage::new(5.0);
    let p2 = Percentage::new(10.0);
    let p3 = Percentage::new(5.0);

    assert!(p1 < p2);
    assert!(p2 > p1);
    assert_eq!(p1, p3);
    assert!(p1 <= p3);
    assert!(p1 >= p3);
}

#[test]
fn rate_ordering() {
    let r1 = Rate::from_percent(2.0);
    let r2 = Rate::from_percent(3.0);
    let r3 = Rate::from_percent(2.0);

    assert!(r1 < r2);
    assert!(r2 > r1);
    assert_eq!(r1, r3);
}
