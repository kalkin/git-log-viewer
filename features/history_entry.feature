Feature: History Entry

    Background:
        Given repo url https://github.com/QubesOS/qubes-antievilmaid

    Scenario: An Example non-merge Entry
        Given history entry for commit 5e313f4
        Then entry is not a merge
