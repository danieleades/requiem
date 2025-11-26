# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/danieleades/requiem/releases/tag/requirements-manager-core-v0.1.1) - 2025-11-26

### Added

- implement comprehensive cycle detection and enhanced validation
- implement 'add' and 'review' MCP endpoints ([#96](https://github.com/danieleades/requiem/pull/96))
- add 'kind' metadata ([#72](https://github.com/danieleades/requiem/pull/72))
- add model context protocol server ([#71](https://github.com/danieleades/requiem/pull/71))

### Fixed

- *(core)* prevent panic on missing parent HRID ([#95](https://github.com/danieleades/requiem/pull/95))

### Other

- *(cli)* improve validate command implementation and code quality
- add comprehensive cycle detection tests
