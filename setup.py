""" git-log-viewer package specification """

import setuptools

with open("README.md", "r") as fh:
    LONG_DESCRIPTION = fh.read()

setuptools.setup(
    name="git-log-viewer",
    version="1.3.0",
    python_requires='>=3.6',
    author="Bahtiar `kalkin` Gadimov",
    author_email="bahtiar@gadimov.de",
    description="A git log viewer with folding merges support",
    url="https://github.com/kalkin/git-log-viewer",
    long_description=LONG_DESCRIPTION,
    long_description_content_type="text/markdown",
    packages=setuptools.find_packages(),
    data_files=[('man/man1', ['docs/glv.1'])],
    entry_points={
        'console_scripts': ['glv=glv.main:cli'],
        'glv_providers': [
            'atlassian=glv.providers:Atlassian',
            'github=glv.providers:GitHub',
        ],
        'glv_icons': [
            'ascii=glv.icon:ASCII',
            'nerdfont=glv.icon:NERDFONT',
        ],
    },
    install_requires=[
        'pygit2 >= 0.28.0', 'prompt_toolkit >= 2.0, <4.0',
        'Babel >= 2.5.1, <3.0', 'certifi', 'urllib3', 'docopt',
        'pykka >= 2.0.0', 'pygments >= 2.6.0', 'xdg >= 4.0.0'
    ],
    tests_require=['aloe >= 0.1.19, <= 0.2.0'],
    classifiers=[
        "Programming Language :: Python :: 3",
        # pylint: disable=line-too-long
        "License :: OSI Approved :: GNU Affero General Public License v3 or later (AGPLv3+):",  # noqa: E501
        "Operating System :: POSIX",
        "Topic :: Software Development :: Version Control :: Git",
    ],
)
