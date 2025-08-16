# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0](https://github.com/KKamJi98/ssh-connector/compare/v0.1.1...v0.2.0) (2025-08-16)


### Features

* group hosts by source file in CLI display ([0649601](https://github.com/KKamJi98/ssh-connector/commit/06496011d5716ac39731519e87346d64d5816cd6))
* show all jump hosts at bottom with group column ([69b32f6](https://github.com/KKamJi98/ssh-connector/commit/69b32f64133403b805b488733b6cfb06625bcccc))


### Bug Fixes

* remove unused variables and clean exception handling ([ca9ed43](https://github.com/KKamJi98/ssh-connector/commit/ca9ed4326f8efae89df294f93e8486e99af9a9b2))


### Documentation

* Move program description to top of README.md ([56dbe9d](https://github.com/KKamJi98/ssh-connector/commit/56dbe9da3eb3c52e8312971be4937f0fde1c7fb3))
* update README for uv and ruff with CI badge ([fc038b9](https://github.com/KKamJi98/ssh-connector/commit/fc038b9efafae432ea9efad8d93dd4e94d5cc589))

## [0.1.1] - 2025-06-29

### Changed
- Updated README.md with enhanced SSH host examples for dev, stg, and prod environments, including jump hosts.

## [0.1.0] - 2025-06-28

### Added
- Initial project setup.
- CLI to list and connect to SSH hosts.
- Display SSH hosts in order of appearance from config file.
- Improve SSH host list formatting for consistent alignment.
- Implement interactive filtering and rich table display for SSH hosts.
- Display 'jump' hosts at the bottom of the list with a 'JUMP-HOSTS' separator.
