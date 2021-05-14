# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [3.0.0-alpha.6] - 2021-05-14

### Added

- Debug impl to fork point Request/Response

### Changed

- Add `ForkPointThread::request_calculation()`
- Subtree operation recognition
- Migrate to clap@v3.0.0-beta.2 for argument parsing

### Fixed

- Showing subject module together with subtrees
- style: Fix `clippy::must-use-candidate`

### Refactored

- out `color_span` to style module
- out reversing test style
- Reuse `ForkPointThread::request_calculation()`

## [3.0.0-alpha.5] - 2021-04-28

### Changed

- `adjust_string()` use match instead of an if chain
- Fix calls to `git_cmd_out`
- Fix `clippy::if-not-else`
- Fix `clippy::inefficient-to-string`
- Fix `clippy::needless-pass-by-value`
- Fix `clippy::needless-pass-by-value`
- Fix `clippy::shadow-unrelated`
- Fix `clippy::unwrap_used`
- Make clippy ♥

### Fixed

- docs: Fix `clippy::missing-errors-doc`
- style: Fix `clippy::default-trait-access`
- style: Fix `clippy::enum-glob-use`
- style: Fix `clippy::explicit-into-iter-loop`
- style: Fix `clippy::explicit-iter-loop`
- style: Fix `clippy::match-wild-err-arm`
- style: Fix `clippy::module-name-repetitions`
- style: Fix `clippy::redundant-closure-for-method-calls`
- style: Fix `clippy::redundant-closure-for-method-calls`
- style: Fix `clippy::wildcard-imports`
- tests: Fix tests

### Refactored

- Fix `clippy::option-if-let-else`
- `ForkPointThread` implement `Default` trait
- If/else branches to make clippy ♥

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
