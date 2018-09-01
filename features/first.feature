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
        Given foldable commit ce537d3
        And a walker over commit children
        When I walk over commits
        then i should have iterated over 3 commits

    Scenario: Find merge base
        Given commit A (fc0f00a)
        And commit B (5126d60)
        When I calculate merge base of A & B
        Then the result commit should be 5126d60
