use std::env;
use std::fs;
use std::process::ExitCode;

use fixturelint::{parse_str, ParseOptions};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
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

fn print_usage() {
    eprintln!("usage: fixturelint <file> [--lenient]");
}
