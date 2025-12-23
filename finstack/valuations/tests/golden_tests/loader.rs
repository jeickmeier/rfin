//! Golden test case loader and validator.
//!
//! Loads test vectors from CSV files for validating pricing implementations
//! against known reference values (QuantLib, Bloomberg, analytical formulas).

use std::path::Path;

/// A single golden test case for European options.
#[derive(Clone, Debug)]
pub struct GoldenTestCase {
    /// Test case name/description
    pub name: String,
    /// Current spot price
    pub spot: f64,
    /// Strike price
    pub strike: f64,
    /// Time to maturity (years)
    pub time: f64,
    /// Risk-free rate (annual, continuous)
    pub rate: f64,
    /// Dividend yield (annual, continuous)
    pub div_yield: f64,
    /// Volatility (annual)
    pub volatility: f64,
    /// Expected price (from reference source)
    pub expected_price: f64,
    /// Absolute tolerance
    pub abs_tolerance: f64,
    /// Relative tolerance (as decimal, e.g., 0.001 for 0.1%)
    pub rel_tolerance: f64,
    /// Option type (call or put)
    pub option_type: OptionType,
}

/// Option type for golden tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionType {
    Call,
    Put,
}

impl std::str::FromStr for OptionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "call" => Ok(OptionType::Call),
            "put" => Ok(OptionType::Put),
            other => Err(format!("Unknown option type: '{}'", other)),
        }
    }
}

/// A golden test case for barrier options.
#[derive(Clone, Debug)]
pub struct BarrierTestCase {
    /// Test case name/description
    pub name: String,
    /// Current spot price
    pub spot: f64,
    /// Strike price
    pub strike: f64,
    /// Barrier level
    pub barrier: f64,
    /// Time to maturity (years)
    pub time: f64,
    /// Risk-free rate
    pub rate: f64,
    /// Dividend yield
    pub div_yield: f64,
    /// Volatility
    pub volatility: f64,
    /// Barrier type
    pub barrier_type: BarrierType,
    /// Expected price
    pub expected_price: f64,
    /// Absolute tolerance
    pub abs_tolerance: f64,
    /// Relative tolerance
    pub rel_tolerance: f64,
}

/// Barrier type for golden tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarrierType {
    UpOut,
    UpIn,
    DownOut,
    DownIn,
}

impl std::str::FromStr for BarrierType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "up_out" | "upout" => Ok(BarrierType::UpOut),
            "up_in" | "upin" => Ok(BarrierType::UpIn),
            "down_out" | "downout" => Ok(BarrierType::DownOut),
            "down_in" | "downin" => Ok(BarrierType::DownIn),
            other => Err(format!("Unknown barrier type: '{}'", other)),
        }
    }
}

/// A golden test case for Asian options.
#[derive(Clone, Debug)]
pub struct AsianTestCase {
    /// Test case name/description
    pub name: String,
    /// Current spot price
    pub spot: f64,
    /// Strike price
    pub strike: f64,
    /// Time to maturity (years)
    pub time: f64,
    /// Risk-free rate
    pub rate: f64,
    /// Dividend yield
    pub div_yield: f64,
    /// Volatility
    pub volatility: f64,
    /// Number of fixings
    pub num_fixings: usize,
    /// Averaging type
    pub averaging: AveragingType,
    /// Expected price
    pub expected_price: f64,
    /// Absolute tolerance
    pub abs_tolerance: f64,
    /// Relative tolerance
    pub rel_tolerance: f64,
}

/// Averaging type for Asian options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AveragingType {
    Geometric,
    Arithmetic,
}

impl std::str::FromStr for AveragingType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "geometric" | "geom" => Ok(AveragingType::Geometric),
            "arithmetic" | "arith" => Ok(AveragingType::Arithmetic),
            other => Err(format!("Unknown averaging type: '{}'", other)),
        }
    }
}

/// Load European option golden test cases from CSV file.
///
/// # CSV Format
///
/// ```csv
/// name,spot,strike,time,rate,div_yield,volatility,expected_price,abs_tol,rel_tol,option_type
/// ATM_1Y,100,100,1.0,0.05,0.02,0.20,8.916,0.05,0.005,call
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read
/// - CSV parsing fails
/// - Required columns are missing
pub fn load_golden_tests<P: AsRef<Path>>(path: P) -> Result<Vec<GoldenTestCase>, String> {
    let path = path.as_ref();

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_path(path)
        .map_err(|e| format!("Failed to open CSV file '{}': {}", path.display(), e))?;

    let mut cases = Vec::new();

    for (idx, result) in reader.records().enumerate() {
        let record = result
            .map_err(|e| format!("Failed to parse row {}: {}", idx + 2, e))?;

        // Skip empty rows
        if record.iter().all(|f| f.trim().is_empty()) {
            continue;
        }

        let name = record.get(0)
            .ok_or_else(|| format!("Row {}: missing 'name' column", idx + 2))?
            .to_string();

        // Skip if name is empty (handles trailing newlines)
        if name.trim().is_empty() {
            continue;
        }

        let spot: f64 = record.get(1)
            .ok_or_else(|| format!("Row {}: missing 'spot' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'spot': {}", idx + 2, e))?;

        let strike: f64 = record.get(2)
            .ok_or_else(|| format!("Row {}: missing 'strike' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'strike': {}", idx + 2, e))?;

        let time: f64 = record.get(3)
            .ok_or_else(|| format!("Row {}: missing 'time' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'time': {}", idx + 2, e))?;

        let rate: f64 = record.get(4)
            .ok_or_else(|| format!("Row {}: missing 'rate' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'rate': {}", idx + 2, e))?;

        let div_yield: f64 = record.get(5)
            .ok_or_else(|| format!("Row {}: missing 'div_yield' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'div_yield': {}", idx + 2, e))?;

        let volatility: f64 = record.get(6)
            .ok_or_else(|| format!("Row {}: missing 'volatility' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'volatility': {}", idx + 2, e))?;

        let expected_price: f64 = record.get(7)
            .ok_or_else(|| format!("Row {}: missing 'expected_price' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'expected_price': {}", idx + 2, e))?;

        let abs_tolerance: f64 = record.get(8)
            .ok_or_else(|| format!("Row {}: missing 'abs_tol' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'abs_tol': {}", idx + 2, e))?;

        let rel_tolerance: f64 = record.get(9)
            .ok_or_else(|| format!("Row {}: missing 'rel_tol' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'rel_tol': {}", idx + 2, e))?;

        let option_type: OptionType = record.get(10)
            .ok_or_else(|| format!("Row {}: missing 'option_type' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'option_type': {}", idx + 2, e))?;

        cases.push(GoldenTestCase {
            name,
            spot,
            strike,
            time,
            rate,
            div_yield,
            volatility,
            expected_price,
            abs_tolerance,
            rel_tolerance,
            option_type,
        });
    }

    Ok(cases)
}

/// Load barrier option golden test cases from CSV file.
pub fn load_barrier_tests<P: AsRef<Path>>(path: P) -> Result<Vec<BarrierTestCase>, String> {
    let path = path.as_ref();

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_path(path)
        .map_err(|e| format!("Failed to open CSV file '{}': {}", path.display(), e))?;

    let mut cases = Vec::new();

    for (idx, result) in reader.records().enumerate() {
        let record = result
            .map_err(|e| format!("Failed to parse row {}: {}", idx + 2, e))?;

        // Skip empty rows
        if record.iter().all(|f| f.trim().is_empty()) {
            continue;
        }

        let name = record.get(0)
            .ok_or_else(|| format!("Row {}: missing 'name' column", idx + 2))?
            .to_string();

        if name.trim().is_empty() {
            continue;
        }

        let spot: f64 = record.get(1)
            .ok_or_else(|| format!("Row {}: missing 'spot' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'spot': {}", idx + 2, e))?;

        let strike: f64 = record.get(2)
            .ok_or_else(|| format!("Row {}: missing 'strike' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'strike': {}", idx + 2, e))?;

        let barrier: f64 = record.get(3)
            .ok_or_else(|| format!("Row {}: missing 'barrier' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'barrier': {}", idx + 2, e))?;

        let time: f64 = record.get(4)
            .ok_or_else(|| format!("Row {}: missing 'time' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'time': {}", idx + 2, e))?;

        let rate: f64 = record.get(5)
            .ok_or_else(|| format!("Row {}: missing 'rate' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'rate': {}", idx + 2, e))?;

        let div_yield: f64 = record.get(6)
            .ok_or_else(|| format!("Row {}: missing 'div_yield' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'div_yield': {}", idx + 2, e))?;

        let volatility: f64 = record.get(7)
            .ok_or_else(|| format!("Row {}: missing 'volatility' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'volatility': {}", idx + 2, e))?;

        let barrier_type: BarrierType = record.get(8)
            .ok_or_else(|| format!("Row {}: missing 'barrier_type' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'barrier_type': {}", idx + 2, e))?;

        let expected_price: f64 = record.get(9)
            .ok_or_else(|| format!("Row {}: missing 'expected_price' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'expected_price': {}", idx + 2, e))?;

        let abs_tolerance: f64 = record.get(10)
            .ok_or_else(|| format!("Row {}: missing 'abs_tol' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'abs_tol': {}", idx + 2, e))?;

        let rel_tolerance: f64 = record.get(11)
            .ok_or_else(|| format!("Row {}: missing 'rel_tol' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'rel_tol': {}", idx + 2, e))?;

        cases.push(BarrierTestCase {
            name,
            spot,
            strike,
            barrier,
            time,
            rate,
            div_yield,
            volatility,
            barrier_type,
            expected_price,
            abs_tolerance,
            rel_tolerance,
        });
    }

    Ok(cases)
}

/// Load Asian option golden test cases from CSV file.
pub fn load_asian_tests<P: AsRef<Path>>(path: P) -> Result<Vec<AsianTestCase>, String> {
    let path = path.as_ref();

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_path(path)
        .map_err(|e| format!("Failed to open CSV file '{}': {}", path.display(), e))?;

    let mut cases = Vec::new();

    for (idx, result) in reader.records().enumerate() {
        let record = result
            .map_err(|e| format!("Failed to parse row {}: {}", idx + 2, e))?;

        // Skip empty rows
        if record.iter().all(|f| f.trim().is_empty()) {
            continue;
        }

        let name = record.get(0)
            .ok_or_else(|| format!("Row {}: missing 'name' column", idx + 2))?
            .to_string();

        if name.trim().is_empty() {
            continue;
        }

        let spot: f64 = record.get(1)
            .ok_or_else(|| format!("Row {}: missing 'spot' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'spot': {}", idx + 2, e))?;

        let strike: f64 = record.get(2)
            .ok_or_else(|| format!("Row {}: missing 'strike' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'strike': {}", idx + 2, e))?;

        let time: f64 = record.get(3)
            .ok_or_else(|| format!("Row {}: missing 'time' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'time': {}", idx + 2, e))?;

        let rate: f64 = record.get(4)
            .ok_or_else(|| format!("Row {}: missing 'rate' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'rate': {}", idx + 2, e))?;

        let div_yield: f64 = record.get(5)
            .ok_or_else(|| format!("Row {}: missing 'div_yield' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'div_yield': {}", idx + 2, e))?;

        let volatility: f64 = record.get(6)
            .ok_or_else(|| format!("Row {}: missing 'volatility' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'volatility': {}", idx + 2, e))?;

        let num_fixings: usize = record.get(7)
            .ok_or_else(|| format!("Row {}: missing 'num_fixings' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'num_fixings': {}", idx + 2, e))?;

        let averaging: AveragingType = record.get(8)
            .ok_or_else(|| format!("Row {}: missing 'averaging' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'averaging': {}", idx + 2, e))?;

        let expected_price: f64 = record.get(9)
            .ok_or_else(|| format!("Row {}: missing 'expected_price' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'expected_price': {}", idx + 2, e))?;

        let abs_tolerance: f64 = record.get(10)
            .ok_or_else(|| format!("Row {}: missing 'abs_tol' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'abs_tol': {}", idx + 2, e))?;

        let rel_tolerance: f64 = record.get(11)
            .ok_or_else(|| format!("Row {}: missing 'rel_tol' column", idx + 2))?
            .parse()
            .map_err(|e| format!("Row {}: invalid 'rel_tol': {}", idx + 2, e))?;

        cases.push(AsianTestCase {
            name,
            spot,
            strike,
            time,
            rate,
            div_yield,
            volatility,
            num_fixings,
            averaging,
            expected_price,
            abs_tolerance,
            rel_tolerance,
        });
    }

    Ok(cases)
}

/// Assert that a calculated result is within tolerance of expected value.
///
/// Uses both absolute and relative tolerances, plus Monte Carlo confidence bounds.
///
/// # Arguments
///
/// * `test_case` - The golden test case with expected values and tolerances
/// * `mc_price` - The calculated price (from MC or analytical)
/// * `mc_stderr` - Standard error (for MC results; use 0.0 for analytical)
pub fn assert_within_tolerance(
    test_case: &GoldenTestCase,
    mc_price: f64,
    mc_stderr: f64,
) {
    let diff = (mc_price - test_case.expected_price).abs();
    let rel_diff = if test_case.expected_price.abs() > 1e-10 {
        diff / test_case.expected_price.abs()
    } else {
        0.0
    };

    // Check absolute tolerance
    let abs_ok = diff < test_case.abs_tolerance;

    // Check relative tolerance
    let rel_ok = rel_diff < test_case.rel_tolerance;

    // Check within MC confidence bounds (4σ)
    let mc_ok = mc_stderr > 0.0 && diff < 4.0 * mc_stderr;

    println!(
        "  {}: Calculated={:.4}, Expected={:.4}, Diff={:.4}, RelDiff={:.2}%, Stderr={:.4}",
        test_case.name,
        mc_price,
        test_case.expected_price,
        diff,
        rel_diff * 100.0,
        mc_stderr
    );

    assert!(
        abs_ok || rel_ok || mc_ok,
        "Test case '{}' failed: Calculated={:.6}, Expected={:.6}, Diff={:.6} \
         (abs_tol={:.6}, rel_tol={:.2}%, 4σ={:.6})",
        test_case.name,
        mc_price,
        test_case.expected_price,
        diff,
        test_case.abs_tolerance,
        test_case.rel_tolerance * 100.0,
        4.0 * mc_stderr
    );
}

/// Get the path to the golden test data directory.
///
/// Returns the path relative to the workspace root.
pub fn golden_data_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden_tests")
        .join("data")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_european_options_csv() {
        let path = golden_data_dir().join("european_options.csv");
        let cases = load_golden_tests(&path).expect("Failed to load European options");

        assert!(!cases.is_empty(), "Should load at least one test case");

        // Check first case
        let first = &cases[0];
        assert_eq!(first.name, "BS_ATM_1Y_Call");
        assert_eq!(first.spot, 100.0);
        assert_eq!(first.strike, 100.0);
        assert_eq!(first.time, 1.0);
        assert_eq!(first.option_type, OptionType::Call);

        // Check we have both calls and puts
        let has_call = cases.iter().any(|c| c.option_type == OptionType::Call);
        let has_put = cases.iter().any(|c| c.option_type == OptionType::Put);
        assert!(has_call, "Should have call options");
        assert!(has_put, "Should have put options");
    }

    #[test]
    fn test_load_barrier_options_csv() {
        let path = golden_data_dir().join("barrier_options.csv");
        let cases = load_barrier_tests(&path).expect("Failed to load barrier options");

        assert!(!cases.is_empty(), "Should load at least one test case");

        // Check first case
        let first = &cases[0];
        assert_eq!(first.name, "Barrier_UpOut_ATM");
        assert_eq!(first.barrier, 120.0);
        assert_eq!(first.barrier_type, BarrierType::UpOut);

        // Check we have different barrier types
        let barrier_types: Vec<_> = cases.iter().map(|c| c.barrier_type).collect();
        assert!(barrier_types.contains(&BarrierType::UpOut));
        assert!(barrier_types.contains(&BarrierType::UpIn));
        assert!(barrier_types.contains(&BarrierType::DownOut));
        assert!(barrier_types.contains(&BarrierType::DownIn));
    }

    #[test]
    fn test_load_asian_options_csv() {
        let path = golden_data_dir().join("asian_options.csv");
        let cases = load_asian_tests(&path).expect("Failed to load Asian options");

        assert!(!cases.is_empty(), "Should load at least one test case");

        // Check first case
        let first = &cases[0];
        assert_eq!(first.name, "Asian_Geom_ATM_12M");
        assert_eq!(first.num_fixings, 12);
        assert_eq!(first.averaging, AveragingType::Geometric);

        // Check we have both averaging types
        let has_geom = cases.iter().any(|c| c.averaging == AveragingType::Geometric);
        let has_arith = cases.iter().any(|c| c.averaging == AveragingType::Arithmetic);
        assert!(has_geom, "Should have geometric averaging");
        assert!(has_arith, "Should have arithmetic averaging");
    }

    #[test]
    fn test_assert_within_tolerance_pass() {
        let case = GoldenTestCase {
            name: "Test".to_string(),
            spot: 100.0,
            strike: 100.0,
            time: 1.0,
            rate: 0.05,
            div_yield: 0.02,
            volatility: 0.20,
            expected_price: 10.0,
            abs_tolerance: 0.5,
            rel_tolerance: 0.01,
            option_type: OptionType::Call,
        };

        // Within absolute tolerance
        assert_within_tolerance(&case, 10.2, 0.01);

        // Within relative tolerance
        assert_within_tolerance(&case, 10.05, 0.001);
    }

    #[test]
    #[should_panic]
    fn test_assert_within_tolerance_fail() {
        let case = GoldenTestCase {
            name: "Test".to_string(),
            spot: 100.0,
            strike: 100.0,
            time: 1.0,
            rate: 0.05,
            div_yield: 0.02,
            volatility: 0.20,
            expected_price: 10.0,
            abs_tolerance: 0.1,
            rel_tolerance: 0.001,
            option_type: OptionType::Call,
        };

        // Outside all tolerances
        assert_within_tolerance(&case, 12.0, 0.01);
    }

    #[test]
    fn test_option_type_parsing() {
        assert_eq!("call".parse::<OptionType>().unwrap(), OptionType::Call);
        assert_eq!("Call".parse::<OptionType>().unwrap(), OptionType::Call);
        assert_eq!("CALL".parse::<OptionType>().unwrap(), OptionType::Call);
        assert_eq!("put".parse::<OptionType>().unwrap(), OptionType::Put);
        assert_eq!("Put".parse::<OptionType>().unwrap(), OptionType::Put);
        assert!("unknown".parse::<OptionType>().is_err());
    }

    #[test]
    fn test_barrier_type_parsing() {
        assert_eq!("up_out".parse::<BarrierType>().unwrap(), BarrierType::UpOut);
        assert_eq!("upout".parse::<BarrierType>().unwrap(), BarrierType::UpOut);
        assert_eq!("down_in".parse::<BarrierType>().unwrap(), BarrierType::DownIn);
        assert!("unknown".parse::<BarrierType>().is_err());
    }

    #[test]
    fn test_averaging_type_parsing() {
        assert_eq!("geometric".parse::<AveragingType>().unwrap(), AveragingType::Geometric);
        assert_eq!("geom".parse::<AveragingType>().unwrap(), AveragingType::Geometric);
        assert_eq!("arithmetic".parse::<AveragingType>().unwrap(), AveragingType::Arithmetic);
        assert_eq!("arith".parse::<AveragingType>().unwrap(), AveragingType::Arithmetic);
        assert!("unknown".parse::<AveragingType>().is_err());
    }
}
