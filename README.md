# fixturelint

Small local leagues pass fixture lists around as plain text or CSV, usually
copy-pasted between a spreadsheet and an email. In practice that means the
occasional typo'd date (`2026-02-30`), a row duplicated when someone pastes
twice, or a team accidentally listed as playing itself. Those errors are easy
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

## Library usage

```rust
use fixturelint::{parse_str, ParseOptions};

let outcome = parse_str(input, &ParseOptions { lenient: false })?;
for fixture in outcome.fixtures {
    println!("{}", fixture);
}
```

## What counts as invalid right now

- malformed lines (not exactly three comma-separated fields)
- dates that aren't zero-padded `YYYY-MM-DD`, or aren't calendar-valid
  (month 13, Feb 30, etc.)
- a team scheduled to play itself
- an exact duplicate fixture (same date, same two teams)

## Status

Early skeleton. No round-robin generation, no team-registry cross-checking,
no output formats beyond the default text listing yet.

## License

MIT, see LICENSE.
