# pylint: disable=missing-docstring,invalid-name,unused-argument
import os

from lettuce import step, world
from pygit_viewer.line import Repo


@step('(?:Given|And) a repository in (.*)')
def init_repo(step, path: str):
    path: str = parse_path(path)
    world.repo = Repo(os.getcwd())
    assert world.repo, 'Get a Repo instance'

def parse_path(text: str) -> str:
    if text == 'current working directory':
        return os.getcwd()
    else:
        assert False, 'Not implemented yet.'

@step('(?:Given|And) starting commit (.*)')
def define_start_commit(step, oid: str):
    world.start = oid
    assert world.start, 'Got oid'


@step('And last commit (.*)')
def define_last_commit(step, oid: str):
    world.last = oid
    assert world.last, 'No oid provided'


@step('When I walk over commits')
def b_when_i_walk_over_commits(step):
    world.walker = world.repo.walker(world.start, world.last)
    assert world.walker, 'Got walker'


@step(u'Then I should have iterated over (\d+) commits')
def b_then_i_should_have_iterate_over_10_commits(step, expected):
    expected = int(expected)
    actual = 0
    for commit in world.walker:
        actual += 1

    assert actual == expected, 'Expected: ' + str(expected) + ' / Got: ' + str(actual)
