# Changelog

## [0.3.5](https://github.com/swiftaspect/sync-branch-deps/compare/v0.3.4...v0.3.5) (2026-08-29)


### Bug Fixes

* **deps:** Bump serde_json from 1.0.150 to 1.0.151 ([830da88](https://github.com/swiftaspect/sync-branch-deps/commit/830da885a4f474b0044f4c29a4905bba666c5509))

## [0.3.4](https://github.com/swiftaspect/sync-branch-deps/compare/v0.3.3...v0.3.4) (2026-08-29)


### Bug Fixes

* **deps:** Bump regex from 1.12.4 to 1.13.1 ([64cd17a](https://github.com/swiftaspect/sync-branch-deps/commit/64cd17a2072fba80d64302fd0f6f97668eb910ef))

## [0.3.3](https://github.com/swiftaspect/sync-branch-deps/compare/v0.3.2...v0.3.3) (2026-08-29)


### Bug Fixes

* **deps:** Bump anyhow from 1.0.103 to 1.0.104 ([4e1b59a](https://github.com/swiftaspect/sync-branch-deps/commit/4e1b59a958f98cf47c122f41ae6f456ef8186a9d))

## [0.3.2](https://github.com/swiftaspect/sync-branch-deps/compare/v0.3.1...v0.3.2) (2026-08-29)


### Bug Fixes

* **deps:** Bump library/rust from `9a2cd30` to `271849e` ([cd971d2](https://github.com/swiftaspect/sync-branch-deps/commit/cd971d227b804fee931e3a8254cb813f1d0b70d0))

## [0.3.1](https://github.com/swiftaspect/sync-branch-deps/compare/v0.3.0...v0.3.1) (2026-08-29)


### Bug Fixes

* **deps:** Bump distroless/static-debian12 from `aef9602` to `afa5c87` ([fa3d78c](https://github.com/swiftaspect/sync-branch-deps/commit/fa3d78cd1f33e5a1c6e3ec834111ee41ad28fac7))

## [0.3.0](https://github.com/swiftaspect/sync-branch-deps/compare/v0.2.0...v0.3.0) (2026-07-04)


### Features

* **git:** detect the branch from .git/HEAD when no git binary is present ([fa96b78](https://github.com/swiftaspect/sync-branch-deps/commit/fa96b789628ff8c9512f296f174ec8ad780922a9))

## [0.2.0](https://github.com/swiftaspect/sync-branch-deps/compare/v0.1.1...v0.2.0) (2026-07-04)


### Features

* **oci:** authenticate private registry lookups from standard credential sources ([75146f4](https://github.com/swiftaspect/sync-branch-deps/commit/75146f48d72af962d73d0b405a8c070ee7f3350d))

## [0.1.1](https://github.com/swiftaspect/sync-branch-deps/compare/v0.1.0...v0.1.1) (2026-07-03)


### Bug Fixes

* **build:** build dist binary for BINARY_TARGET instead of mislabeling the native one ([5255f84](https://github.com/swiftaspect/sync-branch-deps/commit/5255f84c74e82daccca91341f4988744898cd114))

## 0.1.0 (2026-07-03)


### Features

* add sync/verify subcommands with located output ([fe34da5](https://github.com/swiftaspect/sync-branch-deps/commit/fe34da579f6ab1c4f3a18087f5ef652d689813af))
* initial sync-branch-deps implementation ([38a8436](https://github.com/swiftaspect/sync-branch-deps/commit/38a8436be31158dd83c16d7fb09ac2ad0f8c8444))


### Bug Fixes

* **compose:** match quoted images and anchor image: key in pin and verify ([97df47c](https://github.com/swiftaspect/sync-branch-deps/commit/97df47c9b1bef746b456913627097156d9dd4c5e))
* **config:** tolerate and warn on non-list values instead of hard-failing ([db06d56](https://github.com/swiftaspect/sync-branch-deps/commit/db06d569389903b43048e3c33577aebf839dff7e))
* **config:** treat a bare null/empty document as an empty config ([f5923df](https://github.com/swiftaspect/sync-branch-deps/commit/f5923df83a330f84f79a765e73937aa168578bbd))
* **package-json:** stop verify mis-classifying protocol deps and numeric pre-releases ([0e2e00d](https://github.com/swiftaspect/sync-branch-deps/commit/0e2e00d6fe5a2a0aef31d465b66e606c050ecd99))
