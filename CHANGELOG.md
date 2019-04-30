# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## Unreleased - 2019-04-30

### Added

- Display StGit commits with ’Ⓟ’

### Changed

- Use aloe instead of lettuce for running feature tests

## [v0.6.0] - 2019-04-26

### Added

- `REVISION` command line option
- preliminary tilix(1) terminal support
- `--workdir` commandline option

### Changed

- icons again
- Improve performance by using custom heuristics and memoization

### Fixed

- finding `descendant_of`
- lettuce tests
- rendering subtree import forkpoint
- typo in features/steps.py
