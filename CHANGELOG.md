# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/danieleades/requiem/compare/v0.1.1...v0.2.0) - 2025-11-22

### Added

- deduplicate the 'sync' and 'diagnose' commands ([#70](https://github.com/danieleades/requiem/pull/70))
- [**breaking**] rework the CLI ([#67](https://github.com/danieleades/requiem/pull/67))
- improve config command help and add get subcommand (Phase 4)
- add rename and move commands (Phase 3)
- enhance init command with --kinds flag (Phase 3)
- add kind management commands (Phase 3)
- rename add → create command (Phase 3)
- add show command with multiple output formats (Phase 3)
- add unified validate command (Phase 2)
- implement Phase 1 CLI redesign with clean replacements

### Fixed

- resolve all clippy pedantic warnings
- general bug fixing ([#63](https://github.com/danieleades/requiem/pull/63))
- *(cli)* update documentation for current CLI commands ([#62](https://github.com/danieleades/requiem/pull/62))

### Other

- partial clean-up of the CLI code ([#69](https://github.com/danieleades/requiem/pull/69))
- partial clean-up of the CLI code ([#68](https://github.com/danieleades/requiem/pull/68))
- *(deps)* bump actions/checkout from 5 to 6 ([#66](https://github.com/danieleades/requiem/pull/66))
- extract helper methods to eliminate too_many_lines warnings
- update codecov badge link in README ([#65](https://github.com/danieleades/requiem/pull/65))
- check in CI that the requirements are in a valid state ([#64](https://github.com/danieleades/requiem/pull/64))
- Update Pages workflow to deploy on release tags ([#60](https://github.com/danieleades/requiem/pull/60))

## [0.1.1](https://github.com/danieleades/requiem/compare/v0.1.0...v0.1.1) - 2025-11-14

### Added

- embed HRIDs directly in document title instead of frontmatter ([#59](https://github.com/danieleades/requiem/pull/59))
- specify and improve CLI ergonomics ([#48](https://github.com/danieleades/requiem/pull/48))
- add 'list' command ([#45](https://github.com/danieleades/requiem/pull/45))
- allow users to specify directory layout ([#42](https://github.com/danieleades/requiem/pull/42))
- improve error handling ([#25](https://github.com/danieleades/requiem/pull/25))
- support namespaces in HRIDs ([#21](https://github.com/danieleades/requiem/pull/21))

### Other

- *(deps)* bump clap from 4.5.50 to 4.5.51 in the patch-updates group ([#52](https://github.com/danieleades/requiem/pull/52))
- *(deps)* bump petgraph from 0.6.5 to 0.8.3 ([#56](https://github.com/danieleades/requiem/pull/56))
- *(deps)* bump indicatif from 0.17.11 to 0.18.3 ([#58](https://github.com/danieleades/requiem/pull/58))
- *(deps)* bump dialoguer from 0.11.0 to 0.12.0 ([#53](https://github.com/danieleades/requiem/pull/53))
- code quality improvements ([#55](https://github.com/danieleades/requiem/pull/55))
- *(deps)* bump actions/checkout from 4 to 5 ([#29](https://github.com/danieleades/requiem/pull/29))
- *(deps)* bump amannn/action-semantic-pull-request from 5 to 6 ([#30](https://github.com/danieleades/requiem/pull/30))
- *(deps)* bump actions/upload-pages-artifact from 3 to 4 ([#47](https://github.com/danieleades/requiem/pull/47))
- *(deps)* bump astral-sh/setup-uv from 6 to 7 ([#39](https://github.com/danieleades/requiem/pull/39))
- update docs CI job
- Add a project 'book' ([#40](https://github.com/danieleades/requiem/pull/40))
- *(deps)* bump criterion from 0.6.0 to 0.7.0 ([#22](https://github.com/danieleades/requiem/pull/22))
- add sphinx and mdbook examples ([#23](https://github.com/danieleades/requiem/pull/23))
- extend and improve benchmarks ([#20](https://github.com/danieleades/requiem/pull/20))
- release v0.1.0 ([#11](https://github.com/danieleades/requiem/pull/11))
