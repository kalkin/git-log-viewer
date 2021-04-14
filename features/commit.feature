Feature: Commit data structure validation

    Background:
        Given repo url https://github.com/QubesOS/qubes-antievilmaid

    Scenario: Check for master references
        Given commit HEAD
        Then commit has reference “origin/master”
        And commit has reference “master”

    Scenario: HEAD is master branch
        Given commit HEAD
        Then commit has branch “origin/master”
        And commit has branch “master”
        And commit is head

    Scenario: Author & Committer fields
        Given commit 49ed713
        When bellow commit is 59e8dcf
        Then commit author name is “Marek Marczykowski-Górecki”
        And commit author email is “marmarek@invisiblethingslab.com”
        And commit committer name is “Marek Marczykowski-Górecki”
        And commit committer email is “marmarek@invisiblethingslab.com”
        And commit subject is “Merge remote-tracking branch 'qubesos/pr/18'”

    Scenario: Inspect merge commit
        Given commit 49ed713
        When bellow commit is 59e8dcf
        Then commit has 1 child commit