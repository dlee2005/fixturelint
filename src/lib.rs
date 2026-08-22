//! Parsing and validation for plain-text sports fixture lists.
//!
//! The format is one fixture per line: `date,home,away`, e.g.
//! `2026-08-23,Arsenal,Chelsea`. Blank lines and lines starting with `#`
//! are ignored.
//!
//! By default parsing is strict: the first bad line (malformed date, a
//! team playing itself, a duplicate fixture) aborts the whole parse.
//! Pass `ParseOptions { lenient: true }` to skip bad lines and collect
//! warnings instead.

use std::collections::HashSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl Date {
    /// Parses a zero-padded `YYYY-MM-DD` date, rejecting calendar-invalid
    /// values like month 13 or Feb 30th outright rather than normalizing them.
    pub fn parse(s: &str) -> Result<Date, String> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
            return Err(format!("expected zero-padded YYYY-MM-DD, got '{}'", s));
        }

        let year: u16 = parts[0]
            .parse()
            .map_err(|_| format!("invalid year in '{}'", s))?;
        let month: u8 = parts[1]
            .parse()
            .map_err(|_| format!("invalid month in '{}'", s))?;
        let day: u8 = parts[2]
            .parse()
            .map_err(|_| format!("invalid day in '{}'", s))?;

        if !(1..=12).contains(&month) {
            return Err(format!("month {} out of range in '{}'", month, s));
        }
        let max_day = days_in_month(year, month);
        if day < 1 || day > max_day {
            return Err(format!(
                "day {} out of range for {}-{:02} in '{}'",
                day, year, month, s
            ));
        }

        Ok(Date { year, month, day })
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

#[derive(Debug, Clone)]
pub struct Fixture {
    pub date: Date,
    pub home: String,
    pub away: String,
}

impl fmt::Display for Fixture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}  {} vs {}", self.date, self.home, self.away)
    }
}

#[derive(Debug)]
pub struct FixtureError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for FixtureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ParseOptions {
    pub lenient: bool,
}

#[derive(Debug, Default)]
pub struct ParseOutcome {
    pub fixtures: Vec<Fixture>,
    pub warnings: Vec<String>,
}

pub fn parse_str(input: &str, options: &ParseOptions) -> Result<ParseOutcome, FixtureError> {
    let mut outcome = ParseOutcome::default();
    let mut seen: HashSet<(Date, String, String)> = HashSet::new();

    for (idx, raw_line) in input.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parsed = parse_line(line).and_then(|(date, home, away)| {
            if home.eq_ignore_ascii_case(&away) {
                return Err(format!("team '{}' cannot play itself", home));
            }
            Ok((date, home, away))
        });

        match parsed {
            Ok((date, home, away)) => {
                let key = (date.clone(), home.to_lowercase(), away.to_lowercase());
                if !seen.insert(key) {
                    let msg = format!("duplicate fixture: {} vs {} on {}", home, away, date);
                    if options.lenient {
                        outcome
                            .warnings
                            .push(format!("line {}: {} (skipped)", line_no, msg));
                        continue;
                    }
                    return Err(FixtureError {
                        line: line_no,
                        message: msg,
                    });
                }
                outcome.fixtures.push(Fixture { date, home, away });
            }
            Err(msg) => {
                if options.lenient {
                    outcome
                        .warnings
                        .push(format!("line {}: {} (skipped)", line_no, msg));
                    continue;
                }
                return Err(FixtureError {
                    line: line_no,
                    message: msg,
                });
            }
        }
    }

    Ok(outcome)
}

fn parse_line(line: &str) -> Result<(Date, String, String), String> {
    let fields: Vec<&str> = line.split(',').map(|f| f.trim()).collect();
    if fields.len() != 3 {
        return Err(format!(
            "expected 3 comma-separated fields (date,home,away), got {}",
            fields.len()
        ));
    }

    let date = Date::parse(fields[0])?;
    let home = fields[1];
    let away = fields[2];
    if home.is_empty() || away.is_empty() {
        return Err("team names cannot be empty".to_string());
    }

    Ok((date, home.to_string(), away.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_clean_list() {
        let input = "2026-08-23,Arsenal,Chelsea\n2026-08-24,Liverpool,Everton\n";
        let outcome = parse_str(input, &ParseOptions::default()).unwrap();
        assert_eq!(outcome.fixtures.len(), 2);
        assert!(outcome.warnings.is_empty());
    }

    #[test]
    fn ignores_blank_lines_and_comments() {
        let input = "# round 1\n\n2026-08-23,Arsenal,Chelsea\n";
        let outcome = parse_str(input, &ParseOptions::default()).unwrap();
        assert_eq!(outcome.fixtures.len(), 1);
    }

    #[test]
    fn strict_mode_rejects_bad_date() {
        let input = "2026-02-30,Arsenal,Chelsea\n";
        let err = parse_str(input, &ParseOptions::default()).unwrap_err();
        assert_eq!(err.line, 1);
    }

    #[test]
    fn strict_mode_rejects_duplicate_fixture() {
        let input = "2026-08-23,Arsenal,Chelsea\n2026-08-23,Arsenal,Chelsea\n";
        let err = parse_str(input, &ParseOptions::default()).unwrap_err();
        assert_eq!(err.line, 2);
    }

    #[test]
    fn strict_mode_rejects_team_playing_itself() {
        let input = "2026-08-23,Arsenal,Arsenal\n";
        let err = parse_str(input, &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("cannot play itself"));
    }

    #[test]
    fn lenient_mode_skips_bad_lines_and_warns() {
        let input = "2026-08-23,Arsenal,Chelsea\n2026-02-30,Foo,Bar\n2026-08-24,Liverpool,Everton\n";
        let options = ParseOptions { lenient: true };
        let outcome = parse_str(input, &options).unwrap();
        assert_eq!(outcome.fixtures.len(), 2);
        assert_eq!(outcome.warnings.len(), 1);
    }
}
