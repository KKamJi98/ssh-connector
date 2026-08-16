# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1](https://github.com/KKamJi98/ssh-connector/compare/v0.3.0...v0.3.1) (2026-08-16)


### Bug Fixes

* declare the license and stop requiring Python 3.13 ([c836f12](https://github.com/KKamJi98/ssh-connector/commit/c836f12585a03e01b4e6c2e7a47dd08a5d3c21d8))

## [0.3.0](https://github.com/KKamJi98/ssh-connector/compare/v0.2.0...v0.3.0) (2026-08-16)


### Features

* **ci:** create the GitHub release alongside the PyPI upload ([34426ee](https://github.com/KKamJi98/ssh-connector/commit/34426ee6a0d6a0518cc5184ae9ed7f45c210e632))


### Bug Fixes

* **ci:** drive the release from release-please, not a tag trigger ([13ce885](https://github.com/KKamJi98/ssh-connector/commit/13ce8854b0b2b166f97e8e94b933bba500c96964))

## [0.2.0] - 2026-08-16

### Added
- Select a host by name as well as by number.
- Show all jump hosts at the bottom under their own group column.
- Ignore hosts whose names carry an abort suffix, and widen the ignore list.
- Publish to PyPI on a version tag, through Trusted Publishing.

### Changed
- Rename the `Default` group label to `Main`.
- Document how `Include` directives are handled.

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
