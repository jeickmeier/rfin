use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

fn main() -> std::io::Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("currency_generated.rs");
    let mut f = File::create(&dest_path)?;

    // Tell cargo to rerun if the CSV file changes
    println!("cargo:rerun-if-changed=data/iso_4217.csv");

    let input = File::open("data/iso_4217.csv")?;
    let reader = BufReader::new(input);
    let mut lines = reader.lines();

    // Skip header
    lines.next();

    let mut currencies = Vec::new();
    for line in lines {
        let line = line?;
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 4 {
            let code = parts[0].to_string();
            let numeric: u16 = parts[1].parse().unwrap_or(0);
            let minor_units: u8 = parts[2].parse().unwrap_or(2);
            let name = parts[3].to_string();
            
            currencies.push((code, numeric, minor_units, name));
        }
    }

    // Generate the enum
    writeln!(f, "/// ISO 4217 currency codes")?;
    writeln!(f, "#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]")?;
    writeln!(f, "#[cfg_attr(feature = \"serde\", derive(serde::Serialize, serde::Deserialize))]")?;
    writeln!(f, "#[repr(u16)]")?;
    writeln!(f, "pub enum Currency {{")?;

    for (code, numeric, _minor_units, name) in &currencies {
        writeln!(f, "    /// {} ({})", name, numeric)?;
        writeln!(f, "    {} = {},", code, numeric)?;
    }

    writeln!(f, "}}")?;
    writeln!(f)?;

    // Generate the minor_units function
    writeln!(f, "impl Currency {{")?;
    writeln!(f, "    /// Returns the number of minor units (decimal places) for this currency")?;
    writeln!(f, "    pub const fn minor_units(self) -> u8 {{")?;
    writeln!(f, "        match self {{")?;

    for (code, _numeric, minor_units, _name) in &currencies {
        writeln!(f, "            Currency::{} => {},", code, minor_units)?;
    }

    writeln!(f, "        }}")?;
    writeln!(f, "    }}")?;
    writeln!(f, "}}")?;
    writeln!(f)?;

    // Generate FromStr lookup table
    writeln!(f, "const CURRENCY_FROM_STR: &[(u32, Currency)] = &[")?;
    for (code, _numeric, _minor_units, _name) in &currencies {
        // Convert currency code to u32 for fast lookup
        let mut hash = 0u32;
        for &byte in code.as_bytes() {
            let upper_byte = if byte >= b'a' && byte <= b'z' {
                byte - b'a' + b'A'
            } else {
                byte
            };
            hash = hash.wrapping_mul(31).wrapping_add(upper_byte as u32);
        }
        writeln!(f, "    ({}, Currency::{}),", hash, code)?;
    }
    writeln!(f, "];")?;
    writeln!(f)?;

    // Generate lookup function
    writeln!(f, "fn hash_currency_code(s: &str) -> u32 {{")?;
    writeln!(f, "    let mut hash = 0u32;")?;
    writeln!(f, "    for byte in s.bytes() {{")?;
    writeln!(f, "        let upper_byte = if byte >= b'a' && byte <= b'z' {{")?;
    writeln!(f, "            byte - b'a' + b'A'")?;
    writeln!(f, "        }} else {{")?;
    writeln!(f, "            byte")?;
    writeln!(f, "        }};")?;
    writeln!(f, "        hash = hash.wrapping_mul(31).wrapping_add(upper_byte as u32);")?;
    writeln!(f, "    }}")?;
    writeln!(f, "    hash")?;
    writeln!(f, "}}")?;
    writeln!(f)?;

    writeln!(f, "pub(crate) fn lookup_currency(s: &str) -> Option<Currency> {{")?;
    writeln!(f, "    let hash = hash_currency_code(s);")?;
    writeln!(f, "    CURRENCY_FROM_STR.iter()")?;
    writeln!(f, "        .find(|(h, _)| *h == hash)")?;
    writeln!(f, "        .map(|(_, currency)| *currency)")?;
    writeln!(f, "}}")?;

    Ok(())
}