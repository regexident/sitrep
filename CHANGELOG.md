# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Please make sure to add your changes to the appropriate categories:

- `Added`: for new functionality
- `Changed`: for changes in existing functionality
- `Deprecated`: for soon-to-be removed functionality
- `Removed`: for removed functionality
- `Fixed`: for fixed bugs
- `Performance`: for performance-relevant changes
- `Security`: for security-relevant changes
- `Other`: for everything else

## [Unreleased]

### Added

- n/a

### Changed

- Updated dependencies:
  - `parking_lot` from `0.12.1` to `0.12.2`
  - `criterion` from `0.5` to `0.5.1`

### Deprecated

- n/a

### Removed

- n/a

### Fixed

- n/a

### Performance

- n/a

### Security

- n/a

### Other

- n/a

## [0.3.0] - 2024-02-25

### Added

- Added enum `State` and corresponding field `state` to `Task` and `Report`.
- Added `NopObserver` implementation for ignoring events.
- Added `as_raw()` getter method to `ProgressId`.
- Added `label()`, `completed()`, `total()` & `state()` getter methods to `Progress`.
- Added `child()` accessor method to `Progress`.
- Added trait `Controller`.
- Added `is_cancelable()` & `is_pausable()` to `Controller`.
- Added `set_cancelable()` & `set_pausable()` to `Progress`.
- Made methods `pause()`, `resume()` & `cancel()` of `Progress` check if task is cancelable/pausable.
- Added `is_canceled()` & `is_paused()` getters to `Controller`.
- Added `partial_report()` method to `Reporter` trait.

### Changed

- Renamed `Event::Removed` to `Event::Detachment`.
- Renamed `RemovalEvent` to `DetachmentEvent`.
- Made `Progress` emit `Event::Removed` only when being detached from a parent.
- Changed signature of method `report()` of `Reporter`, changing `&self` to `self: &Arc<Self>`.
- Made `Progress` impl `Controller`.
- Moved `get()` method from `Progress` into `Controller`.
- Relaxed memory ordering from `Ordering::SeqCst` to `Ordering::Relaxed`.
- Moved `last_change` from `Task` into `atomic_state` field of `Progress`.
- Simplified logic behind `last_change` generations, removing need for shared `last_tree_change`.
- Refactored `report()` method of `Progress`.

### Fixed

- Fixed bug in `last_change` bumping logic.

## [0.2.0] - 2024-02-19

### Added

- Made `StdMpscObserver` impl `Send` + `Sync`.
- Added `get()` method to `Progress` for accessing a specific (sub)progress (within the progress subtree) by its identifier.
- Made `ProgressId` impl `Default`.
- Made `Report` impl `Default`.
- Added `to_pruned()`/`into_pruned()` methods to `Report` for obtaining a pruned report containing only those sub-reports that were updated since a certain generation and thus need updating of their corresponding user-facing UI.
- Added `as_raw()` method to `Generation` for accessing the raw internal generational counter value.
- Added `last_change()` getter method to `Report` for accessing its last change's generation.
- Added `parent()` getter to `Progress`.
- Added `children()` getter to `Progress`.
- Added method `detach_from_parent()` to `Progress` as a counter-part for `detach_child()`.
- Add `child()` accessor method to `Progress`.

### Changed

- Renamed field `generation` of `Task` to `last_change`.
- Renamed field `generation` of `Report` to `last_change`.
- Rename field `max_generation` of `Progress` to `last_tree_change`.
- Changed type of field `label` of `Task` from `Option<String>` to `Option<Cow<'static, str>>`.
- Changed type of field `label` of `Report` from `Option<String>` to `Option<Cow<'static, str>>`.
- Changed type of field `message` of `MessageEvent` from `String` to `Cow<'static, str>`.

### Performance

- Improved performance of `Reporter.report()` for `&'static str` task labels (instead of `String`).
- Improved performance of `Progress.message()` for `&'static str` messages (instead of `String`).

## [0.1.0] - 2024-02-15

Initial release.
