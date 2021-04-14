Feature: Repository Walker
    In order to get commits we need an abstraction layer for the `pygit2.Repositor`.

    Background:
        Given repo url https://github.com/QubesOS/qubes-antievilmaid

    Scenario: Count commits between HEAD & HEAD~10
        Given range HEAD~10..HEAD
        Then I should have 10 commits

    Scenario: Count commits between HEAD~3 & HEAD~10
        Given range HEAD~10..HEAD~3
        Then I should have 7 commits
