# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.0](https://github.com/mbhall88/ssubmit/compare/1.2.0...1.3.0) (2026-08-06)


### Features

* add installable ssubmit agent skill ([#16](https://github.com/mbhall88/ssubmit/issues/16)) ([#23](https://github.com/mbhall88/ssubmit/issues/23)) ([de95bf5](https://github.com/mbhall88/ssubmit/commit/de95bf5188904beb1d7d1100de82a41654e6ad08))
* add JSON scheduler test results ([#15](https://github.com/mbhall88/ssubmit/issues/15)) ([#22](https://github.com/mbhall88/ssubmit/issues/22)) ([cd3a835](https://github.com/mbhall88/ssubmit/commit/cd3a835cda302f07c17d0bda3fdb533555b381e4))
* add versioned JSON job planning ([#13](https://github.com/mbhall88/ssubmit/issues/13)) ([#20](https://github.com/mbhall88/ssubmit/issues/20)) ([24e3462](https://github.com/mbhall88/ssubmit/commit/24e34622dc139ab24caeb6920ef17e55a17bebe5))
* migrate release and installer automation to cargo-dist ([#18](https://github.com/mbhall88/ssubmit/issues/18)) ([#24](https://github.com/mbhall88/ssubmit/issues/24)) ([f90b2e8](https://github.com/mbhall88/ssubmit/commit/f90b2e8258097829953a7b6eda545dd0e47c872d))
* return structured JSON submission results ([#14](https://github.com/mbhall88/ssubmit/issues/14)) ([#21](https://github.com/mbhall88/ssubmit/issues/21)) ([0c0d3ed](https://github.com/mbhall88/ssubmit/commit/0c0d3edb90dc3bf5ec3406149caa88442c3fb183))


### Bug Fixes

* establish reliable batch CLI contract ([#12](https://github.com/mbhall88/ssubmit/issues/12)) ([484f055](https://github.com/mbhall88/ssubmit/commit/484f055ad3a1d2940da46d0701525e420f586fde))
* replace dead installer URL ([b558c4b](https://github.com/mbhall88/ssubmit/commit/b558c4b2c6aab37e21166a453722d2b5e9d54cf5))

## [1.2.0](https://github.com/mbhall88/ssubmit/compare/1.1.0...1.2.0) (2025-08-07)


### Features

* add --export option to control environment variable export ([509f312](https://github.com/mbhall88/ssubmit/commit/509f312926c9ad36498fbac64f5798f31f52e038))

## [1.1.0](https://github.com/mbhall88/ssubmit/compare/1.0.0...1.1.0) (2025-08-06)


### Features

* add interactive job support ([#8](https://github.com/mbhall88/ssubmit/issues/8)) ([5062291](https://github.com/mbhall88/ssubmit/commit/50622912a9ced1ab5c8a2597996c8cd6e31ff034))

## [1.0.0](https://github.com/mbhall88/ssubmit/compare/0.3.0...1.0.0) (2024-08-21)


### ⚠ BREAKING CHANGES

* improve memory parsing

### Features

* env vars for setting time, memory, shebang and set [[#5](https://github.com/mbhall88/ssubmit/issues/5)] ([9e87374](https://github.com/mbhall88/ssubmit/commit/9e87374c712085256fa5f220ee3a95f65bab92a4))
* improve memory parsing ([74d2e0f](https://github.com/mbhall88/ssubmit/commit/74d2e0f77094c632c9f7afacfc98bc467f988238))

## [0.3.0](https://github.com/mbhall88/ssubmit/compare/0.2.0...0.3.0) (2024-01-30)


### Features

* add test-only option to estimate start time ([4ee01c4](https://github.com/mbhall88/ssubmit/commit/4ee01c41b00605b4362f407701073fa6b8b4e32f))

## [Unreleased]

### Changed

- Small linting fixes and modernise CI

## [0.2.0]

### Changed

- Changed default `--set` from `-eux` to `-euxo pipefail`.

## [0.1.1]

### Added

- changelog

## [0.1.0]

### Added

- Everthing!

[0.1.0]: https://github.com/mbhall88/ssubmit/releases/tag/0.1.0
[0.1.1]: https://github.com/mbhall88/ssubmit/releases/tag/0.1.1
[0.2.0]: https://github.com/mbhall88/ssubmit/compare/0.1.1...0.2.0
[unreleased]: https://github.com/mbhall88/ssubmit/compare/0.2.0...HEAD
