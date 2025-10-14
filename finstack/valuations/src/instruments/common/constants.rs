//! ISDA 2014 standard constants used by the engine
pub mod isda_constants {

    /// Standard recovery rate for senior unsecured (40%)
    pub const STANDARD_RECOVERY_SENIOR: f64 = 0.40;

    /// Standard recovery rate for subordinated (20%)
    pub const STANDARD_RECOVERY_SUB: f64 = 0.20;

    /// Standard integration points per year for protection leg
    pub const STANDARD_INTEGRATION_POINTS: usize = 40;

    /// Standard coupon payment day
    pub const STANDARD_COUPON_DAY: u8 = 20;

    /// Tolerance for numerical calculations
    pub const NUMERICAL_TOLERANCE: f64 = 1e-10;

    /// Business days per year for North America (US markets)
    pub const BUSINESS_DAYS_PER_YEAR_US: f64 = 252.0;

    /// Business days per year for Europe (UK markets)
    pub const BUSINESS_DAYS_PER_YEAR_UK: f64 = 250.0;

    /// Business days per year for Asia (Japan markets)
    pub const BUSINESS_DAYS_PER_YEAR_JP: f64 = 255.0;
}
