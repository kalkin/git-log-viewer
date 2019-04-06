Feature: Identifying different commit types


    Scenario: Subtree Pull
        Given foldable commit ae8e713
        And a walker over commit children
        When I unfold commit
         And I walk over commits
        Then last child commit should be eab9745
         And last child class should be LastCommit

    Scenario: Rebased Merge handling
        Given rebased-merge commit a0167a1
          And next class is a ForkPoint
        When I unfold commit
         And I walk over commits
         Then last child class should be Commit

    Scenario: Subtree Import
        Given foldable commit b818a16
        When I unfold commit
         And I walk over commits
        Then last child commit should be dd3cf00
         And last child class should be InitialCommit

    Scenario: Subtree Import
        Given foldable commit b818a16
        Then next is 38b9c56
        And next class should not be ForkPoint
        And next class should be Foldable
