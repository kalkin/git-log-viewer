#!/usr/bin/env python
''' Setup script '''

from setuptools import find_packages, setup

with open('requirements.txt') as req_file:
    REQUIREMENTS = req_file.read()

with open('test-requirements.txt') as req_file:
    TEST_REQUIREMENTS = req_file.read()

setup(
    name="glv",
    author="Bahtiar `kalkin-` Gadimov",
    author_email="bahtiar@gadimov.de",
    python_requires='>=3.7',
    url="https://github.com/kalkin/git-log-viewer",
    classifiers=[
        "Operating System :: POSIX",
        'Programming Language :: Python :: 3',
        'Programming Language :: Python :: 3.7',
        'Programming Language :: Python :: 3.8',
        'Programming Language :: Python :: 3.9',
        'License :: OSI Approved :: GNU Affero General Public License v3 or later (AGPLv3+):',  # noqa: E501
        'Topic :: Software Development :: Version Control :: Git'
    ],
    description="git log viewer with foling & unfolding merges support",
    keywords="git GitPython tig tui lazygit",
    data_files=[
        ('man/man1', ['docs/glv.1'])
    ],
    long_description=
    '''An alternative to `tig(1)`/`lazygit(1) which supports folding merges and is expandable via plugins. The application can resolve the default merge titles done by using GitHub or Bitbucket to the actual pull request names.''',
    long_description_content_type='text/markdown',
    install_requires=REQUIREMENTS,
    test_require=TEST_REQUIREMENTS,
    entry_points={
        'console_scripts': [
            'glv=glv.main:cli',
        ],
        'glv_providers': [
            'atlassian=glv.providers:Atlassian',
            'github=glv.providers:GitHub',
        ],
        'glv_icons': [
            'ascii=glv.icons:ASCII',
            'nerdfont=glv.icons:NERDFONT',
        ],
    },
    packages=find_packages(),
    version="2.0.0")
