<div align="center">

# bmm

**A fast, local-first bookmark manager for your terminal.**

Save links, find them with a few words, organize them with tags and notes, and open them in your browser — from a keyboard-driven TUI or a scriptable CLI.

[Features](#features) · [Install](#installation) · [Quick start](#quick-start) · [TUI guide](#terminal-user-interface-tui) · [CLI reference](#command-line-reference) · [Contributing](#contributing)

</div>

---

## 🧙 Why use bmm?

Browser bookmarks can become difficult to search and organize, and they stay tied to a particular browser. **bmm** keeps your links in a local SQLite database instead. Browse them in an interactive terminal interface, or use ordinary commands to save, search, export, and manage bookmarks from scripts.

Your bookmark database stays on your device. bmm does not require an account or a cloud service. Network access is only needed for actions such as fetching a website description, checking links, or downloading an update.

## Features

- **Two ways to work:** an interactive terminal user interface (TUI) and a command-line interface (CLI) for scripts and shell workflows.
- **Fast, flexible search:** search URLs, titles, and tags with multiple terms; choose a specific search scope when needed.
- **Simple link capture:** save one bookmark or many. A missing URL scheme is automatically changed to `https://`.
- **Tags and tag suggestions:** add multiple tags, browse tags and their bookmark counts, rename or delete tags, and use hierarchical tags such as `learning/rust`.
- **Notes and favorites:** attach searchable private notes to bookmarks and star important links.
- **Multiple databases:** keep work, study, and personal bookmarks separate; switch databases, move bookmarks, and group databases into folders.
- **Search across databases:** find links or tags across your local databases without switching between them.
- **Link checking:** check saved web links for errors, with configurable timeouts and concurrency.
- **Import and export-friendly output:** import browser-export HTML, JSON, or plain-text URL files; list results as plain text, JSON, or delimited output.
- **Browser and clipboard actions:** open one or more links, use a private window where supported, and copy one or all results.
- **Description fetching:** fetch a web page's description into a bookmark title.
- **Backup and restore:** copy database files to and from a folder outside bmm's data directory.
- **Local-first storage:** bookmarks are stored in SQLite on your device; no bmm account or hosted service is needed.
- **Android support:** use bmm in Termux, with Termux:API for clipboard and browser integration.

## Installation

### Download a release binary

Visit the [GitHub Releases](https://github.com/raisubham1024/bmm/releases) page and download the binary for your platform. The latest release currently provides binaries for **Linux x86_64** and **Android aarch64**. Release assets and supported platforms may change, so check the release page before downloading.

After downloading a binary on Linux, you can rename it to `bmm`, make it executable, and place it in a directory on your `PATH`:

```bash
chmod +x bmm-linux-x86_64
mkdir -p "$HOME/.local/bin"
mv bmm-linux-x86_64 "$HOME/.local/bin/bmm"
```

Make sure `$HOME/.local/bin` is on your `PATH`, then verify the installation:

```bash
bmm --version
```

### Install from source with Cargo

Install Rust (the repository pins Rust **1.96**) and Git, then run:

```bash
cargo install --git https://github.com/raisubham1024/bmm.git
```

Verify that Cargo's binary directory is on your `PATH`:

```bash
bmm --version
```

To build a local checkout instead:

```bash
git clone https://github.com/raisubham1024/bmm.git
cd bmm
cargo build --release
./target/release/bmm --help
```

### Android / Termux

1. Install [Termux](https://github.com/termux/termux-app) from its official GitHub releases or F-Droid. Avoid mixing Termux apps installed from different sources.
2. Open Termux and update its packages:

   ```bash
   pkg update && pkg upgrade
   ```

3. Install Termux:API from the same app source as Termux, then install its command-line package:

   ```bash
   pkg install termux-api
   ```

4. Download the Android **aarch64** binary from [bmm Releases](https://github.com/raisubham1024/bmm/releases), or build from source using the Termux Rust package.
5. Put the executable somewhere on your `PATH`, make it executable, and run `bmm`.

Termux:API is needed for clipboard and browser actions. Without it, those actions cannot work. See [TERMUX.md](TERMUX.md) for additional Termux build and troubleshooting notes.

> **Note:** Prebuilt binaries are only published for the platforms listed on the Releases page. Building from source may work on other Rust-supported platforms, but those platforms are not currently covered by the self-updater.

## Quick start

Running `bmm` with no subcommand opens the TUI. You can also start it explicitly:

```bash
bmm tui
```

Save a bookmark from the CLI:

```bash
bmm save https://www.rust-lang.org \
  --title "The Rust programming language" \
  --tags programming,rust
```

Search across the bookmark's URL, title, and tags:

```bash
bmm search rust programming
```

Show all saved bookmarks, or list only those with a matching tag:

```bash
bmm list
bmm list --tags rust
```

The first command to try in the TUI is `?` for the built-in help. Press `Alt+m` to open the mode switcher.

## Terminal user interface (TUI)

The TUI is keyboard-driven. Use `j`/`Down` and `k`/`Up` to move through lists. Press `?` in the TUI for the full, up-to-date help tables; shortcuts can depend on the active view.

### Main bookmark list

| Key | Action |
| --- | --- |
| `s` | Open search. Results update as you type. |
| `Enter` in an empty search | Show all bookmarks in the current database. |
| `l` | Show all bookmarks and clear active filters. |
| `a` | Add a bookmark. |
| `e` | Edit the selected bookmark's title and tags. |
| `E` | Edit the selected bookmark's URL, title, and tags. |
| `n` / `N` | Add or edit a note / delete a note. |
| `*` | Star or unstar the selected bookmark. |
| `S` | Show starred bookmarks. |
| `t` or `Tab` | Browse tags in the current database (when not searching). |
| `T` | Browse tags across databases. |
| `z` | Search across databases. |
| `Alt+n` | Search bookmarks by note text. |
| `Alt+B` | Check the links currently shown; broken links are listed with a reason. |
| `o` / `i` | Open the selected link normally / in a private window, where supported. |
| `O` / `I` | Open the listed links normally / privately (up to 30 links). |
| `y` / `Y` | Copy the selected URL / all listed URLs to the clipboard. |
| `d` or `Delete` | Delete the selected bookmark after confirmation. |
| `D` | Delete all bookmarks currently listed after confirmation. |
| `Space`, `m`, `M` | Mark bookmarks and move one or more to another database. |
| `A` | Open the database list. |
| `Alt+b` / `Alt+g` | Back up / restore database files. |
| `Alt+u` | Check for and install an update on supported platforms. |
| `Alt+m` | Open the mode switcher. |
| `Esc` / `q` | Go back / exit when there is no previous view. |

**Search scopes:** While searching, press `Alt+s` to search all fields, URLs only, titles only, or tags only. Every search term must match somewhere in the selected scope; terms may match different fields when searching all fields. For example, `rust guide` can find a bookmark with `rust` in its tags and `guide` in its title.

### Add and edit bookmarks

In the add/edit view, use `Tab` or `Down` to move to the next field and `Shift+Tab` or `Up` to move back. Enter a URL, an optional title, and comma-separated tags. Press `Ctrl+s` to save; bmm asks for confirmation. In the title field, `Alt+F` fetches the page description when available.

The two edit shortcuts are intentionally different: `e` changes only the title and tags, while `E` also makes the URL editable.

### Tags, suggestions, and folders

- In the tag list, `/` opens tag search and `Enter` shows bookmarks with the selected tag.
- bmm suggests existing tags while you type. In an add/edit tag field, `Alt+z` shows all saved tags; use `Up`/`Down` and `Enter` to choose suggestions.
- Tags support up to four levels, separated by `/`, for example `learning/rust/cli`. Each level can contain up to 30 letters or digits from any language, emoji, `_`, or `-`. Tags are stored in lowercase.
- In the database list, `F` opens database folders. Folders organize database names; they do not move the database files. Use `C` to create a folder, `R` to rename, and `D` to delete a folder (the databases inside it are kept). Inside a folder, use `Space` to mark databases and the move action shown in `?` to organize them.

### Databases

Press `A` to open the database list. Select a database and press `Enter` to use it for the current session. A new bmm session starts with the default `bmm.db` database. Use `C` to create a database and `R` to rename one; the default database cannot be renamed. Use `m` to move the selected bookmark, or mark bookmarks with `Space` and use `M` to move them together.

Databases can also be selected for individual CLI commands with `--db`:

```bash
bmm list --db work
bmm save https://example.com --title "Example" --db work
```

## Command-line reference

Run `bmm --help` or `bmm <command> --help` for the full help text for your installed version.

### Global options

These options can be used with commands that operate on a database:

| Option | Description |
| --- | --- |
| `--db <NAME>` | Use a database by name, such as `work` or `work.db`. |
| `--db-path <PATH>` | Use a database at a specific file path. Cannot be combined with `--db`. |
| `--debug` | Print parsed arguments and the selected database path. |
| `-v`, `-V`, `--version` | Print the bmm version. |

### Bookmark commands

| Command | What it does |
| --- | --- |
| `bmm` or `bmm tui` | Open the TUI. |
| `bmm save <URL>` | Add or update a bookmark. Options include `--title`, `--tags`, `--ptag`, `--editor`, `--star`, `--fail-if-already-saved`, `--reset-missing-details`, and `--ignore-attribute-errors`. |
| `bmm save-all [URL ...]` | Save multiple URLs. Use `--stdin` to read URLs, one per line, from standard input; `--tags` applies tags to all of them. |
| `bmm list` | List bookmarks. Filter with `--uri`, `--title`, and/or `--tags`; choose `--format plain`, `json`, or `delimited`; set `--limit`. |
| `bmm search <TERM ...>` | Search URL, title, and tags with all supplied terms. Use `--tui` for interactive results; `--format` and `--limit` control CLI output. |
| `bmm show <URL>` | Show details for the exact saved URL. |
| `bmm delete <URL ...>` | Delete one or more exact URLs. Use `-y` or `--yes` to skip confirmation. |
| `bmm import <FILE>` | Import an `.html`, `.json`, or `.txt` bookmark file. Options: `--dry-run`, `--reset-missing-details`, and `--ignore-attribute-errors`. |
| `bmm check` | Check saved links. Filter with `--uri` and/or `--tags`; tune `--limit`, `--concurrency`, and `--timeout`; use `--show-all` to include working links. |
| `bmm fetch <URL>` | Fetch and print a page's description. Use `--timeout` to change the timeout in seconds. |
| `bmm notes <URL>` | Open the configured editor to add or edit a note. Use `--print` to print the saved note instead. |

Examples:

```bash
# Save a bookmark and mark it as a favorite
bmm save https://example.com --title "Example" --tags reading,reference --star

# Save URLs from a command or file, one per line
printf '%s\n' https://example.com https://www.rust-lang.org \
  | bmm save-all --stdin --tags reading

# Search, with JSON output and a result limit
bmm search rust systems --format json --limit 20

# Check links tagged "reading", including working links in the output
bmm check --tags reading --show-all

# Preview an import without changing the database
bmm import bookmarks.html --dry-run

# Print a bookmark note without opening an editor
bmm notes https://example.com --print
```

### Tag commands

| Command | What it does |
| --- | --- |
| `bmm tags list` | List tags. Add `--show-stats` for bookmark counts; `--format` accepts `plain`, `json`, or `delimited`; `--tui` opens the tag view. |
| `bmm tags rename <SOURCE> <TARGET>` | Rename a tag in the selected database. |
| `bmm tags delete <TAG ...>` | Delete one or more tags after confirmation. Use `--yes` to skip confirmation. |

For nested tags, use `--ptag` to prefix tags while saving. For example:

```bash
bmm save https://example.com --tags article --ptag learning/rust
```

This creates the tag `learning/rust/article`.

### Import formats

- **HTML:** browser bookmark exports in the Netscape bookmark format, such as exports from Firefox or Chrome. Folder names are not imported as database folders; bookmark URLs, titles, and available tags are imported.
- **JSON:** an array of objects with a required `uri` and optional `title` and `tags` fields. `tags` is a comma-separated string.
- **TXT:** one URL per line. Imported text bookmarks have no title or tags unless added later.

Use `bmm import --help` for additional examples and options. A dry run prints the parsed bookmarks without saving them. Imports are limited to 9,999 bookmarks per invocation.

## Data, privacy, and backups

bmm stores bookmark data in local SQLite database files. The default database is named `bmm.db`; use the TUI database list or `--db` to work with other databases. On Unix-like systems, bmm respects an absolute `XDG_DATA_HOME` if set. Otherwise, it uses the platform's standard application data directory. `--db-path` can select an exact database file for a command.

TUI backup and restore shortcuts copy every `.db` file to/from a `links` folder outside bmm's data directory. The folder is:

- Android: `~/sdcard/Download(s)/links`
- Windows: `C:\links`
- Linux and macOS: `~/Download(s)/links`

The existing `Download` or `Downloads` directory is detected where possible. **Restore overwrites database files with the same names.** Keep an independent backup before restoring, and restart bmm after restoring a database that is already open in a session. The backup contains your local database files; it is not uploaded by bmm.

## Platform notes and configuration

### Android / Termux

Install both the Termux:API app and the `termux-api` package for clipboard and browser actions. Regular URL opening uses `termux-open-url`. Some browser-specific actions (such as private tabs and opening several links in separate tabs) use Android intents and default to supported Brave/Chrome package names.

### Optional environment variables

| Variable | Purpose |
| --- | --- |
| `XDG_DATA_HOME` | On Unix-like systems, choose the application data directory. It must be an absolute path. |
| `BMM_EDITOR` | Choose the editor used by `bmm save --editor` and `bmm notes`; takes precedence over `EDITOR`. |
| `EDITOR` | Fallback editor for CLI bookmark details and notes. |
| `BMM_BROWSER` | Choose a browser for private/incognito opens when automatic detection does not find one. |
| `BMM_BROWSER_INCOGNITO_FLAG` | Set the private-mode command-line flag for `BMM_BROWSER`; defaults to `--incognito`. |
| `BMM_ANDROID_BROWSER_PACKAGE` | Android only: set the package name used for browser-specific actions. |
| `BMM_ANDROID_BROWSER_ACTIVITY` | Android only: override the browser activity used with the package above. |

Private/incognito browser support depends on the platform and installed browser. The updater currently supports Linux x86_64 and Android aarch64; on other platforms, update by downloading a release or installing a new build manually.

## Development

Requirements: Rust **1.96** (the version pinned in `rust-toolchain.toml`) and Git.

```bash
git clone https://github.com/raisubham1024/bmm.git
cd bmm
cargo test --locked
cargo fmt --all -- --check
cargo build --release
```

To run the local build:

```bash
./target/release/bmm --help
./target/release/bmm
```

## Contributing

Bug reports, feature requests, and pull requests are welcome. Before opening an issue, please check [existing issues](https://github.com/raisubham1024/bmm/issues). For a pull request, describe the problem and solution, keep changes focused, and run the test and formatting commands above.

## Inspiration and attribution

This project is inspired by [dhth/bmm](https://github.com/dhth/bmm), whose simple approach to bookmark management motivated this work. This repository includes its own changes and adaptations, including Android/Termux support and additional workflows. The project is distributed under the [MIT License](LICENSE); see the license file for attribution and terms.

## License

[MIT](LICENSE)
