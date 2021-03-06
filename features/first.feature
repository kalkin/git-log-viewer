Feature: Repository Walker
    In order to get commits we need an abstraction layer for the `pygit2.Repositor`.

    Scenario: Count commits between HEAD & HEAD~10
        Given commit START (HEAD)
        And commit LAST (HEAD~10)
        And a walker over commits between START & LAST
        When I walk over commits
        Then I should have iterated over 10 commits

    Scenario: Count commits between HEAD~3 & HEAD~10
        Given commit START (HEAD~3)
        And commit LAST (HEAD~10)
        And a walker over commits between START & LAST
        When I walk over commits
        Then I should have iterated over 7 commits

    Scenario: Iterate over commits of the first parent
        Given foldable commit 67c65a8
        And a walker over commit children
        When I walk over commits
        Then i should have iterated over 6 commits

    Scenario: Find merge base
        Given commit A (b4ea5ef)
        And commit B (79185f3)
        When I calculate merge base of A & B
        Then the result commit should be 79185f3
