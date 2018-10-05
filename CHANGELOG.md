# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## Unreleased - 2018-10-05

### Changed

- folding toggling to `<tab>`
- opening/closing fold to left/right

### Fixed

- crash if commit message is empty
- detecting fast forward parent merge
- finding merge point relative to the main branch
- less exiting if output is smaller than a page
- performance by only listing last 100 commits
- pylint protected-access error
- return type of Repo.merge_base()
- showing merged folded subtrees properly

### Refactored

- opening pager
- order of imports in __init__.py

### Removed

- committed *.pyc file

