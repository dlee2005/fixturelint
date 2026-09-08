# fixturelint

Small local leagues pass fixture lists around as plain text or CSV, usually
copy-pasted between a spreadsheet and an email. In practice that means the
occasional typo'd date (`2026-02-30`), a row duplicated when someone pastes
twice, a team accidentally listed as playing itself, or a team double-booked
for two different fixtures on the same date. Those errors are easy
to miss by eye and annoying to discover after they've already gone into a
calendar or a pitch-booking system.

`fixturelint` parses that format and refuses to let bad rows through
silently. It's strict by default: the first invalid line stops the parse
with a line number and a reason. When you just want to salvage whatever is
usable from a messy file, pass `--lenient` and bad lines are skipped with a
warning instead of aborting the whole import.

## Format

One fixture per line: `date,home,away`, with the date as zero-padded
`YYYY-MM-DD`. Blank lines and lines starting with `#` are ignored.

```
# round 1
2026-08-23,Arsenal,Chelsea
2026-08-24,Liverpool,Everton
```

## CLI usage

```
$ fixturelint fixtures.txt
2026-08-23  Arsenal vs Chelsea
2026-08-24  Liverpool vs Everton
2 fixture(s) parsed, 0 warning(s)
```

A bad file stops immediately and tells you why:

```
$ cat bad.txt
2026-08-23,Arsenal,Chelsea
2026-08-23,Arsenal,Chelsea
$ fixturelint bad.txt
2026-08-23  Arsenal vs Chelsea
error: line 2: duplicate fixture: Arsenal vs Chelsea on 2026-08-23
(pass --lenient to skip bad lines instead of failing)
```

With `--lenient` the same file imports what it can:

```
$ fixturelint --lenient bad.txt
2026-08-23  Arsenal vs Chelsea
warning: line 2: duplicate fixture: Arsenal vs Chelsea on 2026-08-23 (skipped)
1 fixture(s) parsed, 1 warning(s)
```

## Checking teams against a registry

Pass `--registry <teams-file>` to reject any fixture where the home or away
team isn't in a known list of teams (same one-name-per-line format the
`generate` subcommand reads). This catches a team name that's misspelled or
was renamed and never updated in the fixture list.

```
$ cat teams.txt
Arsenal
Chelsea
$ fixturelint --registry teams.txt fixtures.txt
error: line 1: team 'Chlesea' is not in the registry
(pass --lenient to skip bad lines instead of failing)
```

## JSON output

Pass `--format json` to get a single JSON object instead of the text listing,
for piping into another tool:

```
$ fixturelint --format json fixtures.txt
{"fixtures":[{"date":"2026-08-23","home":"Arsenal","away":"Chelsea"}],"warnings":[]}
```

A strict-mode failure prints a JSON error object instead of the usual
`error:` line, and the process still exits non-zero:

```
$ fixturelint --format json bad.txt
{"error":"line 2: duplicate fixture: Arsenal vs Chelsea on 2026-08-23"}
```

## Generating a round-robin schedule

Given a plain list of team names (one per line, `#` comments allowed), the
`generate` subcommand produces a single round-robin schedule — every team
plays every other team exactly once — using the standard circle method. Byes
are handled automatically for an odd number of teams.

```
$ cat teams.txt
Arsenal
Chelsea
Liverpool
Everton
$ fixturelint generate teams.txt --start-date 2026-08-23
2026-08-23,Arsenal,Everton
2026-08-23,Chelsea,Liverpool
2026-08-30,Liverpool,Arsenal
2026-08-30,Chelsea,Everton
2026-09-06,Arsenal,Chelsea
2026-09-06,Liverpool,Everton
```

Rounds are spaced a week apart by default; pass `--days-between-rounds N` to
change that. The output is in the same `date,home,away` format the parser
reads, so it can be piped straight into a file and fed back through
`fixturelint` for validation.

## Library usage

```rust
use fixturelint::{parse_str, ParseOptions};

let outcome = parse_str(input, &ParseOptions::default())?;
for fixture in outcome.fixtures {
    println!("{}", fixture);
}
println!("{}", outcome.to_json());
```

## What counts as invalid right now

- malformed lines (not exactly three comma-separated fields)
- dates that aren't zero-padded `YYYY-MM-DD`, or aren't calendar-valid
  (month 13, Feb 30, etc.)
- a team scheduled to play itself
- an exact duplicate fixture (same date, same two teams)
- a team double-booked with two different fixtures on the same date, even
  against different opponents
- a team not present in a supplied `--registry` file (only checked when
  `--registry` is passed)

## Status

Early skeleton. Round-robin generation, team-registry cross-checking,
same-day double-booking detection, and JSON output are in.

## License

MIT, see LICENSE.
