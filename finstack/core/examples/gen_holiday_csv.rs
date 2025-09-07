//! Utility for exporting rule-based calendars to CSV for validation or interoperability.
//!
//! This binary generates CSV files containing holiday dates by evaluating rule-based
//! calendar implementations over a specified date range. The generated CSVs are used
//! for validation purposes and external interoperability, but are not used by the
//! runtime engine itself.

use std::fs::{create_dir_all, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use time::{Date, Duration, Month};

use finstack_core::dates::calendar::{Cnbe, Hkhk};
use finstack_core::dates::HolidayCalendar;

fn write_calendar_csv<P: Into<PathBuf>>(
    path: P,
    cal: &dyn HolidayCalendar,
    start_year: i32,
    end_year: i32,
) -> std::io::Result<()> {
    let path: PathBuf = path.into();
    if let Some(dir) = path.parent() {
        create_dir_all(dir)?;
    }
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);

    // Optional header (parser ignores it)
    writeln!(w, "date")?;

    for year in start_year..=end_year {
        let mut d = Date::from_calendar_date(year, Month::January, 1).unwrap();
        while d.year() == year {
            if cal.is_holiday(d) {
                writeln!(w, "{d}")?; // Date Display impl => YYYY-MM-DD
            }
            d += Duration::days(1);
        }
    }
    w.flush()?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let root = env!("CARGO_MANIFEST_DIR");
    let mut cnbe_path = PathBuf::from(root);
    cnbe_path.push("data/holidays/cnbe.csv");
    let mut hkhk_path = PathBuf::from(root);
    hkhk_path.push("data/holidays/hkhk.csv");

    // Use rules via `is_holiday` to generate CSVs for 1970..=2150
    write_calendar_csv(cnbe_path, &Cnbe, 1970, 2150)?;
    write_calendar_csv(hkhk_path, &Hkhk, 1970, 2150)?;

    Ok(())
}
