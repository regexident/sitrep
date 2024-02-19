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

- Added enum `State` and corresponding field `state` to `Task` and `Report`.
- Added `NopObserver` implementation for ignoring events.
- Added `as_raw()` getter method to `ProgressId`.
- Added `label()`, `completed()`, `total()` & `state()` getter methods to `Progress`.
- Added `child()` accessor method to `Progress`.

### Changed

- Renamed `Event::Removed` to `Event::Detachment`.

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
