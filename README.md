# A Git Log Viewer

## About

An alternative to tig / gvc which supports folding the merges and is expandable
via plugins.

## Installation

* `git clone https://github.com/kalkin/pygit-viewer.git`
* `cd pygit-viewer`
* `pip3 install --user --update .`

## Usage

    pygit_viewer [--workdir=DIR] [REVISION] [-d | --debug]
    pygit_viewer --version

### Options

    REVISION        A branch, tag or commit [default: HEAD]
    --workdir=DIR   Directory where the git repository is
    -d --debug      Enable sending debuggin output to journalctl
