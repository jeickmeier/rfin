//! Builder pattern implementation for Repo instruments.

use super::types::{CollateralSpec, Repo, RepoType};
use crate::instruments::traits::Attributes;
use finstack_core::prelude::*;
use finstack_core::F;

/// Builder for constructing Repo instruments.
#[derive(Debug)]
pub struct RepoBuilder {
    id: Option<String>,
    cash_amount: Option<Money>,
    collateral: Option<CollateralSpec>,
    repo_rate: Option<F>,
    start_date: Option<Date>,
    maturity: Option<Date>,
    haircut: F,
    repo_type: RepoType,
    triparty: bool,
    day_count: DayCount,
    bdc: BusinessDayConvention,
    calendar_id: Option<&'static str>,
    disc_id: Option<&'static str>,
    attributes: Attributes,
}

impl Default for RepoBuilder {
    fn default() -> Self {
        Self {
            haircut: 0.02, // Default 2% haircut
            repo_type: RepoType::Term,
            triparty: false,
            day_count: DayCount::Act360, // Standard for repos
            bdc: BusinessDayConvention::Following,
            calendar_id: Some("target2"), // ECB TARGET2 standard for EUR repos
            attributes: Attributes::default(),
            id: None,
            cash_amount: None,
            collateral: None,
            repo_rate: None,
            start_date: None,
            maturity: None,
            disc_id: None,
        }
    }
}

impl RepoBuilder {
    /// Create a new repo builder with sensible defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the instrument identifier.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the cash amount being lent/borrowed.
    pub fn cash_amount(mut self, amount: Money) -> Self {
        self.cash_amount = Some(amount);
        self
    }

    /// Set the collateral specification.
    pub fn collateral(mut self, collateral: CollateralSpec) -> Self {
        self.collateral = Some(collateral);
        self
    }

    /// Set the repo rate (annual, as decimal).
    pub fn repo_rate(mut self, rate: F) -> Self {
        self.repo_rate = Some(rate);
        self
    }

    /// Set start and end dates.
    pub fn dates(mut self, start: Date, maturity: Date) -> Self {
        self.start_date = Some(start);
        self.maturity = Some(maturity);
        self
    }

    /// Set the haircut percentage (as decimal).
    pub fn haircut(mut self, haircut: F) -> Self {
        self.haircut = haircut;
        self
    }

    /// Set the repo type.
    pub fn repo_type(mut self, repo_type: RepoType) -> Self {
        self.repo_type = repo_type;
        self
    }

    /// Configure as an overnight repo (auto-calculates maturity).
    pub fn overnight(mut self, start_date: Date) -> Self {
        self.start_date = Some(start_date);
        self.repo_type = RepoType::Overnight;
        // Maturity will be calculated in build() based on calendar
        self
    }

    /// Configure as a term repo with specified maturity.
    pub fn term(mut self, start_date: Date, maturity: Date) -> Self {
        self.start_date = Some(start_date);
        self.maturity = Some(maturity);
        self.repo_type = RepoType::Term;
        self
    }

    /// Configure as an open repo (can be terminated with notice).
    pub fn open(mut self, start_date: Date, initial_maturity: Date) -> Self {
        self.start_date = Some(start_date);
        self.maturity = Some(initial_maturity);
        self.repo_type = RepoType::Open;
        self
    }

    /// Set as tri-party repo.
    pub fn triparty(mut self, triparty: bool) -> Self {
        self.triparty = triparty;
        self
    }

    /// Set day count convention.
    pub fn day_count(mut self, day_count: DayCount) -> Self {
        self.day_count = day_count;
        self
    }

    /// Set business day convention.
    pub fn bdc(mut self, bdc: BusinessDayConvention) -> Self {
        self.bdc = bdc;
        self
    }

    /// Set calendar identifier.
    pub fn calendar_id(mut self, calendar_id: &'static str) -> Self {
        self.calendar_id = Some(calendar_id);
        self
    }

    /// Set discount curve identifier.
    pub fn disc_id(mut self, disc_id: &'static str) -> Self {
        self.disc_id = Some(disc_id);
        self
    }

    /// Add a tag to the instrument attributes.
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.attributes = self.attributes.with_tag(tag);
        self
    }

    /// Add metadata to the instrument attributes.
    pub fn with_meta(mut self, key: &str, value: &str) -> Self {
        self.attributes = self.attributes.with_meta(key, value);
        self
    }

    /// Build the repo instrument with validation.
    pub fn build(self) -> Result<Repo> {
        // Validate required fields
        let id = self.id.ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        
        let cash_amount = self.cash_amount.ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        
        let collateral = self.collateral.ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        
        let repo_rate = self.repo_rate.ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        
        let start_date = self.start_date.ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        
        let disc_id = self.disc_id.ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;

        // Calculate maturity for overnight repos
        let maturity = match (self.maturity, self.repo_type) {
            (Some(mat), _) => mat,
            (None, RepoType::Overnight) => {
                // Use appropriate calendar for business day calculation
                match self.calendar_id {
                    Some("target2") => start_date.add_business_days(1, &finstack_core::dates::calendar::Target2)?,
                    Some("nyse") => start_date.add_business_days(1, &finstack_core::dates::calendar::Nyse)?,
                    Some("gblo") => start_date.add_business_days(1, &finstack_core::dates::calendar::Gblo)?,
                    _ => start_date.add_business_days(1, &finstack_core::dates::calendar::Target2)?, // Default
                }
            },
            (None, _) => {
                return Err(Error::Input(finstack_core::error::InputError::Invalid));
            }
        };

        // Validate date order
        if start_date >= maturity {
            return Err(Error::Input(finstack_core::error::InputError::InvalidDateRange));
        }

        // Validate positive values
        if repo_rate < 0.0 {
            return Err(Error::Input(finstack_core::error::InputError::NegativeValue));
        }

        if self.haircut < 0.0 {
            return Err(Error::Input(finstack_core::error::InputError::NegativeValue));
        }

        if collateral.quantity <= 0.0 {
            return Err(Error::Input(finstack_core::error::InputError::NonPositiveValue));
        }

        Ok(Repo {
            id: InstrumentId::new(id),
            cash_amount,
            collateral,
            repo_rate,
            start_date,
            maturity,
            haircut: self.haircut,
            repo_type: self.repo_type,
            triparty: self.triparty,
            day_count: self.day_count,
            bdc: self.bdc,
            calendar_id: self.calendar_id,
            disc_id,
            attributes: self.attributes,
        })
    }
}
