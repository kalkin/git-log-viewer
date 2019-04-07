# pylint: disable=missing-docstring,fixme
import json
import netrc
import os
import pathlib
import re
import sys
from typing import Any

import certifi
import urllib3


class Cache:
    def __init__(self, file_path: str) -> None:
        self._storage: dict = {}
        self._cache_file = file_path
        if os.path.isfile(file_path):
            with open(file_path, encoding='utf-8') as data_file:
                self._storage = json.loads(data_file.read())

    def __getitem__(self, key) -> Any:
        return self._storage[key]

    def __setitem__(self, key, value):
        self._storage[key] = value
        with open(self._cache_file, 'w') as outfile:
            json.dump(self._storage, outfile)


class Atlassian:
    def __init__(self, url: str, cache_dir: str) -> None:
        tmp = urllib3.util.parse_url(url)
        pathlib.Path(cache_dir).mkdir(parents=True, exist_ok=True)
        self._cache = Cache(cache_dir + '/bitbucket.json')

        self.auth_failed = False
        self.pattern = re.compile(r'#([0-9]+)')
        parts = tmp.path.split('/')
        name = parts[1].upper()
        self._http = urllib3.PoolManager(
            cert_reqs='CERT_REQUIRED', ca_certs=certifi.where())
        auth_store = netrc.netrc()
        auth_tupple = auth_store.authenticators(tmp.host)
        if auth_tupple:
            username = auth_tupple[0]
            password = auth_tupple[2]
            basic_auth = '%s:%s' % (username, password)
            self._headers = urllib3.make_headers(basic_auth=basic_auth)

        repo_name = parts[2]
        if repo_name.endswith('.git'):
            repo_name = repo_name[:-4]

        self._url = str(
            tmp._replace(
                path='/rest/api/1.0/projects/' + name + '/repos/' + repo_name,
                scheme='https',
                port=443))

    @staticmethod
    def enabled(identifier: str) -> bool:
        try:
            url = urllib3.util.parse_url(identifier)
            return url.hostname.startswith('bitbucket')
        except Exception:  # pylint: disable=broad-except
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
                tmp = self._url + '/pull-requests/' + _id
                request = self._http.request(
                    'GET',
                    tmp,
                    headers=self._headers,
                )
                if request.status == 200:
                    self._cache[subject] = json.loads(
                        request.data.decode('utf-8'))['title']
                elif request.status == 401:
                    print('Failed to authenticate', file=sys.stderr)
                    self.auth_failed = True
                    return subject
                else:
                    return subject

        return self._cache[subject]
