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

impl Date {
    /// Returns the date `days` days after this one. Walks month by month
    /// rather than converting to a Julian day number, since the spans
    /// involved (a season's worth of round-robin rounds) are small.
    pub fn add_days(&self, days: u32) -> Date {
        let mut year = self.year;
        let mut month = self.month;
        let mut day = self.day as u32;
        let mut remaining = days;

        loop {
            let days_left_in_month = days_in_month(year, month) as u32 - day;
            if remaining <= days_left_in_month {
                day += remaining;
                break;
            }
            remaining -= days_left_in_month + 1;
            day = 1;
            if month == 12 {
                month = 1;
                year += 1;
            } else {
                month += 1;
            }
        }

        Date { year, month, day: day as u8 }
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

impl Fixture {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"date\":\"{}\",\"home\":\"{}\",\"away\":\"{}\"}}",
            self.date,
            json_escape(&self.home),
            json_escape(&self.away)
        )
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

#[derive(Debug, Clone, Default)]
pub struct ParseOptions {
    pub lenient: bool,
    /// When set, every home and away team must appear in the registry.
    pub registry: Option<TeamRegistry>,
}

/// A known set of team names to validate fixtures against, so a typo'd or
/// renamed team gets caught instead of silently entering the schedule.
#[derive(Debug, Clone, Default)]
pub struct TeamRegistry {
    teams: HashSet<String>,
}

impl TeamRegistry {
    /// Parses a plain list of team names, one per line. Blank lines and
    /// lines starting with `#` are ignored, matching the teams-file format
    /// `generate` already reads.
    pub fn parse_str(input: &str) -> TeamRegistry {
        let teams = input
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| line.to_lowercase())
            .collect();
        TeamRegistry { teams }
    }

    /// Case-insensitive membership check.
    pub fn contains(&self, team: &str) -> bool {
        self.teams.contains(&team.to_lowercase())
    }
}

#[derive(Debug, Default)]
pub struct ParseOutcome {
    pub fixtures: Vec<Fixture>,
    pub warnings: Vec<String>,
}

impl ParseOutcome {
    /// Renders the outcome as a single JSON object with `fixtures` and
    /// `warnings` arrays. Written by hand rather than pulling in a JSON
    /// crate, since the shape here is fixed and small.
    pub fn to_json(&self) -> String {
        let fixtures: Vec<String> = self.fixtures.iter().map(Fixture::to_json).collect();
        let warnings: Vec<String> = self
            .warnings
            .iter()
            .map(|w| format!("\"{}\"", json_escape(w)))
            .collect();
        format!(
            "{{\"fixtures\":[{}],\"warnings\":[{}]}}",
            fixtures.join(","),
            warnings.join(",")
        )
    }
}

/// Escapes a string for embedding in a JSON string literal.
pub fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
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
            if let Some(registry) = &options.registry {
                if !registry.contains(&home) {
                    return Err(format!("team '{}' is not in the registry", home));
                }
                if !registry.contains(&away) {
                    return Err(format!("team '{}' is not in the registry", away));
                }
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

/// Generates a single round-robin schedule (every team plays every other
/// team exactly once) using the standard circle method: fix one team, and
/// rotate the rest one position each round.
///
/// Rounds are spaced `days_between_rounds` apart starting on `start`. Team
/// names are matched case-insensitively for duplicate detection but kept in
/// their original casing in the output.
pub fn generate_round_robin(
    teams: &[String],
    start: &Date,
    days_between_rounds: u32,
) -> Result<Vec<Fixture>, String> {
    let mut names: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for team in teams {
        let team = team.trim();
        if team.is_empty() {
            continue;
        }
        if !seen.insert(team.to_lowercase()) {
            return Err(format!("duplicate team '{}' in list", team));
        }
        names.push(team.to_string());
    }

    if names.len() < 2 {
        return Err("need at least 2 distinct teams to generate fixtures".to_string());
    }

    let bye = names.len() % 2 == 1;
    if bye {
        names.push("BYE".to_string());
    }
    let n = names.len();
    let half = n / 2;

    let mut arrangement: Vec<usize> = (0..n).collect();
    let mut fixtures = Vec::new();

    for round in 0..(n - 1) {
        let date = start.add_days(round as u32 * days_between_rounds);
        for i in 0..half {
            let a = arrangement[i];
            let b = arrangement[n - 1 - i];
            if names[a] == "BYE" || names[b] == "BYE" {
                continue;
            }
            // Alternate which side of the pairing is home each round so one
            // team doesn't end up hosting every fixture it's involved in.
            let (home, away) = if round % 2 == 0 { (a, b) } else { (b, a) };
            fixtures.push(Fixture {
                date: date.clone(),
                home: names[home].clone(),
                away: names[away].clone(),
            });
        }
        let last = arrangement.pop().unwrap();
        arrangement.insert(1, last);
    }

    Ok(fixtures)
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

    #[test]
    fn registry_accepts_known_teams() {
        let registry = TeamRegistry::parse_str("Arsenal\nChelsea\n");
        let options = ParseOptions {
            lenient: false,
            registry: Some(registry),
        };
        let outcome = parse_str("2026-08-23,Arsenal,Chelsea\n", &options).unwrap();
        assert_eq!(outcome.fixtures.len(), 1);
    }

    #[test]
    fn registry_check_is_case_insensitive() {
        let registry = TeamRegistry::parse_str("arsenal\nCHELSEA\n");
        assert!(registry.contains("Arsenal"));
        assert!(registry.contains("chelsea"));
        assert!(!registry.contains("Everton"));
    }

    #[test]
    fn strict_mode_rejects_team_not_in_registry() {
        let registry = TeamRegistry::parse_str("Arsenal\nChelsea\n");
        let options = ParseOptions {
            lenient: false,
            registry: Some(registry),
        };
        let err = parse_str("2026-08-23,Arsenal,Everton\n", &options).unwrap_err();
        assert!(err.message.contains("not in the registry"));
    }

    #[test]
    fn lenient_mode_warns_on_team_not_in_registry() {
        let registry = TeamRegistry::parse_str("Arsenal\nChelsea\n");
        let options = ParseOptions {
            lenient: true,
            registry: Some(registry),
        };
        let outcome = parse_str("2026-08-23,Arsenal,Everton\n", &options).unwrap();
        assert!(outcome.fixtures.is_empty());
        assert_eq!(outcome.warnings.len(), 1);
    }

    #[test]
    fn add_days_rolls_over_month_and_year_boundaries() {
        let d = Date { year: 2026, month: 1, day: 30 };
        assert_eq!(d.add_days(3), Date { year: 2026, month: 2, day: 2 });

        let new_years_eve = Date { year: 2026, month: 12, day: 31 };
        assert_eq!(new_years_eve.add_days(1), Date { year: 2027, month: 1, day: 1 });

        let leap_day_eve = Date { year: 2028, month: 2, day: 28 };
        assert_eq!(leap_day_eve.add_days(1), Date { year: 2028, month: 2, day: 29 });
    }

    #[test]
    fn round_robin_pairs_every_team_exactly_once() {
        let teams = vec!["Arsenal", "Chelsea", "Liverpool", "Everton"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        let start = Date { year: 2026, month: 8, day: 23 };
        let fixtures = generate_round_robin(&teams, &start, 7).unwrap();

        // 4 teams -> 3 rounds of 2 matches each.
        assert_eq!(fixtures.len(), 6);

        let mut seen_pairs: HashSet<(String, String)> = HashSet::new();
        for fixture in &fixtures {
            let mut pair = [fixture.home.clone(), fixture.away.clone()];
            pair.sort();
            assert!(
                seen_pairs.insert((pair[0].clone(), pair[1].clone())),
                "pair {:?} appeared more than once",
                pair
            );
        }
        assert_eq!(seen_pairs.len(), 6);
    }

    #[test]
    fn round_robin_gives_every_team_a_bye_with_odd_count() {
        let teams = vec!["Arsenal", "Chelsea", "Liverpool"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        let start = Date { year: 2026, month: 8, day: 23 };
        let fixtures = generate_round_robin(&teams, &start, 7).unwrap();

        // 3 teams -> 3 rounds, one match each (one team byes per round).
        assert_eq!(fixtures.len(), 3);
        for fixture in &fixtures {
            assert!(fixture.home != "BYE" && fixture.away != "BYE");
        }
    }

    #[test]
    fn round_robin_rejects_duplicate_teams() {
        let teams = vec!["Arsenal", "arsenal"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        let start = Date { year: 2026, month: 8, day: 23 };
        assert!(generate_round_robin(&teams, &start, 7).is_err());
    }

    #[test]
    fn outcome_to_json_renders_fixtures_and_warnings() {
        let input = "2026-08-23,Arsenal,Chelsea\n2026-02-30,Foo,Bar\n";
        let options = ParseOptions {
            lenient: true,
            registry: None,
        };
        let outcome = parse_str(input, &options).unwrap();
        let json = outcome.to_json();
        assert_eq!(
            json,
            "{\"fixtures\":[{\"date\":\"2026-08-23\",\"home\":\"Arsenal\",\"away\":\"Chelsea\"}],\
             \"warnings\":[\"line 2: day 30 out of range for 2026-02 in '2026-02-30' (skipped)\"]}"
        );
    }

    #[test]
    fn json_escape_handles_quotes_and_backslashes() {
        assert_eq!(json_escape("a\"b\\c"), "a\\\"b\\\\c");
        assert_eq!(json_escape("tab\there"), "tab\\there");
    }

    #[test]
    fn round_robin_rejects_fewer_than_two_teams() {
        let teams = vec!["Arsenal".to_string()];
        let start = Date { year: 2026, month: 8, day: 23 };
        assert!(generate_round_robin(&teams, &start, 7).is_err());
    }
}
