# pylint: disable=missing-docstring,invalid-name,unused-argument
import os

from lettuce import after, before, step, world
from pygit_viewer.line import Repo, Foldable, Commit


@before.each_scenario
def init_repo(scenario):
    world.repo = Repo(os.getcwd())
    world.commits = {}


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


@step('When I walk over commits')
def walk_over_commits(_):
    world.result = 0
    for _ in world.walker:
        # print(commit)
        world.result += 1
    assert world.result > 0


@step(r'Then I should have iterated over (\d+) commits?')
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
def compare_commit(self, sth):
    expected = world.repo.get(sth).oid
    actual = world.result.oid
    assert expected == actual, 'Got {}' % actual
