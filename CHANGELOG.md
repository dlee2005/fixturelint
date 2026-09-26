# Changelog

## 0.1.0

First release.

- `date,home,away` fixture list parsing, strict by default
- rejects malformed lines, calendar-invalid dates, a team playing itself,
  exact duplicate fixtures, and a team double-booked on the same date
- `--lenient` skips bad lines and collects warnings instead of aborting
- `--registry <teams-file>` rejects fixtures with a team not in a known list
- `--format json` for a single JSON object instead of the text listing
- `--sort-by-date` to order output by date instead of file order
- `generate` subcommand: round-robin schedule generation from a team list
