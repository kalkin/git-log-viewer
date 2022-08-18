# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [3.0.2] - 2022-08-18

### Changed

- Use '…' when shortening subject
- Remove separator between icon and graph
- parse provided path arguments relative to CWD

### Fixed

- Exit with error if no commits are found
- Tilted printing by moving the cursor to the beginning of the line
- Actually make update-informer optional

## [3.0.1] - 2022-05-06

### Added

- Hide refs/prefetch references
- Integrate update-informer

### Changed

- Hide timezone in history view

### Fixed

- Search does not crash anymore
- Search jump to branch & tag matches
- Show full subject message on ConventionalCommits of type deps
- Show correct committer name in the diff view

## [3.0.0-beta.8] - 2022-04-21

### Added

- Add highlighting for issue ids and scope
- Highlight the version on ConventionCommits of type Deps

## [3.0.0-beta.7] - 2022-03-06

### Changed

- Use `claps::derive` Args for command line parsing
- History window hide time when date > 7 days
- Reset screen before printing a panic

## [3.0.0-beta.6] - 2022-02-08

### Added

- Cache resolved Bitbucket and GitHub PR titles
- Add authentication for Bitbucket & GitHub
- Add Bitbucket Server API support
- Set log level via --debug argument

### Changed

- Reset icon after fetching commit subject from GitHub/BitBucket
- Replace some panics with `log::warn!`
- Obey GitHub API rate limiting rules

### Fixed

- Fix recognizing urls ending with .git
- Fix identifying forge url from scp style remotes
- Make GitHub integration work again
- Filter by path when delta(1) is not installed
- Display correctly commits without parents

## [3.0.0-beta.5] - 2022-01-17

### Added

- Add licensing information
- Update README.md

### Changed

- Highlight subtrees in yellow
- Exit with error code & message when repository init fails
- Handle gracefully invalid revisions

## [3.0.0-beta.4] - 2022-01-05

### Added

- Add support for 'dev' conventional commit icon
- Support git(1) like -C, --git-dir & --work-tree cli args

### Changed

- Fix clippy::manual-assert

### Fixed

- Accessing the command line argument --work-tree

## [3.0.0-beta.3] - 2021-09-30

### Added

- Display only diff for paths we are interested in

### Fixed

- Search use the provided paths as filter
- Do not crash on RGB color values

## [3.0.0-beta.2] - 2021-05-27

### Added

- Add README.md

### Changed

- Shorten subtree name if too long
- Show scope always in brackets

### Fixed

- DiffView use word Strees instead of Modules

## [3.0.0-beta.1] - 2021-05-23

### Added

- Shorten common prefix in remote branches
- Search in diff view is working again

### Changed

- Replace `cursive` ui library with `crossterm`. This changed forced me to
  rewrite the ui.
- Async recursive search. The search is still slow, but now it returns visual
  feedback about the progress.
- Mouse selection actually works now

### Fixed

- Fixed calculating fork point

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
