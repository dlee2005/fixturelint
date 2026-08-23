use std::env;
use std::fs;
use std::process::ExitCode;

use fixturelint::{generate_round_robin, parse_str, Date, ParseOptions};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.first().map(String::as_str) == Some("generate") {
        return run_generate(&args[1..]);
    }

    let mut lenient = false;
    let mut path: Option<String> = None;

    for arg in args {
        match arg.as_str() {
            "--lenient" => lenient = true,
            "-h" | "--help" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            other if path.is_none() => path = Some(other.to_string()),
            other => {
                eprintln!("unexpected argument: {}", other);
                print_usage();
                return ExitCode::FAILURE;
            }
        }
    }

    let path = match path {
        Some(p) => p,
        None => {
            print_usage();
            return ExitCode::FAILURE;
        }
    };

    let input = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to read '{}': {}", path, e);
            return ExitCode::FAILURE;
        }
    };

    let options = ParseOptions { lenient };
    match parse_str(&input, &options) {
        Ok(outcome) => {
            for fixture in &outcome.fixtures {
                println!("{}", fixture);
            }
            for warning in &outcome.warnings {
                eprintln!("warning: {}", warning);
            }
            eprintln!(
                "{} fixture(s) parsed, {} warning(s)",
                outcome.fixtures.len(),
                outcome.warnings.len()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {}", e);
            eprintln!("(pass --lenient to skip bad lines instead of failing)");
            ExitCode::FAILURE
        }
    }
}

fn run_generate(args: &[String]) -> ExitCode {
    let mut teams_path: Option<String> = None;
    let mut start_date: Option<String> = None;
    let mut days_between_rounds: u32 = 7;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--start-date" => {
                i += 1;
                start_date = args.get(i).cloned();
            }
            "--days-between-rounds" => {
                i += 1;
                days_between_rounds = match args.get(i).and_then(|v| v.parse().ok()) {
                    Some(n) => n,
                    None => {
                        eprintln!("--days-between-rounds requires a positive integer");
                        return ExitCode::FAILURE;
                    }
                };
            }
            "-h" | "--help" => {
                print_generate_usage();
                return ExitCode::SUCCESS;
            }
            other if teams_path.is_none() => teams_path = Some(other.to_string()),
            other => {
                eprintln!("unexpected argument: {}", other);
                print_generate_usage();
                return ExitCode::FAILURE;
            }
        }
        i += 1;
    }

    let teams_path = match teams_path {
        Some(p) => p,
        None => {
            print_generate_usage();
            return ExitCode::FAILURE;
        }
    };
    let start_date = match start_date {
        Some(d) => d,
        None => {
            eprintln!("--start-date YYYY-MM-DD is required");
            print_generate_usage();
            return ExitCode::FAILURE;
        }
    };

    let start = match Date::parse(&start_date) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("invalid --start-date: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let teams_input = match fs::read_to_string(&teams_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to read '{}': {}", teams_path, e);
            return ExitCode::FAILURE;
        }
    };
    let teams: Vec<String> = teams_input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(String::from)
        .collect();

    match generate_round_robin(&teams, &start, days_between_rounds) {
        Ok(fixtures) => {
            for fixture in &fixtures {
                println!("{},{},{}", fixture.date, fixture.home, fixture.away);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn print_usage() {
    eprintln!("usage: fixturelint <file> [--lenient]");
    eprintln!("       fixturelint generate <teams-file> --start-date YYYY-MM-DD [--days-between-rounds N]");
}

fn print_generate_usage() {
    eprintln!("usage: fixturelint generate <teams-file> --start-date YYYY-MM-DD [--days-between-rounds N]");
    eprintln!("teams-file: one team name per line; blank lines and lines starting with # are ignored");
}
