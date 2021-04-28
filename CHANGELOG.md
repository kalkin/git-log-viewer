# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [3.0.0-alpha.4] - 2021-04-28

### Changed

- Date column use human relative date

### Fixed

- test: Fix BDD tests
- tests: BDD fix matching “Given commit for…”

### Refactored

- `ForkPointThread`
- `HistoryEntry` use `self.is_fork_point()`
- Limit the usage of `HistoryEntry.commit()`
- Move `adjust_string()` to `history_entry.rs`
- Move config stuff to own modules
- Move `fork_point` field from `Commit` → `HistoryEntry`
- Replace `HistoryEntry.commit_mut` with `set_fork_point`

### Removed

- unneeded clone in History.toggle_folding()
