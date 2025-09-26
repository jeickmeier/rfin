/// Generate calendar implementations from JSON definitions.

use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct CalendarDef {
    id: String,
    name: String,
    ignore_weekends: Option<bool>,
    rules: Vec<RuleDef>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RuleDef {
    Fixed { 
        month: MonthName, 
        day: u8, 
        observed: Option<ObservedName> 
    },
    EasterOffset { days: i16 },
    NthWeekday { n: i8, weekday: WeekdayName, month: MonthName },
    WeekdayShift { weekday: WeekdayName, month: MonthName, day: u8, dir: DirectionName },
    Span { start: Box<RuleDef>, len: u8 },
    ChineseNewYear,
    QingMing,
    BuddhasBirthday,
    VernalEquinoxJp,
    AutumnalEquinoxJp,
}

#[derive(Debug, Clone, Copy)]
enum ObservedName {
    NextMonday,
    FriIfSatMonIfSun,
}

#[derive(Debug, Clone, Copy)]
enum MonthName {
    January, February, March, April, May, June,
    July, August, September, October, November, December,
}

#[derive(Debug, Clone, Copy)]
enum WeekdayName {
    Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday,
}

#[derive(Debug, Clone, Copy)]
enum DirectionName {
    After, Before,
}

impl<'de> Deserialize<'de> for MonthName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?.to_lowercase();
        match s.as_str() {
            "january" => Ok(MonthName::January),
            "february" => Ok(MonthName::February),
            "march" => Ok(MonthName::March),
            "april" => Ok(MonthName::April),
            "may" => Ok(MonthName::May),
            "june" => Ok(MonthName::June),
            "july" => Ok(MonthName::July),
            "august" => Ok(MonthName::August),
            "september" => Ok(MonthName::September),
            "october" => Ok(MonthName::October),
            "november" => Ok(MonthName::November),
            "december" => Ok(MonthName::December),
            _ => Err(serde::de::Error::custom(format!("Unknown month: {}", s))),
        }
    }
}

impl<'de> Deserialize<'de> for WeekdayName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?.to_lowercase();
        match s.as_str() {
            "monday" => Ok(WeekdayName::Monday),
            "tuesday" => Ok(WeekdayName::Tuesday),
            "wednesday" => Ok(WeekdayName::Wednesday),
            "thursday" => Ok(WeekdayName::Thursday),
            "friday" => Ok(WeekdayName::Friday),
            "saturday" => Ok(WeekdayName::Saturday),
            "sunday" => Ok(WeekdayName::Sunday),
            _ => Err(serde::de::Error::custom(format!("Unknown weekday: {}", s))),
        }
    }
}

impl<'de> Deserialize<'de> for DirectionName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?.to_lowercase();
        match s.as_str() {
            "after" => Ok(DirectionName::After),
            "before" => Ok(DirectionName::Before),
            _ => Err(serde::de::Error::custom(format!("Unknown direction: {}", s))),
        }
    }
}

impl<'de> Deserialize<'de> for ObservedName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?.to_lowercase();
        match s.as_str() {
            "next_monday" => Ok(ObservedName::NextMonday),
            "fri_if_sat_mon_if_sun" => Ok(ObservedName::FriIfSatMonIfSun),
            _ => Err(serde::de::Error::custom(format!("Unknown observed: {}", s))),
        }
    }
}

impl MonthName {
    fn to_rust_code(self) -> &'static str {
        match self {
            MonthName::January => "Month::January",
            MonthName::February => "Month::February",
            MonthName::March => "Month::March",
            MonthName::April => "Month::April",
            MonthName::May => "Month::May",
            MonthName::June => "Month::June",
            MonthName::July => "Month::July",
            MonthName::August => "Month::August",
            MonthName::September => "Month::September",
            MonthName::October => "Month::October",
            MonthName::November => "Month::November",
            MonthName::December => "Month::December",
        }
    }
}

impl WeekdayName {
    fn to_rust_code(self) -> &'static str {
        match self {
            WeekdayName::Monday => "Weekday::Monday",
            WeekdayName::Tuesday => "Weekday::Tuesday",
            WeekdayName::Wednesday => "Weekday::Wednesday",
            WeekdayName::Thursday => "Weekday::Thursday",
            WeekdayName::Friday => "Weekday::Friday",
            WeekdayName::Saturday => "Weekday::Saturday",
            WeekdayName::Sunday => "Weekday::Sunday",
        }
    }
}

impl DirectionName {
    fn to_rust_code(self) -> &'static str {
        match self {
            DirectionName::After => "Direction::After",
            DirectionName::Before => "Direction::Before",
        }
    }
}

impl ObservedName {
    fn to_rust_code(self) -> &'static str {
        match self {
            ObservedName::NextMonday => "Observed::NextMonday",
            ObservedName::FriIfSatMonIfSun => "Observed::FriIfSatMonIfSun",
        }
    }
}

impl RuleDef {
    fn to_rust_code(&self) -> String {
        match self {
            RuleDef::Fixed { month, day, observed } => {
                match observed {
                    None => format!("Rule::fixed({}, {})", month.to_rust_code(), day),
                    Some(obs) => format!(
                        "Rule::Fixed {{ month: {}, day: {}, observed: {} }}",
                        month.to_rust_code(), day, obs.to_rust_code()
                    ),
                }
            }
            RuleDef::EasterOffset { days } => {
                format!("Rule::EasterOffset({})", days)
            }
            RuleDef::NthWeekday { n, weekday, month } => {
                format!(
                    "Rule::NthWeekday {{ n: {}, weekday: {}, month: {} }}",
                    n, weekday.to_rust_code(), month.to_rust_code()
                )
            }
            RuleDef::WeekdayShift { weekday, month, day, dir } => {
                format!(
                    "Rule::WeekdayShift {{ weekday: {}, month: {}, day: {}, dir: {} }}",
                    weekday.to_rust_code(), month.to_rust_code(), day, dir.to_rust_code()
                )
            }
            RuleDef::Span { start, len } => {
                format!("Rule::Span {{ start: &({}), len: {} }}", start.to_rust_code(), len)
            }
            RuleDef::ChineseNewYear => "Rule::ChineseNewYear".to_string(),
            RuleDef::QingMing => "Rule::QingMing".to_string(),
            RuleDef::BuddhasBirthday => "Rule::BuddhasBirthday".to_string(),
            RuleDef::VernalEquinoxJp => "Rule::VernalEquinoxJP".to_string(),
            RuleDef::AutumnalEquinoxJp => "Rule::AutumnalEquinoxJP".to_string(),
        }
    }
}

pub fn generate() -> io::Result<()> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let calendar_dir = Path::new(&manifest_dir).join("data").join("calendars");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = Path::new(&out_dir).join("calendars.rs");
    
    // Collect all calendar definitions
    let mut calendars = BTreeMap::new();
    
    for entry in fs::read_dir(calendar_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let json_str = fs::read_to_string(&path)?;
            let cal: CalendarDef = serde_json::from_str(&json_str)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, 
                    format!("Failed to parse {}: {}", path.display(), e)))?;
            calendars.insert(cal.id.clone(), cal);
        }
    }
    
    let mut output = String::new();
    
    // Header
    output.push_str("// Auto-generated from JSON calendar definitions - DO NOT EDIT\n\n");
    output.push_str("use time::{Month, Weekday};\n");
    output.push_str("use crate::dates::calendar::rule::{Rule, Observed, Direction};\n");
    output.push_str("use crate::dates::calendar::types::Calendar;\n");
    output.push_str("use crate::dates::calendar::business_days::HolidayCalendar;\n\n");
    
    // Generate constants for each calendar
    let mut calendar_names = Vec::new();
    for (id, cal) in &calendars {
        let const_name = id.to_uppercase();
        calendar_names.push((id.clone(), const_name.clone()));
        
        // Generate rules array
        output.push_str(&format!("static {}_RULES: &[Rule] = &[\n", const_name));
        for rule in &cal.rules {
            output.push_str("    ");
            output.push_str(&rule.to_rust_code());
            output.push_str(",\n");
        }
        output.push_str("];\n\n");
        
        // Generate calendar constant
        output.push_str(&format!(
            "/// {}\npub static {}: Calendar = Calendar::new(\n    \"{}\",\n    \"{}\",\n    {},\n    {}_RULES,\n);\n\n",
            cal.name,
            const_name,
            id,
            cal.name,
            cal.ignore_weekends.unwrap_or(false),
            const_name
        ));
    }
    
    // Generate ALL_IDS array
    output.push_str("/// All available calendar identifiers.\npub static ALL_IDS: &[&str] = &[\n");
    for (id, _) in &calendar_names {
        output.push_str(&format!("    \"{}\",\n", id));
    }
    output.push_str("];\n\n");
    
    // Generate calendar_by_id function
    output.push_str("/// Resolve a calendar by its identifier.\npub fn calendar_by_id(id: &str) -> Option<&'static dyn HolidayCalendar> {\n");
    output.push_str("    match id.to_lowercase().as_str() {\n");
    for (id, const_name) in &calendar_names {
        output.push_str(&format!("        \"{}\" => Some(&{}),\n", id, const_name));
    }
    output.push_str("        _ => None,\n");
    output.push_str("    }\n");
    output.push_str("}\n\n");
    
    
    fs::write(out_path, output)?;
    Ok(())
}
