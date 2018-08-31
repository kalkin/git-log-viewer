Feature: Repository Walker
  In order to get commits we need an abstraction layer for the `pygit2.Repositor`.

  Scenario: Iterate over part of repository
    Given a repository in current working directory
      And starting commit HEAD
      And last commit HEAD~10
     When I walk over commits
     Then I should have iterated over 10 commits

  Scenario: Iterate over part2 of repository
    Given starting commit HEAD~3
      And last commit HEAD~10
     When I walk over commits
     Then I should have iterated over 10 commits
