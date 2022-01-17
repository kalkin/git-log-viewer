# Git Log Viewer

## About

An alternative to `tig(1)`/`lazygit(1) which supports folding merges and is
expandable via plugins. The application can resolve the default merge titles
done by using GitHub or Bitbucket to the actual pull request names.

## Installation

* `cargo install --git=https://github.com/kalkin/git-log-viewer --branch=rust-master`

## Usage

    USAGE:
        glv [OPTIONS] [REVISION] [-- <path>…]

    ARGS:
        <REVISION>    Branch, tag or commit id [default: HEAD]
        <path>…     Show only commits touching the paths

    OPTIONS:
        -C <dir>                          Change to <dir> before start
            --git-dir <git-dir>           Directory where the GIT_DIR is.
        -h, --help                        Print help information
        -V, --version                     Print version information
            --work-tree <working-tree>    Directory where the GIT_WORK_TREE is.

### Current State

This is a rewrite in rust. Many features are missing stil. It's work in
progress.
