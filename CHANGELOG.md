# Changelog

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
