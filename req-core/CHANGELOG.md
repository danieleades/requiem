# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/danieleades/requiem/releases/tag/requirements-manager-core-v0.1.1) - 2026-07-13

### Added

- improve Sphinx/mdBook integration ([#152](https://github.com/danieleades/requiem/pull/152))
- *(mcp)* add update, link/unlink, and delete requirement tools ([#151](https://github.com/danieleades/requiem/pull/151))
- [**breaking**] use xxHash3 (128-bit) for requirement fingerprints ([#147](https://github.com/danieleades/requiem/pull/147))
- implement lowercase namespace support (CORE-DFT-014) ([#102](https://github.com/danieleades/requiem/pull/102))
- implement 'add' and 'review' MCP endpoints ([#96](https://github.com/danieleades/requiem/pull/96))
- add 'kind' metadata ([#72](https://github.com/danieleades/requiem/pull/72))
- add model context protocol server ([#71](https://github.com/danieleades/requiem/pull/71))

### Fixed

- harden storage durability and correct CLI/domain defects ([#146](https://github.com/danieleades/requiem/pull/146))
- *(core)* prevent panic on missing parent HRID ([#95](https://github.com/danieleades/requiem/pull/95))

### Other

- split god modules into focused submodules ([#148](https://github.com/danieleades/requiem/pull/148))
- *(deps)* bump criterion from 0.7.0 to 0.8.2
- *(deps)* bump borsh from 1.5.7 to 1.6.0 ([#97](https://github.com/danieleades/requiem/pull/97))
- *(cli)* add cycle detection ([#100](https://github.com/danieleades/requiem/pull/100))
