# To do

## Refactor

* ~~move `fork_point` field from `Commit` to `HistoryEntry`~~

## Testing

* ~~Separate config stuff from `core.rs` to `config.rs`~~
* Handle *Subject* as own `struct` or `enum` to
   distinguish between different subject options.
* `HistoryEntry::folded` should be an enum with values `Yes, No, Invalid`
* ~~`HistoryEntry` add `has_children`~~

## Benchmarks

* How do benches work in rust?
* Benchmark how long it takes to read my monorepo to Commits
* Benchmark how long it takes to read my monorepo to HistoryEntries

## History View

### Branches rework

* Move branch names before subject
* Other references should be after subject
* Color local branches different from remote.
   For this we can use `git branch -a` output

### Date display

* Show the freshest date from Author and Committer date-fields

### Horizontal Scrolling
