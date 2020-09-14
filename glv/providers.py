# pylint: disable=missing-docstring,fixme
#
# Copyright (c) 2018-2020 Bahtiar `kalkin-` Gadimov.
#
# This file is part of Git Log Viewer
# (see https://github.com/kalkin/git-log-viewer).
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as
# published by the Free Software Foundation, either version 3 of the
# License, or (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <http://www.gnu.org/licenses/>.
#
import json
import logging
import netrc
import os
import pathlib
import re
import sys
from datetime import datetime
from time import time
from typing import Any, Optional, Tuple

import certifi
import urllib3  # type: ignore

LOG = logging.getLogger('glv')


class Cache:
    def __init__(self, file_path: str) -> None:
        self._storage: dict = {}
        cache_dir = os.path.dirname(file_path)
        self._cache_file = file_path
        self._ro_backend = False

        try:
            pathlib.Path(cache_dir).mkdir(parents=True, exist_ok=True)
            if os.path.isfile(file_path):  # restore cache
                with open(file_path, encoding='utf-8') as data_file:
                    self._storage = json.loads(data_file.read())
        except PermissionError:
            LOG.warning('Read only git-dir, no data will be cached')
            self._ro_backend = True
        except json.decoder.JSONDecodeError as exc:
            LOG.warning('Failed to parse %s: %s', data_file.name, exc.msg)

    def __getitem__(self, key) -> Any:
        return self._storage[key]

    def __setitem__(self, key, value):
        self._storage[key] = value
        if not self._ro_backend:
            with open(self._cache_file, 'w') as outfile:
                json.dump(self._storage, outfile)


class Provider():
    def __init__(self, repo, cache: Cache) -> None:
        self._cache = cache
        if isinstance(repo, str):
            url = repo
        else:
            url = repo.remotes['origin'].url

        self._url = urllib3.util.parse_url(url)
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
        except netrc.NetrcParseError as e:
            print(str(e), file=sys.stderr)
            auth_tupple = None
        except FileNotFoundError:
            auth_tupple = None

        if auth_tupple:
            if auth_tupple is not None:
                return (auth_tupple[0], auth_tupple[2])  # type: ignore
            return (auth_tupple[0], '')

        return None


class GitHub(Provider):
    def __init__(self, repo, cache_dir: str) -> None:
        self._rate_limit: Optional[int] = None
        file_path = os.path.join(cache_dir, 'github.json')
        super().__init__(repo, Cache(file_path))

        self.pattern = re.compile(r'#([0-9]+)')
        auth_tupple = self.authorization()
        if auth_tupple:
            basic_auth = '%s:%s' % auth_tupple
            self._headers = urllib3.make_headers(basic_auth=basic_auth,
                                                 user_agent='glv')
        else:
            self._headers = urllib3.make_headers(user_agent='glv')

        parts = self._url.path.split('/')
        owner = parts[1]
        name = parts[2]
        if name.endswith('.git'):
            name = os.path.splitext(name)[0]
        self._api_url = 'https://api.github.com/repos/%s/%s/pulls/' % (owner,
                                                                       name)

    @staticmethod
    def enabled(repo) -> bool:
        result = False
        try:
            if isinstance(repo, str):
                _url = repo
            elif repo.remotes:
                _url = repo.remotes['origin'].url
            url = urllib3.util.parse_url(_url)
            result = url.hostname == 'github.com'
        except Exception:  # pylint: disable=broad-except
            pass
        LOG.debug('github-api: enabled %s', result)
        return result

    def has_match(self, subject: str) -> bool:
        return bool(self.pattern.search(subject))

    def provide(self, subject: str) -> str:
        try:
            return self._cache[subject]
        except KeyError:
            if (self._rate_limit and self._rate_limit >= time()) \
                    or self.auth_failed:
                return subject

            if self._rate_limit:
                self._rate_limit = None

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
                    LOG.debug('github-api: \ue27d #%s', _id)
                    json_data = json.loads(request.data.decode('utf-8'))
                    self._cache[subject] = json_data['title'] + ' (#%s)' % _id
                elif request.status == 401:
                    LOG.error('github-api: ⛔ authentication failure')
                    self.auth_failed = True
                    return subject
                elif request.status == 403:
                    self._rate_limit = int(
                        request.headers['X-Ratelimit-Reset'])
                    date = datetime.utcfromtimestamp(self._rate_limit)
                    LOG.warning('github-api: ⚠ rate limited until %s', date)
                    return subject
                else:
                    LOG.error('github-api ⛔ (%s) %s', request.status,
                              request.data)
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
        else:
            self._headers = urllib3.make_headers(user_agent='glv')

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
                    LOG.error('Failed to authenticate')
                    self.auth_failed = True
                    return subject
                else:
                    return subject

        return self._cache[subject]
