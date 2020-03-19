# A Git Log Viewer

## About

An alternative to tig / gvc which supports folding the merges and is expandable
via plugins.

## Installation

* `git clone https://github.com/kalkin/pygit-viewer.git`
* `cd pygit-viewer`
* `pip3 install --user --upgrade .`

## Update

* `cd pygit-viewer`
* `git pull`
* `pip3 install --user --upgrade .`

## Usage

    pygit_viewer [--workdir=DIR] [REVISION] [-d | --debug]
    pygit_viewer --version

### Options

    REVISION        A branch, tag or commit [default: HEAD]
    --workdir=DIR   Directory where the git repository is
    -d --debug      Enable sending debuggin output to journalctl

### Current State

This piece of software works for me. I will not invest any more time besides
fixing annoying bugs. I'm currently working on a more performant implementation
in Ada.

### Debugging

When `-d` option provided the application will log debug data to
`journalctl(1)`. You can follow it like this:

    journalctl --user -f
