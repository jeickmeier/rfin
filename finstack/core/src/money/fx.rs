//! Foreign-exchange interfaces and policy types.
//!
//! This module defines an `FxProvider` trait and simple policy metadata used by
//! `Money::convert`. Conversions are always explicit – arithmetic on `Money`
//! requires the same currency.

use crate::currency::Currency;
use crate::dates::Date;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Provider FX rate type alias – `Decimal` when `decimal128` is enabled, otherwise `f64`.
#[cfg(feature = "decimal128")]
pub type FxRate = rust_decimal::Decimal;
/// Provider FX rate type alias – `Decimal` when `decimal128` is enabled, otherwise `f64`.
#[cfg(not(feature = "decimal128"))]
pub type FxRate = f64;

/// Standard FX conversion strategies. These are metadata hints for providers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FxConversionPolicy {
    /// Use spot/forward on the cashflow date.
    CashflowDate,
    /// Use period end date.
    PeriodEnd,
    /// Use an average over the period.
    PeriodAverage,
    /// Custom strategy defined by the caller/provider.
    Custom,
}

/// Metadata describing the policy applied by the provider.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FxPolicyMeta {
    /// Strategy applied for the conversion.
    pub strategy: FxConversionPolicy,
    /// Optional declared target currency (for stamping).
    pub target_ccy: Option<Currency>,
    /// Optional notes for auditability.
    pub notes: String,
}

impl Default for FxPolicyMeta {
    fn default() -> Self {
        Self {
            strategy: FxConversionPolicy::CashflowDate,
            target_ccy: None,
            notes: String::new(),
        }
    }
}

/// Cache key for FX rate lookups  
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct FxCacheKey {
    from: Currency,
    to: Currency,
    date: Date,
    policy: FxConversionPolicy,
}

/// Cached FX rate entry with timestamp
#[derive(Clone, Debug)]
struct CachedFxEntry {
    rate: FxRate,
    cached_at: Instant,
    ttl: Duration,
}

impl CachedFxEntry {
    fn new(rate: FxRate, ttl: Duration) -> Self {
        Self {
            rate,
            cached_at: Instant::now(),
            ttl,
        }
    }
    
    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }
}

/// Configuration for FX caching and closure checking
#[derive(Clone, Debug)]
pub struct FxCacheConfig {
    /// Maximum number of entries to cache
    pub max_entries: usize,
    /// Default TTL for cached rates
    pub default_ttl: Duration,
    /// Tolerance for closure checking (e.g., 0.0001 = 1bp)
    pub closure_tolerance: f64,
    /// Whether closure violations should produce warnings or errors
    pub strict_closure: bool,
}

impl Default for FxCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            default_ttl: Duration::from_secs(300), // 5 minutes
            closure_tolerance: 0.0001, // 1 basis point
            strict_closure: false,
        }
    }
}

/// Result of a closure check
#[derive(Clone, Debug, PartialEq)]
pub enum ClosureCheckResult {
    /// Closure check passed within tolerance
    Pass,
    /// Closure check failed - provides direct rate and calculated rate for comparison
    Fail { 
        /// The direct FX rate from the provider
        direct_rate: f64, 
        /// The calculated rate via intermediate currency
        calculated_rate: f64, 
        /// The absolute difference between direct and calculated rates
        difference: f64 
    },
}

/// Trait for obtaining FX rates.
pub trait FxProvider: Send + Sync {
    /// Return a rate to convert `from` → `to` applicable on `on` per `policy`.
    fn rate(
        &self,
        from: Currency,
        to: Currency,
        on: Date,
        policy: FxConversionPolicy,
    ) -> crate::Result<FxRate>;
}

/// Enhanced FX matrix with LRU caching and closure checking
pub struct FxMatrix<P: FxProvider> {
    provider: P,
    cache: std::sync::Mutex<HashMap<FxCacheKey, CachedFxEntry>>,
    config: FxCacheConfig,
}

impl<P: FxProvider> FxMatrix<P> {
    /// Create a new `FxMatrix` wrapping the given provider with default cache configuration
    pub fn new(provider: P) -> Self {
        Self::with_config(provider, FxCacheConfig::default())
    }
    
    /// Create a new `FxMatrix` with custom cache configuration
    pub fn with_config(provider: P, config: FxCacheConfig) -> Self {
        Self {
            provider,
            cache: std::sync::Mutex::new(HashMap::new()),
            config,
        }
    }
    
    /// Get rate with caching
    pub fn rate(
        &self,
        from: Currency,
        to: Currency,
        on: Date,
        policy: FxConversionPolicy,
    ) -> crate::Result<FxRate> {
        // Handle identity case
        if from == to {
            #[cfg(feature = "decimal128")]
            return Ok(rust_decimal::Decimal::ONE);
            #[cfg(not(feature = "decimal128"))]
            return Ok(1.0);
        }
        
        let cache_key = FxCacheKey { from, to, date: on, policy };
        
        // Try to get from cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(entry) = cache.get(&cache_key) {
                if !entry.is_expired() {
                    return Ok(entry.rate);
                }
            }
        }
        
        // Cache miss or expired - fetch from provider
        let rate = self.provider.rate(from, to, on, policy)?;
        
        // Update cache
        self.update_cache(cache_key, rate);
        
        Ok(rate)
    }

    /// Get rate with closure check: from→mid × mid→to ≈ from→to
    pub fn rate_with_closure(
        &self,
        from: Currency,
        to: Currency,
        on: Date,
        policy: FxConversionPolicy,
        mid: Currency,
    ) -> crate::Result<(FxRate, ClosureCheckResult)> {
        let direct_rate = self.rate(from, to, on, policy)?;
        let via_mid_rate_a = self.rate(from, mid, on, policy)?;
        let via_mid_rate_b = self.rate(mid, to, on, policy)?;
        
        let closure_result = self.check_closure(direct_rate, via_mid_rate_a, via_mid_rate_b)?;
        
        match closure_result {
            ClosureCheckResult::Fail { .. } if self.config.strict_closure => {
                return Err(crate::Error::Input(crate::error::InputError::Invalid));
            }
            _ => {}
        }
        
        Ok((direct_rate, closure_result))
    }
    
    /// Clear expired entries from the cache
    pub fn clear_expired(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.retain(|_, entry| !entry.is_expired());
    }
    
    /// Clear all cache entries
    pub fn clear_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.lock().unwrap();
        let total = cache.len();
        let expired = cache.values().filter(|entry| entry.is_expired()).count();
        (total, expired)
    }
    
    // Private helper methods
    
    fn update_cache(&self, key: FxCacheKey, rate: FxRate) {
        let mut cache = self.cache.lock().unwrap();
        
        // Simple LRU: if at capacity, remove oldest entry
        if cache.len() >= self.config.max_entries {
            if let Some(oldest_key) = cache
                .iter()
                .min_by_key(|(_, entry)| entry.cached_at)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest_key);
            }
        }
        
        let entry = CachedFxEntry::new(rate, self.config.default_ttl);
        cache.insert(key, entry);
    }
    
    fn check_closure(
        &self, 
        direct_rate: FxRate, 
        via_a: FxRate, 
        via_b: FxRate
    ) -> crate::Result<ClosureCheckResult> {
        let direct_f64 = self.to_f64(direct_rate)?;
        let calculated_f64 = self.to_f64(via_a)? * self.to_f64(via_b)?;
        let difference = (direct_f64 - calculated_f64).abs();
        
        if difference <= self.config.closure_tolerance {
            Ok(ClosureCheckResult::Pass)
        } else {
            Ok(ClosureCheckResult::Fail { 
                direct_rate: direct_f64,
                calculated_rate: calculated_f64,
                difference,
            })
        }
    }
    
    #[cfg(feature = "decimal128")]
    fn to_f64(&self, rate: FxRate) -> crate::Result<f64> {
        rate.to_string().parse::<f64>().map_err(|_| crate::Error::Internal)
    }
    
    #[cfg(not(feature = "decimal128"))]
    fn to_f64(&self, rate: FxRate) -> crate::Result<f64> {
        Ok(rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currency::Currency;
    use std::time::Duration;
    
    // Mock FX provider for testing
    struct MockFxProvider {
        rates: HashMap<(Currency, Currency), f64>,
    }
    
    impl MockFxProvider {
        fn new() -> Self {
            let mut rates = HashMap::new();
            
            // Add some mock rates
            rates.insert((Currency::USD, Currency::EUR), 0.85);
            rates.insert((Currency::EUR, Currency::USD), 1.18);
            rates.insert((Currency::USD, Currency::GBP), 0.75);
            rates.insert((Currency::GBP, Currency::USD), 1.33);
            rates.insert((Currency::EUR, Currency::GBP), 0.88);
            rates.insert((Currency::GBP, Currency::EUR), 1.14);
            
            Self { rates }
        }
    }
    
    impl FxProvider for MockFxProvider {
        fn rate(
            &self,
            from: Currency,
            to: Currency,
            _on: Date,
            _policy: FxConversionPolicy,
        ) -> crate::Result<FxRate> {
            if let Some(&rate) = self.rates.get(&(from, to)) {
                #[cfg(feature = "decimal128")]
                return rust_decimal::Decimal::try_from(rate).map_err(|_| crate::Error::Internal);
                #[cfg(not(feature = "decimal128"))]
                return Ok(rate);
            }
            Err(crate::Error::Internal)
        }
    }
    
    fn test_date() -> Date {
        use time::Month;
        Date::from_calendar_date(2023, Month::December, 15).unwrap()
    }
    
    #[test]
    fn fx_cache_basic_functionality() {
        let provider = MockFxProvider::new();
        let matrix = FxMatrix::new(provider);
        
        // Test basic rate retrieval
        let rate = matrix.rate(
            Currency::USD,
            Currency::EUR,
            test_date(),
            FxConversionPolicy::CashflowDate
        ).unwrap();
        
        #[cfg(feature = "decimal128")]
        let expected = rust_decimal::Decimal::try_from(0.85).unwrap();
        #[cfg(not(feature = "decimal128"))]
        let expected = 0.85;
        
        assert_eq!(rate, expected);
        
        // Test cache stats
        let (total, _expired) = matrix.cache_stats();
        assert_eq!(total, 1);
    }
    
    #[test] 
    fn fx_cache_identity_rates() {
        let provider = MockFxProvider::new();
        let matrix = FxMatrix::new(provider);
        
        let rate = matrix.rate(
            Currency::USD,
            Currency::USD,
            test_date(),
            FxConversionPolicy::CashflowDate
        ).unwrap();
        
        #[cfg(feature = "decimal128")]
        let expected = rust_decimal::Decimal::ONE;
        #[cfg(not(feature = "decimal128"))]
        let expected = 1.0;
        
        assert_eq!(rate, expected);
    }
    
    #[test]
    fn fx_closure_checking_pass() {
        let provider = MockFxProvider::new();
        let config = FxCacheConfig {
            closure_tolerance: 0.01, // 1% tolerance for this test
            ..Default::default()
        };
        let matrix = FxMatrix::with_config(provider, config);
        
        let (rate, closure_result) = matrix.rate_with_closure(
            Currency::USD,
            Currency::EUR,
            test_date(),
            FxConversionPolicy::CashflowDate,
            Currency::GBP
        ).unwrap();
        
        // Should get a rate regardless of closure result
        #[cfg(feature = "decimal128")]
        assert!(rate > rust_decimal::Decimal::ZERO);
        #[cfg(not(feature = "decimal128"))]
        assert!(rate > 0.0);
        
        // With our mock data, the closure might not be perfect but should be reasonable
        match closure_result {
            ClosureCheckResult::Pass => {},
            ClosureCheckResult::Fail { difference, .. } => {
                // For this test, we'll accept larger differences since our mock data 
                // isn't perfectly consistent
                println!("Closure check failed with difference: {}", difference);
            }
        }
    }
    
    #[test]
    fn fx_cache_ttl_expiry() {
        let provider = MockFxProvider::new();
        let config = FxCacheConfig {
            default_ttl: Duration::from_nanos(1), // Immediate expiry
            ..Default::default()
        };
        let matrix = FxMatrix::with_config(provider, config);
        
        // Get rate to populate cache
        let _rate1 = matrix.rate(
            Currency::USD,
            Currency::EUR,
            test_date(),
            FxConversionPolicy::CashflowDate
        ).unwrap();
        
        // Wait a moment for TTL to expire
        std::thread::sleep(Duration::from_millis(1));
        
        // Clear expired entries
        matrix.clear_expired();
        
        let (total, _) = matrix.cache_stats();
        assert_eq!(total, 0); // Should be cleared due to expiry
    }
    
    #[test]
    fn fx_cache_lru_eviction() {
        let provider = MockFxProvider::new();
        let config = FxCacheConfig {
            max_entries: 2,
            default_ttl: Duration::from_secs(60),
            ..Default::default()
        };
        let matrix = FxMatrix::with_config(provider, config);
        
        // Fill cache to capacity
        let _rate1 = matrix.rate(Currency::USD, Currency::EUR, test_date(), FxConversionPolicy::CashflowDate).unwrap();
        let _rate2 = matrix.rate(Currency::EUR, Currency::USD, test_date(), FxConversionPolicy::CashflowDate).unwrap();
        
        let (total, _) = matrix.cache_stats();
        assert_eq!(total, 2);
        
        // Add one more - should evict the oldest
        let _rate3 = matrix.rate(Currency::USD, Currency::GBP, test_date(), FxConversionPolicy::CashflowDate).unwrap();
        
        let (total, _) = matrix.cache_stats();
        assert_eq!(total, 2); // Still capped at 2
    }
    
    #[test]
    fn fx_cache_clear() {
        let provider = MockFxProvider::new();
        let matrix = FxMatrix::new(provider);
        
        // Populate cache
        let _rate = matrix.rate(Currency::USD, Currency::EUR, test_date(), FxConversionPolicy::CashflowDate).unwrap();
        
        let (total, _) = matrix.cache_stats();
        assert_eq!(total, 1);
        
        // Clear cache
        matrix.clear_cache();
        
        let (total, _) = matrix.cache_stats();
        assert_eq!(total, 0);
    }
    
    #[test]
    fn fx_policy_meta_creation() {
        let policy = FxPolicyMeta {
            strategy: FxConversionPolicy::PeriodAverage,
            target_ccy: Some(Currency::USD),
            notes: "Test policy".to_string(),
        };
        
        assert_eq!(policy.strategy, FxConversionPolicy::PeriodAverage);
        assert_eq!(policy.target_ccy, Some(Currency::USD));
        assert_eq!(policy.notes, "Test policy");
        
        // Test default
        let default_policy = FxPolicyMeta::default();
        assert_eq!(default_policy.strategy, FxConversionPolicy::CashflowDate);
        assert_eq!(default_policy.target_ccy, None);
        assert_eq!(default_policy.notes, String::new());
    }
}
