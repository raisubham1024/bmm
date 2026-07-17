# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Alt+m` opens a mode switcher overlay, from any view, listing every mode
  (all bookmarks, search, tags, duplicates, starred, global search,
  databases, help). Navigate with `j`/`k`/arrows, `Enter` jumps to the
  selected mode, `Alt+m`/`Esc`/`q` closes it without switching. Selecting a
  mode reuses that mode's existing shortcut logic exactly, so behavior is
  identical to typing the shortcut key directly.

### Fixed

- Android/Termux: `i`/`I` (open in incognito) now actually opens a private tab
  instead of a normal one; since Chrome doesn't let third-party apps load a
  url directly into an incognito tab, the url(s) are copied to the clipboard
  instead, with a message telling you to paste
- Android/Termux: `Y` (copy all listed links) now works on terminals/keyboards
  that report Shift+Y as lowercase `y` with a shift modifier instead of an
  uppercase `Y`

## [v0.3.1] - May 16, 2026

### Changed

- Dependency and toolchain updates

## [v0.3.0] - Mar 10, 2025

### Added

- Allow ignoring attribute errors while saving/importing bookmarks (longer
    titles will be trimmed, some invalid tags will be corrected)
- Allow copying URI(s) to the system clipboard via the TUI

### Fixed

- Listing bookmarks by tag(s) shows all tags for each bookmark returned

## [v0.2.0] - Feb 27, 2025

### Changed

- Allow searching over multiple terms
- Respect XDG_DATA_HOME on MacOS as well, if set

## [v0.1.0] - Feb 20, 2025

### Added

- Initial release

[unreleased]: https://github.com/dhth/bmm/compare/v0.3.1...HEAD
[v0.3.1]: https://github.com/dhth/bmm/compare/v0.3.0...v0.3.1
[v0.3.0]: https://github.com/dhth/bmm/compare/v0.2.0...v0.3.0
[v0.2.0]: https://github.com/dhth/bmm/compare/v0.1.0...v0.2.0
[v0.1.0]: https://github.com/dhth/bmm/commits/v0.1.0/
