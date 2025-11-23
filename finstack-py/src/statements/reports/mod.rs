//! Reports module bindings for financial models.

use crate::core::dates::periods::PyPeriodId;
use crate::statements::evaluator::PyResults;
use crate::statements::types::model::PyFinancialModelSpec;
use finstack_statements::reports::{
    Alignment, CreditAssessmentReport, PLSummaryReport, Report, TableBuilder,
};
use finstack_statements::types::DebtInstrumentSpec;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::{wrap_pyfunction, Bound};
use std::fmt::Write as FmtWrite;

/// Alignment options for table columns.
#[pyclass(module = "finstack.statements.reports", name = "Alignment", frozen)]
#[derive(Clone, Copy)]
pub struct PyAlignment {
    inner: Alignment,
}

#[pymethods]
impl PyAlignment {
    #[classattr]
    const LEFT: Self = Self {
        inner: Alignment::Left,
    };

    #[classattr]
    const RIGHT: Self = Self {
        inner: Alignment::Right,
    };

    #[classattr]
    const CENTER: Self = Self {
        inner: Alignment::Center,
    };

    fn __repr__(&self) -> String {
        format!("Alignment.{:?}", self.inner)
    }
}

/// Builder for ASCII and Markdown tables.
#[pyclass(module = "finstack.statements.reports", name = "TableBuilder")]
pub struct PyTableBuilder {
    inner: TableBuilder,
}

#[pymethods]
impl PyTableBuilder {
    #[new]
    /// Create a new table builder.
    ///
    /// Returns
    /// -------
    /// TableBuilder
    ///     Table builder instance
    fn new() -> Self {
        Self {
            inner: TableBuilder::new(),
        }
    }

    #[pyo3(signature = (name))]
    /// Add a column header.
    ///
    /// Parameters
    /// ----------
    /// name : str
    ///     Column header text
    fn add_header(&mut self, name: String) {
        self.inner.add_header(name);
    }

    #[pyo3(signature = (name, alignment))]
    /// Add a column header with specific alignment.
    ///
    /// Parameters
    /// ----------
    /// name : str
    ///     Column header text
    /// alignment : Alignment
    ///     Column alignment
    fn add_header_with_alignment(&mut self, name: String, alignment: &PyAlignment) {
        self.inner.add_header_with_alignment(name, alignment.inner);
    }

    #[pyo3(signature = (cells))]
    /// Add a data row.
    ///
    /// Parameters
    /// ----------
    /// cells : list[str]
    ///     List of cell values
    fn add_row(&mut self, cells: Vec<String>) {
        self.inner.add_row(cells);
    }

    #[pyo3(signature = ())]
    /// Build ASCII table.
    ///
    /// Returns
    /// -------
    /// str
    ///     Formatted ASCII table with box-drawing characters
    fn build(&self) -> String {
        self.inner.build()
    }

    #[pyo3(signature = ())]
    /// Build Markdown table.
    ///
    /// Returns
    /// -------
    /// str
    ///     Formatted Markdown table
    fn build_markdown(&self) -> String {
        self.inner.build_markdown()
    }

    fn __repr__(&self) -> String {
        "TableBuilder()".to_string()
    }
}

/// P&L summary report.
#[pyclass(module = "finstack.statements.reports", name = "PLSummaryReport")]
pub struct PyPLSummaryReport {
    results: PyResults,
    line_items: Vec<String>,
    periods: Vec<finstack_core::dates::PeriodId>,
}

#[pymethods]
impl PyPLSummaryReport {
    #[new]
    #[pyo3(signature = (results, line_items, periods))]
    /// Create a new P&L summary report.
    ///
    /// Parameters
    /// ----------
    /// results : Results
    ///     Evaluation results
    /// line_items : list[str]
    ///     Node IDs to include
    /// periods : list[PeriodId]
    ///     Periods to display
    ///
    /// Returns
    /// -------
    /// PLSummaryReport
    ///     Report instance
    fn new(
        results: &PyResults,
        line_items: Vec<String>,
        periods: Vec<crate::core::dates::periods::PyPeriodId>,
    ) -> Self {
        Self {
            results: results.clone(),
            line_items,
            periods: periods.into_iter().map(|p| p.inner).collect(),
        }
    }

    #[pyo3(signature = ())]
    /// Convert report to string format.
    ///
    /// Returns
    /// -------
    /// str
    ///     Formatted report
    fn to_string(&self) -> String {
        let report = PLSummaryReport::new(
            &self.results.inner,
            self.line_items.clone(),
            self.periods.clone(),
        );
        report.to_string()
    }

    #[pyo3(signature = ())]
    /// Convert report to Markdown format.
    ///
    /// Returns
    /// -------
    /// str
    ///     Markdown formatted report
    fn to_markdown(&self) -> String {
        let report = PLSummaryReport::new(
            &self.results.inner,
            self.line_items.clone(),
            self.periods.clone(),
        );
        report.to_markdown()
    }

    #[pyo3(signature = ())]
    /// Print report to stdout.
    fn print(&self) {
        let report = PLSummaryReport::new(
            &self.results.inner,
            self.line_items.clone(),
            self.periods.clone(),
        );
        report.print();
    }

    fn __repr__(&self) -> String {
        format!(
            "PLSummaryReport(line_items={}, periods={})",
            self.line_items.len(),
            self.periods.len()
        )
    }

    fn __str__(&self) -> String {
        self.to_string()
    }
}

/// Credit assessment report.
#[pyclass(
    module = "finstack.statements.reports",
    name = "CreditAssessmentReport"
)]
pub struct PyCreditAssessmentReport {
    results: PyResults,
    as_of: finstack_core::dates::PeriodId,
}

#[pymethods]
impl PyCreditAssessmentReport {
    #[new]
    #[pyo3(signature = (results, as_of))]
    /// Create a new credit assessment report.
    ///
    /// Parameters
    /// ----------
    /// results : Results
    ///     Evaluation results
    /// as_of : PeriodId
    ///     Period for assessment
    ///
    /// Returns
    /// -------
    /// CreditAssessmentReport
    ///     Report instance
    fn new(results: &PyResults, as_of: &crate::core::dates::periods::PyPeriodId) -> Self {
        Self {
            results: results.clone(),
            as_of: as_of.inner,
        }
    }

    #[pyo3(signature = ())]
    /// Convert report to string format.
    ///
    /// Returns
    /// -------
    /// str
    ///     Formatted report
    fn to_string(&self) -> String {
        let report = CreditAssessmentReport::new(&self.results.inner, self.as_of);
        report.to_string()
    }

    #[pyo3(signature = ())]
    /// Convert report to Markdown format.
    ///
    /// Returns
    /// -------
    /// str
    ///     Markdown formatted report
    fn to_markdown(&self) -> String {
        let report = CreditAssessmentReport::new(&self.results.inner, self.as_of);
        report.to_markdown()
    }

    #[pyo3(signature = ())]
    /// Print report to stdout.
    fn print(&self) {
        let report = CreditAssessmentReport::new(&self.results.inner, self.as_of);
        report.print();
    }

    fn __repr__(&self) -> String {
        format!("CreditAssessmentReport(as_of={})", self.as_of)
    }

    fn __str__(&self) -> String {
        self.to_string()
    }
}

/// Debt summary report providing a quick view of capital structure.
#[pyclass(module = "finstack.statements.reports", name = "DebtSummaryReport")]
#[derive(Clone)]
pub struct PyDebtSummaryReport {
    model: PyFinancialModelSpec,
    results: PyResults,
    as_of: finstack_core::dates::PeriodId,
}

#[pymethods]
impl PyDebtSummaryReport {
    #[new]
    #[pyo3(signature = (model, results, as_of))]
    /// Create a new debt summary report.
    ///
    /// Parameters
    /// ----------
    /// model : FinancialModelSpec
    ///     Financial model containing capital structure details.
    /// results : Results
    ///     Evaluation results for metric lookups.
    /// as_of : PeriodId
    ///     Reporting period.
    fn new(model: &PyFinancialModelSpec, results: &PyResults, as_of: &PyPeriodId) -> Self {
        Self {
            model: model.clone(),
            results: results.clone(),
            as_of: as_of.inner,
        }
    }

    #[pyo3(signature = ())]
    /// Convert the report to a formatted string.
    fn to_string(&self) -> String {
        render_debt_summary(&self.model.inner, &self.results.inner, self.as_of)
    }

    #[pyo3(signature = ())]
    /// Convert the report to Markdown format (currently same as text).
    fn to_markdown(&self) -> String {
        self.to_string()
    }

    #[pyo3(signature = ())]
    /// Print the report to stdout.
    fn print(&self) {
        println!("{}", self.to_string());
    }

    fn __repr__(&self) -> String {
        format!("DebtSummaryReport(as_of={})", self.as_of)
    }

    fn __str__(&self) -> String {
        self.to_string()
    }
}

/// Convenience function to print a debt summary.
#[pyfunction]
#[pyo3(signature = (model, results, as_of))]
fn print_debt_summary(model: &PyFinancialModelSpec, results: &PyResults, as_of: &PyPeriodId) {
    let report = PyDebtSummaryReport::new(model, results, as_of);
    println!("{}", report.to_string());
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(_py, "reports")?;
    module.setattr("__doc__", "Convenience reporting for financial statements.")?;

    module.add_class::<PyAlignment>()?;
    module.add_class::<PyTableBuilder>()?;
    module.add_class::<PyPLSummaryReport>()?;
    module.add_class::<PyCreditAssessmentReport>()?;
    module.add_class::<PyDebtSummaryReport>()?;
    module.add_function(wrap_pyfunction!(print_debt_summary, &module)?)?;

    parent.add_submodule(&module)?;
    parent.setattr("reports", &module)?;

    Ok(vec![
        "Alignment",
        "TableBuilder",
        "PLSummaryReport",
        "CreditAssessmentReport",
        "DebtSummaryReport",
        "print_debt_summary",
    ])
}

fn render_debt_summary(
    model: &finstack_statements::types::FinancialModelSpec,
    results: &finstack_statements::results::Results,
    as_of: finstack_core::dates::PeriodId,
) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "Debt Summary - {}", as_of);
    let _ = writeln!(output, "==============================");

    if let Some(cs) = &model.capital_structure {
        let mut bond = 0usize;
        let mut swap = 0usize;
        let mut term = 0usize;
        let mut generic = 0usize;

        for instrument in &cs.debt_instruments {
            match instrument {
                DebtInstrumentSpec::Bond { .. } => bond += 1,
                DebtInstrumentSpec::Swap { .. } => swap += 1,
                DebtInstrumentSpec::TermLoan { .. } => term += 1,
                DebtInstrumentSpec::Generic { .. } => generic += 1,
            }
        }

        let total = cs.debt_instruments.len();
        let _ = writeln!(output, "Total instruments : {}", total);
        if total > 0 {
            let _ = writeln!(output, "  Bonds         : {}", bond);
            let _ = writeln!(output, "  Swaps         : {}", swap);
            let _ = writeln!(output, "  Term Loans    : {}", term);
            let _ = writeln!(output, "  Custom/Other  : {}", generic);
        }

        if !cs.debt_instruments.is_empty() {
            let _ = writeln!(output, "\nInstruments:");
            for instrument in &cs.debt_instruments {
                let (id, kind) = match instrument {
                    DebtInstrumentSpec::Bond { id, .. } => (id, "Bond"),
                    DebtInstrumentSpec::Swap { id, .. } => (id, "Swap"),
                    DebtInstrumentSpec::TermLoan { id, .. } => (id, "Term Loan"),
                    DebtInstrumentSpec::Generic { id, .. } => (id, "Custom"),
                };
                let _ = writeln!(output, "  - {} ({})", id, kind);
            }
        }
    } else {
        let _ = writeln!(output, "No capital structure defined in the model.");
    }

    const METRICS: [(&str, &str, bool); 6] = [
        ("total_debt", "Total Debt", false),
        ("net_debt", "Net Debt", false),
        ("gross_debt", "Gross Debt", false),
        ("debt_service", "Debt Service", false),
        ("interest_expense", "Interest Expense", false),
        ("debt_to_ebitda", "Debt / EBITDA", true),
    ];

    let mut any_metrics = false;
    for (node_id, label, is_ratio) in METRICS {
        if let Some(value) = results.get(node_id, &as_of) {
            if !any_metrics {
                let _ = writeln!(output, "\nKey Metrics:");
                any_metrics = true;
            }
            if is_ratio {
                let _ = writeln!(output, "  {:<20}: {:.2}x", label, value);
            } else {
                let _ = writeln!(output, "  {:<20}: {:.2}", label, value);
            }
        }
    }

    if !any_metrics {
        let _ = writeln!(
            output,
            "\nNo standardized debt metrics found in results for period {}.",
            as_of
        );
    }

    output
}
