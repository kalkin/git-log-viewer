Feature: History Entry

    Background:
        Given repo url https://github.com/QubesOS/qubes-antievilmaid

    Scenario: An Example non-merge Entry
        Given history entry for commit 5e313f4
        Then entry is not foldable

    Scenario: A fork point History Entry
        Given history entries for range 9b4d3f2~1..82a98b1
        When fork point calculation done
        Then entry with index 1 is a fork point
