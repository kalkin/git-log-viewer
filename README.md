# A tig(1) alternative

## 2018-08-25

The current version uses prompt_toolkit’s `TextArea` widget to just dump the
commit log as a bunch of text lines. All manipulation for folding are done via
inserting lines in to a huge string.

## History

After a few tries I finally ended up with the Python programming language.
Currently (2018-08-25) python hits the sweet point because it provides a good
wrapper around libgit2 (pygit2), a good terminal library prompt_toolkit and is a
dependency brought in by many operating systems.

The first draft used urwid library and the extension library urwidtrees. Sadly
git data structures fit very poorly for the default `TreeWidget`s.

The second current (2018-08-25) version uses the new prompt_toolkit 2.0 library
