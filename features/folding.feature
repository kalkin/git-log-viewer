Feature: Commit folding

    Scenario: Unfold commit ce537d3
        Given foldable commit ce537d3
         When I unfold commit
         Then commit is not folded

    Scenario: Proper levels
        Given foldable commit ce537d3
          And I unfold commit
          When I walk over commits
          Then all commit levels should be 1
