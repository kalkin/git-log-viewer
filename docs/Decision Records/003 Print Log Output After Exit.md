# Print Log Output After Exit

## Status

implemented *2022-10-01*

## Issue

Currently the logging is printed too `STDERR` and is deleted after exiting the
application by terminal reset.

## Context

Printing to `STDERR` disturbs the rendering of the application. The only sane
way to handle logging is currently to redirect it to a file. This is suboptimal

## Decision

Use `memory_logger` instead of `simple_logger`. The library adheres to the rust
`log` façade and reuse the same configuration logic for log levels. It allows
logging to memory and dumping the log after exit.

## Consequences

The contents of the log should be preserved on program exit.
