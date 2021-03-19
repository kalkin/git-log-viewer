Feature: Parse revision specified on command line

    Scenario: HEAD
        Given command line: glv HEAD
        Then found revision

    Scenario: Branch
        Given command line: glv master
        Then found revision

    Scenario: Short Commit
        Given command line: glv 03de9b6
        Then found revision

    Scenario: Short Commit Range
        Given command line: glv 840a98e..03de9b6
        When found revision
        Then revision log has 9 commits

    Scenario: Open Range
        Given command line: glv 840a98e..
        When found revision
        Then revision log has more then 9 commits

    Scenario: HEAD Range
        Given command line: glv HEAD~2..HEAD
        When found revision
        Then revision log has 2 commits

    Scenario: HEAD Open range
        Given command line: glv HEAD~2..
        When found revision
        Then revision log has 2 commits
