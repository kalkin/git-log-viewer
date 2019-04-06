# pylint: disable=missing-docstring,invalid-name,unused-argument
import os

from lettuce import after, before, step, world

from pygit_viewer import Commit, Foldable, RebasedMerge, Repo


@before.each_scenario
def init_repo(scenario):
    world.repo = Repo(os.getcwd())
    world.commits = {}
    world.passed_commits = []


@after.each_scenario
def unset_world(scenario):
    world.repo = None
    world.walker = None
    world.commit = None
    try:
        del world.result
    except Exception:  # pylint: disable=broad-except
        pass


def parse_path(text: str) -> str:
    if text == 'current working directory':
        return os.getcwd()
    else:
        assert False, 'Not implemented yet.'


@step(r'(?:a )?walker over commits between (\w+) & (\w+)')
def walker_over_commits(_, first, last):
    start = world.commits[first]
    end = world.commits[last]
    world.walker = world.repo.walker(start, end)


@step('I walk over commits')
def walk_over_commits(_):
    world.result = 0
    for commit in world.walker:
        # print(commit)
        world.passed_commits.append(commit)
        world.result += 1
    assert world.result > 0


@step(r'I should have iterated over (\d+) commits?')
def assert_number_of_commits(_, expected):
    assert world.result == int(expected), 'Expected: ' + str(
        expected) + ' / Got: ' + str(world.result)


@step(r'foldable commit (\w+)')
def foldable_commit(_, sth):
    world.commit = world.repo.get(sth)
    assert isinstance(world.commit, Foldable), 'A foldable commit'


@step(r'commit (\w+) \(([\w~]+)\)')
def any_commit(_, name, sth):
    world.commits[name] = world.repo.get(sth)
    assert isinstance(world.commits[name], Commit), 'Got commit ' + str(sth)


@step(u'And a walker over commit children')
def children_walker(_):
    world.walker = world.commit.child_log()


@step(r'I calculate merge base of (\w+) & (\w+)')
def merge_base(_, a, b):
    a = world.commits[a]
    b = world.commits[b]

    # pylint: disable=protected-access
    world.result = world.repo.merge_base(a._commit, b._commit)
    assert isinstance(world.result, Commit), 'Got commit ' + str(world.result)


@step(r'Then the result commit should be (\w+)')
def compare_commit(_, sth):
    expected = world.repo.get(sth).oid
    actual = world.result.oid
    assert expected == actual, 'Got {}' % actual


@step(r'last child commit should be (\w+)')
def last_commit_id(_, expected):
    result = world.passed_commits[-1].short_id()
    assert result == expected, "Expected: %s got %s" % (expected, result)


@step(r'rebased-merge commit (\w+)')
def rabesed_merge(_, sth):
    world.commit = world.repo.get(sth)
    assert isinstance(
        world.commit, RebasedMerge
    ), 'Expected a RebasedMerge got %s' % world.commit.__class__.__name__


@step(r'last child class should be (\w+)')
def last_child_class(_, expected):
    assert world.passed_commits, "No passed commits"
    result = world.passed_commits[-1].__class__.__name__
    assert result == expected, "Expected: %s got %s" % (expected, result)


@step(r'next class (?:should be|is a) (\w+)')
def next_class(_, expected):
    result = world.repo.first_parent(world.commit).__class__.__name__
    assert result == expected, "Expected: %s got %s" % (expected, result)


@step(r'next class should not be (\w+)')
def next_not_class(_, expected):
    result = world.repo.first_parent(world.commit).__class__.__name__
    assert result != expected, "Expected: Not %s got %s" % (expected, result)


@step(r'I unfold commit')
def unfold_commit(_):
    world.commit.unfold()
    world.walker = world.commit.child_log()


@step(r'all commit levels should be (\d+)')
def check_level(_, level):
    for commit in world.passed_commits:
        assert commit.level == int(
            level), 'Expected ' + level + ' / Got ' + str(commit.level)


@step('commit is not folded')
def not_folded(_):
    assert world.commit.is_folded is False


@step(r'Then next is (\w+)')
def next_is(_, expected):
    result = world.repo.first_parent(world.commit).short_id()
    assert result == expected, "Expected: %s got %s" % (expected, result)
