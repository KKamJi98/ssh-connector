
from setuptools import setup, find_packages

setup(
    name="ssh-connector",
    version="0.1.0",
    packages=find_packages(),
    include_package_data=True,
    install_requires=[
        "click",
    ],
    extras_require={
        "dev": [
            "uv",
            "black",
            "isort",
            "pytest",
        ]
    },
    entry_points={
        "console_scripts": [
            "ssh-connector=ssh_connector:cli",
        ],
    },
    author="TaeJi-Kim",
    author_email="rlaxowl5460@gmail.com",
    description="A CLI tool to easily select and connect to SSH hosts from your config file.",
    long_description=open("README.md").read(),
    long_description_content_type="text/markdown",
    url="https://github.com/TaeJi-Kim/ssh-connector",
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
    ],
    python_requires=">=3.13.0",
)
