# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).


## [v1.2.0] - 2020-04-13

### Added

- support for renamed subtrees
- make install target
- pygments >= 2.6.0 to the requirements
- docs: Add manual `glv(1)` (#10, #11)
- generate and install the manpage
- bind 'CTRL-L' to repainting the app
- bind 'n' & 'p' to next & previous search
- bind 'q' to exit the app in history view
- commit bar showing currently selected commit id
- resolve pull-request titles async without freezing ui
- add a diff view
- diff view use key bindings '{' & '}' for jumping between hunks
- make diff view searchable
- show diff stats in the diff view
- use a statusbar for showing search progress
- improvement: add own custom style


### Changed

- rename package to 'git-log-viewer'
- rename application to 'glv'

### Fixed

- tests #9 & disable subtree tests for now
- crash when cache file has invalid json
- CTRL-C exits anything
- missing Refs line in diff view
- whitespace in search highlighting

### Refactored

- remove dependency on the DIFF_VIEW global
- `Commit.branches()` return only branches pointing to itself
- `Commit.__stgit` protected-access
- get rid of asserts in code
- key bindings for log view do not depend on global `LOG_VIEW`
- log view to ui package
- Make mypy ♥
- make pylint ♥ by not using assert
- move history window code to history module
- move rendering of branchnames to `LogEntry`
- remove dead code
- remove unneeded Croasroads class
- remove unused CommitBar
- remove unused ForkPoint class
- remove unused self.noffff property
- remove unused vcs-show
- screen size functionality to utils module

## [v1.0.0] - 2020-03-19

### Added

- limiting logs to specific files
- support for vcs(1) modules
- vcs helper functions
- jump to first search match
- backward search
- vim like keybindings for j & k

### Changed

- module color to ansiyellow

### Refactored

- commit rendering

### Fixed

- getting terminal name from `$TERM`
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
