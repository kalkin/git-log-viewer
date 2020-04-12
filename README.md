# A Git Log Viewer

## About

An alternative to `tig(1)`/`lazygit(1) which supports folding merges and is
expandable via plugins. The application can resolve the default merge titles
done by using GitHub or Bitbucket to the actual pull request names.

## Installation

* `git clone https://github.com/kalkin/git-log-viewer.git`
* `cd git-log-viewer`
* `pip3 install --user .`

## Update

* `cd git-log-viewer`
* `git pull --rebase`
* `pip3 install --user --upgrade .`

## Usage

    glv [--workdir=DIR] [REVISION] [-d | --debug] [[--] <path>...]
    glv --version

### Options

    REVISION        A branch, tag or commit [default: HEAD]
    --workdir=DIR   Directory where the git repository is
    -d --debug      Enable sending debuggin output to journalctl
                    (journalctl --user -f)

### Current State

This piece of software works for me and is pretty stable.

### Debugging

When `-d` option provided the application will log debug data to
`journalctl(1)`. You can follow it like this:

    journalctl --user -f
