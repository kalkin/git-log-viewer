Feature: Identifying different commit types


    Scenario: Subtree Pull
        Given foldable commit ae8e713
        And a walker over commit children
        When I unfold commit
         And I walk over commits
        Then last child commit should be 67078cc
         And last child class should be CommitLink

    Scenario: Rebased Merge handling
        Given rebased-merge commit a0167a1
          And next commit is a fork-point
        When I unfold commit
         And I walk over commits
         Then last child class should be Commit

    Scenario: Subtree Import
        Given foldable commit b818a16
        When I unfold commit
         And I walk over commits
        Then last child commit should be dd3cf00
         And last child class should be InitialCommit

    Scenario: Subtree Import 2
        Given foldable commit b818a16
        Then next is 38b9c56
        And next class should not be fork-point
        And next class should be Merge
