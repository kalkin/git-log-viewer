Feature: Commit folding

    Scenario: Unfold commit 67c65a8
        Given foldable commit 67c65a8
         When I unfold commit
         Then commit is not folded

    Scenario: Proper levels
        Given foldable commit 67c65a8
          And I unfold commit
          When I walk over commits
          Then all commit levels should be 1
