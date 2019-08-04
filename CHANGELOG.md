# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## Unreleased - 2019-08-04

### Added

- limiting logs to specific files
- support for vcs(1) modules
- vcs helper functions
- jump to first search match
- backward search

### Changed

- module color to ansiyellow

### Refactored

- commit rendering

### Fixed

- Fix search in pygit-viewer
- duplicate walker initialization
- goto line
- performance for module recognition
- exit if not in git repo
- Show module name in subject if it's unknown
- going beyound 0 line
- KeyError when no commits match path filter
- KeyError when revision not found

## [v0.7.0] - 2019-04-30

### Added

    - Display StGit commits with ’Ⓟ’
- Keybindings for `<Home>` & `<End>` (#7)

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
