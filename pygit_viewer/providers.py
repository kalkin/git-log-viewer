# pylint: disable=missing-docstring,fixme
import json
import logging
import netrc
import os
import pathlib
import re
import sys
from typing import Any, Optional, Tuple

import certifi
import urllib3  # type: ignore

LOG = logging.getLogger('pygit-viewer')


class Cache:
    def __init__(self, file_path: str) -> None:
        cache_dir = os.path.dirname(file_path)
        pathlib.Path(cache_dir).mkdir(parents=True, exist_ok=True)
        self._storage: dict = {}
        self._cache_file = file_path
        if os.path.isfile(file_path):
            with open(file_path, encoding='utf-8') as data_file:
                try:
                    self._storage = json.loads(data_file.read())
                except json.decoder.JSONDecodeError as e:
                    LOG.warning('Failed to parse %s: %s', data_file.name,
                                e.msg)
                    self._storage = {}

    def __getitem__(self, key) -> Any:
        return self._storage[key]

    def __setitem__(self, key, value):
        self._storage[key] = value
        with open(self._cache_file, 'w') as outfile:
            json.dump(self._storage, outfile)


class Provider():
    def __init__(self, repo, cache: Cache) -> None:
        self._cache = cache
        self._repo = repo
        self._url = urllib3.util.parse_url(self._repo.remotes['origin'].url)
        self.auth_failed = False
        self._http = urllib3.PoolManager(cert_reqs='CERT_REQUIRED',
                                         ca_certs=certifi.where())

    @staticmethod
    def enabled(repo) -> bool:
        raise NotImplementedError

    def has_match(self, subject: str) -> bool:
        raise NotImplementedError

    def provide(self, subject: str) -> str:
        raise NotImplementedError

    def authorization(self) -> Optional[Tuple[str, str]]:
        try:
            auth_store = netrc.netrc()
            auth_tupple = auth_store.authenticators(self._url.host)
        except (FileNotFoundError):
            auth_tupple = None

        if auth_tupple:
            if auth_tupple is not None:
                return (auth_tupple[0], auth_tupple[2])  # type: ignore
            return (auth_tupple[0], '')

        return None


class GitHub(Provider):
    def __init__(self, repo, cache_dir: str) -> None:
        file_path = os.path.join(cache_dir, 'github.json')
        super().__init__(repo, Cache(file_path))

        self.pattern = re.compile(r'#([0-9]+)')
        auth_tupple = self.authorization()
        if auth_tupple:
            basic_auth = '%s:%s' % auth_tupple
            self._headers = urllib3.make_headers(basic_auth=basic_auth,
                                                 user_agent='pygit-viewer')

        parts = self._url.path.split('/')
        owner = parts[1]
        name = parts[2]
        self._api_url = 'https://api.github.com/repos/%s/%s/pulls/' % (owner,
                                                                       name)

    @staticmethod
    def enabled(repo) -> bool:
        try:
            if repo.remotes:
                url = urllib3.util.parse_url(repo.remotes['origin'].url)
                return url.hostname == 'github.com'
        except Exception:  # pylint: disable=broad-except
            pass
        return False

    def has_match(self, subject: str) -> bool:
        return bool(self.pattern.search(subject))

    def provide(self, subject: str) -> str:
        try:
            return self._cache[subject]
        except KeyError:
            if self.auth_failed:
                return subject

            results = self.pattern.search(subject)
            if results:
                _id = results.group(1)
                tmp = self._api_url + _id
                request = self._http.request(
                    'GET',
                    tmp,
                    headers=self._headers,
                )
                if request.status == 200:
                    self._cache[subject] = json.loads(
                        request.data.decode('utf-8'))['title'] + ' (#%s)' % _id
                elif request.status == 401:
                    print('Failed to authenticate', file=sys.stderr)
                    self.auth_failed = True
                    return subject
                else:
                    print(request.data, file=sys.stderr)
                    return subject

        return self._cache[subject]


class Atlassian(Provider):
    def __init__(self, repo, cache_dir: str) -> None:
        file_path = os.path.join(cache_dir, 'bitbucket.json')
        super().__init__(repo, Cache(file_path))

        self.pattern = re.compile(r'#([0-9]+)')
        auth_tupple = self.authorization()
        if auth_tupple:
            basic_auth = '%s:%s' % auth_tupple
            self._headers = urllib3.make_headers(basic_auth=basic_auth)

        parts = self._url.path.split('/')
        name = parts[1].upper()
        repo_name = parts[2]
        if repo_name.endswith('.git'):
            repo_name = repo_name[:-4]

        self._api_url = str(
            self._url._replace(path='/rest/api/1.0/projects/' + name +
                               '/repos/' + repo_name,
                               scheme='https',
                               port=443))

    @staticmethod
    def enabled(repo) -> bool:
        try:
            if repo.remotes:
                url = urllib3.util.parse_url(repo.remotes['origin'].url)
                return url.hostname.startswith('bitbucket')
        except Exception:  # pylint: disable=broad-except
            pass
        return False

    def has_match(self, subject: str) -> bool:
        return bool(self.pattern.search(subject))

    def provide(self, subject: str) -> str:
        try:
            return self._cache[subject]
        except KeyError:
            if self.auth_failed:
                return subject

            results = self.pattern.search(subject)
            if results:
                _id = results.group(1)
                tmp = self._api_url + '/pull-requests/' + _id
                request = self._http.request(
                    'GET',
                    tmp,
                    headers=self._headers,
                )
                if request.status == 200:
                    self._cache[subject] = json.loads(
                        request.data.decode('utf-8'))['title'] + ' (#%s)' % _id
                elif request.status == 401:
                    print('Failed to authenticate', file=sys.stderr)
                    self.auth_failed = True
                    return subject
                else:
                    return subject

        return self._cache[subject]
