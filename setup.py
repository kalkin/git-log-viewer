""" pygit-viewer package specification """

import setuptools

with open("README.md", "r") as fh:
    LONG_DESCRIPTION = fh.read()

setuptools.setup(
    name="pygit-viewer",
    version="0.0.1",
    python_requires='>=3.6',
    author="Bahtiar `kalkin` Gadimov",
    author_email="bahtiar@gadimov.de",
    description="A git log viewer with folding support",
    url="git@github.com:monorepo.git:/pygit-viewer",
    long_description=LONG_DESCRIPTION,
    long_description_content_type="text/markdown",
    packages=setuptools.find_packages(),
    entry_points='''
        [console_scripts]
        pygit-viewer=pygit_viewer.main:cli
    ''',
    install_requires=[
        'pygit2', 'prompt_toolkit >= 2.0, <=3.0', 'Babel >= 2.5.1, <=3.0'
    ],
    classifiers=[
        "Programming Language :: Python :: 3",
        # pylint: disable=line-too-long
        "License :: OSI Approved :: GNU Affero General Public License v3 or later (AGPLv3+):",  # noqa: E501
        "Operating System :: POSIX",
        "Topic :: Software Development :: Version Control :: Git",
    ],
)
