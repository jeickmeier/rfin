//! Table formatting utilities for ASCII and Markdown output.

use std::fmt::Write as FmtWrite;

/// Alignment options for table columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    /// Left-aligned
    Left,
    /// Right-aligned
    Right,
    /// Center-aligned
    Center,
}

impl Default for Alignment {
    fn default() -> Self {
        Self::Left
    }
}

/// Builder for ASCII and Markdown tables.
///
/// # Examples
///
/// ```rust
/// use finstack_statements::reports::tables::{TableBuilder, Alignment};
///
/// let mut table = TableBuilder::new();
/// table.add_header("Name");
/// table.add_header_with_alignment("Value", Alignment::Right);
/// table.add_row(vec!["Revenue".to_string(), "$100M".to_string()]);
/// table.add_row(vec!["COGS".to_string(), "$40M".to_string()]);
///
/// let ascii = table.build();
/// println!("{}", ascii);
/// ```
#[derive(Debug, Clone)]
pub struct TableBuilder {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    alignment: Vec<Alignment>,
}

impl TableBuilder {
    /// Create a new table builder.
    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
            rows: Vec::new(),
            alignment: Vec::new(),
        }
    }

    /// Add a column header.
    ///
    /// # Arguments
    ///
    /// * `name` - Column header text
    pub fn add_header(&mut self, name: impl Into<String>) {
        self.headers.push(name.into());
        self.alignment.push(Alignment::Left);
    }

    /// Add a column header with specific alignment.
    ///
    /// # Arguments
    ///
    /// * `name` - Column header text
    /// * `alignment` - Column alignment
    pub fn add_header_with_alignment(&mut self, name: impl Into<String>, alignment: Alignment) {
        self.headers.push(name.into());
        self.alignment.push(alignment);
    }

    /// Add a data row.
    ///
    /// # Arguments
    ///
    /// * `cells` - Vector of cell values
    pub fn add_row(&mut self, cells: Vec<String>) {
        self.rows.push(cells);
    }

    /// Build ASCII table.
    ///
    /// Returns a formatted ASCII table with box-drawing characters.
    pub fn build(&self) -> String {
        if self.headers.is_empty() {
            return String::new();
        }

        let mut output = String::new();

        // Calculate column widths
        let widths = self.calculate_column_widths();

        // Top border
        self.write_border(&mut output, &widths, "┌", "┬", "┐");
        output.push('\n');

        // Headers
        output.push('│');
        for (i, header) in self.headers.iter().enumerate() {
            let width = widths[i];
            let aligned = self.align_text(header, width, self.alignment[i]);
            write!(&mut output, " {} │", aligned).expect("writing to String cannot fail");
        }
        output.push('\n');

        // Header separator
        self.write_border(&mut output, &widths, "├", "┼", "┤");
        output.push('\n');

        // Data rows
        for row in &self.rows {
            output.push('│');
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    let width = widths[i];
                    let align = if i < self.alignment.len() {
                        self.alignment[i]
                    } else {
                        Alignment::Left
                    };
                    let aligned = self.align_text(cell, width, align);
                    write!(&mut output, " {} │", aligned).expect("writing to String cannot fail");
                }
            }
            output.push('\n');
        }

        // Bottom border
        self.write_border(&mut output, &widths, "└", "┴", "┘");

        output
    }

    /// Build Markdown table.
    ///
    /// Returns a formatted Markdown table.
    pub fn build_markdown(&self) -> String {
        if self.headers.is_empty() {
            return String::new();
        }

        let mut output = String::new();

        // Calculate column widths
        let widths = self.calculate_column_widths();

        // Headers
        output.push('|');
        for (i, header) in self.headers.iter().enumerate() {
            let width = widths[i];
            let aligned = self.align_text(header, width, Alignment::Left);
            write!(&mut output, " {} |", aligned).expect("writing to String cannot fail");
        }
        output.push('\n');

        // Separator
        output.push('|');
        for (i, &width) in widths.iter().enumerate() {
            let align = if i < self.alignment.len() {
                self.alignment[i]
            } else {
                Alignment::Left
            };

            let sep = match align {
                Alignment::Left => format!(":{}", "-".repeat(width)),
                Alignment::Right => format!("{}:", "-".repeat(width)),
                Alignment::Center => format!(":{}:", "-".repeat(width.saturating_sub(1).max(1))),
            };
            write!(&mut output, " {} |", sep).expect("writing to String cannot fail");
        }
        output.push('\n');

        // Data rows
        for row in &self.rows {
            output.push('|');
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    let width = widths[i];
                    let align = if i < self.alignment.len() {
                        self.alignment[i]
                    } else {
                        Alignment::Left
                    };
                    let aligned = self.align_text(cell, width, align);
                    write!(&mut output, " {} |", aligned).expect("writing to String cannot fail");
                }
            }
            output.push('\n');
        }

        output
    }

    // Internal helpers

    fn calculate_column_widths(&self) -> Vec<usize> {
        let mut widths = self.headers.iter().map(|h| h.len()).collect::<Vec<_>>();

        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(cell.len());
                }
            }
        }

        widths
    }

    fn align_text(&self, text: &str, width: usize, alignment: Alignment) -> String {
        let text_len = text.len();
        if text_len >= width {
            return text.to_string();
        }

        let padding = width - text_len;

        match alignment {
            Alignment::Left => format!("{}{}", text, " ".repeat(padding)),
            Alignment::Right => format!("{}{}", " ".repeat(padding), text),
            Alignment::Center => {
                let left_pad = padding / 2;
                let right_pad = padding - left_pad;
                format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
            }
        }
    }

    fn write_border(
        &self,
        output: &mut String,
        widths: &[usize],
        left: &str,
        middle: &str,
        right: &str,
    ) {
        output.push_str(left);
        for (i, &width) in widths.iter().enumerate() {
            output.push_str(&"─".repeat(width + 2));
            if i < widths.len() - 1 {
                output.push_str(middle);
            }
        }
        output.push_str(right);
    }
}

impl Default for TableBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_table() {
        let table = TableBuilder::new();
        assert_eq!(table.build(), "");
    }

    #[test]
    fn test_simple_table() {
        let mut table = TableBuilder::new();
        table.add_header("Name");
        table.add_header("Value");
        table.add_row(vec!["Revenue".to_string(), "100".to_string()]);
        table.add_row(vec!["COGS".to_string(), "40".to_string()]);

        let output = table.build();
        assert!(output.contains("Name"));
        assert!(output.contains("Value"));
        assert!(output.contains("Revenue"));
        assert!(output.contains("COGS"));
        assert!(output.contains("┌"));
        assert!(output.contains("└"));
    }

    #[test]
    fn test_markdown_table() {
        let mut table = TableBuilder::new();
        table.add_header("Name");
        table.add_header_with_alignment("Value", Alignment::Right);
        table.add_row(vec!["Revenue".to_string(), "100".to_string()]);

        let output = table.build_markdown();
        println!("Output:\n{}", output);
        assert!(output.contains("| Name"));
        assert!(output.contains("| Value |"));
        // Markdown alignment markers should be present
        assert!(output.contains(":---") || output.contains("---:"));
    }

    #[test]
    fn test_alignment() {
        let table = TableBuilder::new();
        assert_eq!(table.align_text("test", 10, Alignment::Left), "test      ");
        assert_eq!(table.align_text("test", 10, Alignment::Right), "      test");
        assert_eq!(
            table.align_text("test", 10, Alignment::Center),
            "   test   "
        );
    }

    #[test]
    fn test_column_width_calculation() {
        let mut table = TableBuilder::new();
        table.add_header("A");
        table.add_header("B");
        table.add_row(vec!["short".to_string(), "much longer text".to_string()]);

        let widths = table.calculate_column_widths();
        assert_eq!(widths[0], 5); // "short" is longer than "A"
        assert_eq!(widths[1], 16); // "much longer text" is longer than "B"
    }
}

