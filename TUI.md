# TUI guide

This guide explains every screen and shortcut of bmm's Terminal User Interface (TUI) in simple language.

## Start the TUI

```bash
bmm
# or
bmm tui
```

Running `bmm` with no command opens the TUI. The TUI needs a terminal at least **40 columns wide and 16 rows high**.

Press `?` anywhere in the TUI to open the built-in Help view. It contains scrollable tables and can be closed with `Esc`, `q`, or `?`.

## Basic navigation

| Key | Action |
| --- | --- |
| `j` or `Down` | Move down. |
| `k` or `Up` | Move up. |
| `g` | Jump to the first item. |
| `G` | Jump to the last item. |
| `Enter` | Open or select the highlighted item. |
| `Esc` or `q` | Go back, close a popup, or exit bmm from the main screen. |
| `Alt+m` | Open or close the mode switcher from almost any screen. |
| Mouse / touch | Available in database and folder lists: click or tap a row, and use the scroll wheel. |

Hold `Shift` while dragging with the mouse if you want to select terminal text instead of using it for TUI navigation.

## Main bookmark list

The main list shows bookmarks in the active database. Details for the selected bookmark are shown below the list.

| Key | Action |
| --- | --- |
| `l` | Show all bookmarks and clear active search/star/tag filters. |
| `s` | Open live bookmark search. |
| `Enter` in an empty search | Show all bookmarks. |
| `a` | Add a bookmark. |
| `e` | Edit the selected bookmark's title and tags. |
| `E` | Edit its URL, title, and tags. |
| `n` | Add or edit its private note. |
| `N` | Delete its note after confirmation. |
| `*` | Star or unstar the bookmark. |
| `S` | Show starred bookmarks. |
| `t` or `Tab` | Open tags in the active database. |
| `T` | Open tags across all databases. |
| `Ctrl+T` | After opening a tag's bookmarks, show tags connected to that tag across databases. |
| `z` | Search across all databases. |
| `Alt+n` | Toggle note search. |
| `Alt+s` | Open the search scope picker. |
| `Alt+B` | Check every listed link and show broken links in red. |
| `Esc` while checking | Cancel a running link check. |
| `o` / `i` | Open the selected URL normally / privately. |
| `O` / `I` | Open all listed URLs normally / privately, up to 30 links. |
| `y` / `Y` | Copy the selected URL / all listed URLs. |
| `d` or `Delete` | Delete the selected bookmark after confirmation. |
| `D` | Delete every currently listed bookmark after confirmation. |
| `Space` | Mark or unmark a bookmark for a bulk move. |
| `m` | Move the selected bookmark to another database. |
| `M` | Move all marked bookmarks. |
| `A` | Open databases. |
| `F` | Open database folders. |
| `Alt+b` / `Alt+g` | Back up / restore all databases. |
| `Alt+u` | Check for and install a bmm update. |

### Search

Press `s` to open the search box. Results update as you type. Press `Enter` to confirm the search and close the search box. Press `Esc` to cancel it.

Search terms can match URL, title/description, or tags. Press `Alt+s` to choose a narrower scope:

| Search scope | Matches |
| --- | --- |
| Everything | URL, title/description, and tags. |
| URL only | URLs only. |
| Description only | Bookmark titles/descriptions only. |
| Tags only | Tag names only. Tag suggestions appear as you type. |

Use `j`/`k`, `Up`/`Down`, or `1`–`4` in the scope picker. Press `Enter` to apply it. `Esc`, `q`, or `Alt+s` closes the picker without changing the scope.

In tags-only search, use `Up`/`Down` to highlight a matching tag and `Enter` to insert it. Press `Esc` to hide tag suggestions first.

Press `Alt+n` to search inside private notes. Press it again to leave note-search mode.

## Add and edit a bookmark

Press `a` to add a bookmark or `e`/`E` to edit one.

- `e` edits only title and tags.
- `E` also makes the URL editable.
- Tags are comma-separated, for example `reading,rust,learning/rust`.
- Missing URL schemes are normalized automatically.
- Press `Ctrl+s` to save. Unsaved changes ask for confirmation before being discarded.

| Key | Action |
| --- | --- |
| `Tab` or `Down` | Move to the next field. |
| `Shift+Tab` or `Up` | Move to the previous field. |
| `Ctrl+s` | Save and confirm. |
| `Esc` | Cancel or close a suggestion box; confirm unsaved changes if necessary. |
| `Alt+F` in the Title field | Fetch the page description and put it in the Title field. |

### Tag suggestions

While typing in the Tags field, bmm suggests saved tags after the last comma.

| Key | Action |
| --- | --- |
| `Down` / `Up` | Move through suggestions. |
| `Enter` | Insert the highlighted tag and add `, ` for the next tag. |
| `Esc` | Hide suggestions without leaving the edit screen. |
| `Alt+z` | Show every saved tag in the active or allowed suggestion pool. Press again to return to matching-only suggestions. |
| `Alt+t` | Include tags from every local database. Press again to use only the active database. |

Live matching supports:

- names beginning with typed text;
- typed words that begin after `_`, `-`, spaces, or another separator;
- text appearing inside a tag name;
- letters typed in the same order as a looser fuzzy match.

`Alt+t` is off when the add/edit screen opens and affects suggestions only; it does not change where the bookmark is saved.

## Tags

Press `t` or `Tab` for tags in the active database. Press `T` for tags across all databases.

| Key | Action |
| --- | --- |
| `/` | Open tag-name search/filter. |
| `Enter` | Show bookmarks with the highlighted tag. |
| `*` | Star or unstar the highlighted tag. |
| `S` | Show only starred tags; press again to show all tags. |
| `Alt+e` | Rename or merge the highlighted tag. |
| `Ctrl+T` | Show tags connected to a tag when currently viewing that tag's bookmarks. |
| `j` / `k` or arrows | Move in the tag list. |
| `g` / `G` | Jump to the first / last tag. |

A starred tag is marked with `★`. In the `T` view, starring applies to the tag in every database. Renaming a starred tag preserves the star. Tag-filter behavior remains separate for `t` and `T` views.

### Tag search matching

Tag search accepts close or letter-in-order matches, for example:

- `rust` matches `rust` or `rust_lang`;
- `dp` can match `edit_dp`, `dp_notes`, or `dopamine`;
- `leanet` can match `learn_networking`.

### Rename tags

Press `Alt+e` in the Tags List. Type the new name and press `Ctrl+s` to save, or `Esc` to cancel.

If the typed name matches an existing tag, bmm offers that tag as a merge suggestion. Use `Up`/`Down` and `Enter` to choose it. Renaming or merging updates every bookmark that used the old tag. In `T` mode, it applies across databases.

### Find related tags

First open a tag with `Enter` from the Tags List. The bookmark list now remembers that tag.

Press `Ctrl+T` to open a related-tags view from all databases:

- the origin tag is marked "this tag";
- tags used together with it are one step away;
- tags used together with those are the next step, and so on;
- nearer tags are listed first, then tags are sorted by name.

Press `Enter` on a related tag to show its bookmarks. Press `Ctrl+T` again on that bookmark list to continue exploring.

## Notes

Press `n` to add or edit the selected bookmark's note. Press `N` to delete it.

| Key | Action |
| --- | --- |
| `Ctrl+s` | Save the note. |
| `Esc` | Cancel and ask before discarding unsaved changes. |
| `Alt+n` | Search all bookmark notes from the main list. |

Notes are private text fields attached to bookmarks. They do not appear in normal bookmark URLs/details unless the view indicates that a bookmark has a note.

## Databases

Press `A` to open the Database List.

| Key | Action |
| --- | --- |
| `1`–`9` | Open the numbered database. |
| `Alt+<digits>` | Jump to databases numbered 10 and above. For database 12: `Alt+1`, then `Alt+2`. |
| `/` | Search database names. |
| `Enter` | Switch to the highlighted database or choose it as a bookmark move destination. |
| `C` | Create a new database while switching databases. |
| `R` | Rename the highlighted database. |
| `a` | Arrange databases in custom order. |
| `v` | Cycle through the five database sort modes. |
| `Alt+Super+1`–`5` | Choose a database sort mode in terminals that support the Super key. |
| `F` | Open database folders. |
| `Esc` | Go back. |

A new bmm session opens `bmm.db` by default. Switching databases is session-based. Database names accept `.db` automatically when missing. `bmm.db` cannot be renamed.

Database sort modes:

| Mode | Order |
| --- | --- |
| 1 | Your custom arranged order |
| 2 | Name A–Z (default) |
| 3 | Name Z–A |
| 4 | Fewest bookmarks first |
| 5 | Most bookmarks first |

During arranging with `a`, use `Up`/`Down` to choose a number, then `Enter` or `a` to save. `Esc` cancels.

## Database folders

Press `F` from the Database List to group databases into folders. Folders are labels only; database files do not move.

The `(no folder)` row contains databases not assigned to a folder.

### Folder list

| Key | Action |
| --- | --- |
| `Enter` | Open the folder. |
| `C` | Create a folder. |
| `R` | Rename the highlighted folder. |
| `D` | Delete the folder. Databases inside become unfiled. |
| `m` | In folder contents, move marked/selected databases to another folder. |
| `Space` | Mark/unmark a database inside the folder. |
| `Esc` | Go back. |

Emoji are allowed in folder names. Folder names can contain up to 60 characters. `(no folder)` is reserved and cannot be used as a folder name.

## Confirmations

Risky actions ask for confirmation in a yes/no popup.

| Key | Action |
| --- | --- |
| `y` or `Y` | Confirm yes. |
| `n`, `N`, or `Esc` | Cancel. |

Confirmed actions include deleting bookmarks, deleting notes, saving changes, moving bookmarks, and bulk operations.

## Mode switcher

Press `Alt+m` to open the mode switcher.

| Number | Mode |
| --- | --- |
| 1 | All bookmarks in the current database |
| 2 | Search scope/search mode |
| 3 | Tags |
| 4 | Starred bookmarks |
| 5 | Search across databases |
| 6 | Note search |
| 7 | Switch database |
| 8 | Help |
| 9 | Database folders |

Use `j`/`k` or arrows, press `Enter`, or press `1`–`9` to select the mode. `Alt+m`, `Esc`, or `q` closes the switcher without changing views.

## Backup and restore

`Alt+b` copies every local `.db` file to an external `links` folder. `Alt+g` copies files from that folder back into bmm.

| Platform | Folder |
| --- | --- |
| Android | `~/sdcard/Download(s)/links` |
| Windows | `C:\links` |
| Linux/macOS | `~/Download(s)/links` |

bmm detects an existing `Download` or `Downloads` folder case-insensitively. Restore overwrites database files with the same names. Keep a separate backup and restart bmm after restoring if a database was open.

## Useful examples

```bash
# Open TUI
bmm

# Open TUI and save an old-style search context
bmm tui

# Save from CLI
bmm save https://example.com --title Example --tags reference

# Search from CLI
bmm search rust programming

# Use a different database
bmm list --db work
```

For installation and CLI options, see [README.md](README.md). For Termux-specific setup, see [TERMUX.md](TERMUX.md).
