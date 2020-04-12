# A Git Log Viewer

## About

An alternative to `tig(1)`/`gvc(1) which supports folding the merges and is
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

This piece of software works for me and is pretty stable. The biggest issue with
the current implementation is that it is not async. The following issues are
arise from this and pull request for them are welcome:

* Search will freeze the whole application until it matched something, even if
  it has to iterate over a *whole* history.
* Resolving merges to a pull request title can take some time.

### Debugging

When `-d` option provided the application will log debug data to
`journalctl(1)`. You can follow it like this:

    journalctl --user -f
