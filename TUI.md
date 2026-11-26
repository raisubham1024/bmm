# TUI guide

This guide explains how to use **bmm's terminal user interface (TUI)**. You can use it without knowing command-line options: open bmm, press keys, and follow the labels shown on screen.

## Start bmm

Run `bmm` or `bmm tui` in a terminal. The TUI opens with the search box ready. Type a search to find bookmarks, or press `Enter` while the box is empty to show all bookmarks. Press `?` at any time to open bmm's built-in shortcut guide.

> **Key names:** `Alt+key` means hold `Alt` while pressing the other key. `Ctrl+s` means hold `Ctrl` while pressing `s`. Uppercase keys are different: for example, `Alt+b` backs up databases, while `Alt+B` checks links.

## Moving around

| Key | What it does |
| --- | --- |
| `j` or `Down` | Move down in a list. |
| `k` or `Up` | Move up in a list. |
| `g` | Jump to the first item. |
| `G` | Jump to the last item. |
| `Enter` | Open or choose the highlighted item; the exact action depends on the screen. |
| `Esc` or `q` | Go back, close a popup, cancel an input, or exit from the main screen. |
| `?` | Open the built-in help tables. In Help, press `?`, `q`, or `Esc` to return. |
| `Alt+m` | Open or close the mode switcher from anywhere, including while typing. |

When a terminal is too small, bmm asks you to enlarge it. The minimum size is **40 columns by 16 rows**.

## Main bookmark list

This is where you view and manage links. The details area shows the selected bookmark's URL, title, tags, and whether it has a note.

| Key | What it does |
| --- | --- |
| `l` | Show every bookmark in the current database and clear the current filter. |
| `s` | Search in the current database. Results update as you type. |
| `a` | Add a bookmark. |
| `e` | Edit the selected bookmark's title and tags. The URL stays read-only. |
| `E` | Edit the selected bookmark's URL, title, and tags. |
| `n` / `N` | Add or edit a note / delete the selected bookmark's note. Deleting asks for confirmation. |
| `*` | Star or unstar the selected bookmark. |
| `S` | Show only starred bookmarks. |
| `t` or `Tab` | Browse tags in the current database. `Tab` works when you are not typing in a search box. |
| `T` | Browse tags from all local databases together. Selecting a tag shows matching links from all of them. |
| `z` | Search across all local databases. Each result shows which database it belongs to. |
| `Alt+n` | Search bookmark notes. Press it again to leave note search. |
| `Alt+B` | Check the links currently shown. bmm displays broken links and why they failed; press `Esc` to return or cancel a running check. |
| `o` / `i` | Open the selected link normally / in a private window, where supported. |
| `O` / `I` | Open all listed links normally / privately (up to 30 links). |
| `y` / `Y` | Copy the selected URL / all listed URLs to the clipboard. |
| `d` or `Delete` | Delete the selected bookmark after confirmation. |
| `D` | Delete all bookmarks currently shown after confirmation. Check the count in the confirmation first. |
| `Space` | Mark or unmark the selected bookmark for a bulk move. |
| `m` / `M` | Move the selected bookmark / all marked bookmarks to another database. bmm asks you to confirm. |
| `A` | Open the database list to switch databases. |
| `F` | Open database folders. |
| `Alt+b` / `Alt+g` | Back up / restore database files. Restore overwrites files with matching names. |
| `Alt+u` | Check for and install an update on supported platforms. |

If you start a search from the first screen, `Esc` first returns to the full bookmark list. From the main list, `Esc` or `q` exits bmm when there is no previous view to return to.

## Search

### Search in one database

Press `s`, then type one or more words. bmm searches the selected fields as you type; press `Enter` to close the search box and keep the results. Press `Enter` with an empty query to show all bookmarks. Press `Esc` to cancel the search and return to the previous view.

### Choose what to search

While the search box is open, or before starting a search from the bookmark list, press `Alt+s`. This opens the search-scope menu. It works for both `s` and `z` searches.

| Choice | What it searches |
| --- | --- |
| All | URL, title/description, and tags. |
| URL | Links only. |
| Description | Bookmark titles only. |
| Tags | Tags only; matching tag suggestions appear as you type. |

Use `Up`/`Down` or `j`/`k` to move, then `Enter` to choose. You can also press `1`–`4` to choose a scope directly. Press `Esc`, `q`, or `Alt+s` to close without changing it. When tag suggestions are visible, use `Up`/`Down` to choose one and `Enter` to insert it; `Esc` hides the suggestions first.

### Search notes

Press `Alt+n` to search within notes instead of bookmark details. Type words to filter the results. Press `Alt+n` again to leave note search.

## Add and edit bookmarks

In the bookmark form, enter a URL, an optional title, and tags separated by commas (for example, `reading, rust`). A URL without `https://` or another scheme is fixed up when saved.

| Key | What it does |
| --- | --- |
| `Tab` or `Down` | Move to the next field. |
| `Shift+Tab` or `Up` | Move to the previous field. |
| `Ctrl+s` | Save. bmm asks you to confirm. |
| `Esc` | Cancel. If you have unsaved changes, bmm asks before discarding them. |
| `Alt+F` in the Title field | Fetch the page description and put it in the title field, if the page has one. |

### Tag suggestions while typing

Focus the Tags field and start typing after the last comma. bmm shows matching tags from the current database. Matching is case-insensitive; a tag can match from its beginning or when the typed letters appear in order (for example, `leanet` can find `learn_networking`). Already-added tags are not suggested again.

| Key | What it does |
| --- | --- |
| `Down` / `Up` | Move through the suggestions. |
| `Enter` | Add the highlighted tag and leave the cursor ready for another tag. |
| `Esc` | Hide suggestions without leaving the form. Press it again to cancel the form. |
| `Alt+z` | Show all saved tags, with matches first. Press again to return to matching tags only. Select several tags with `Up`/`Down` and `Enter`. |
| `Alt+t` | Include tags from other local databases as well. Press again to use only the current database's tags. |

`Alt+t` is off by default. It affects suggestions in the form, not which database the bookmark is saved in.

## Tags

Open the current database's tag list with `t` or `Tab`, or all databases' tags with `T`. The list shows how many bookmarks use each tag. Select a tag and press `Enter` to see its bookmarks.

| Key | What it does |
| --- | --- |
| `/` | Search/filter the tag list as you type. Matching letters can appear in order, even if they are not next to each other. |
| `Enter` | Show bookmarks with the selected tag. |
| `Alt+e` | Rename the selected tag. The new name is applied to its bookmarks. If you opened all databases' tags with `T`, the rename applies across databases too. |
| `Up` / `Down` while searching | Move through the filtered tags. |
| `Enter` while searching | Keep the filter and return to the tag list. |
| `Esc` while searching | Cancel the filter. |

When renaming, type the new name and press `Ctrl+s` to save, or `Esc` to cancel. Existing matching tag names may appear below the input; choose one with `Up`/`Down` and `Enter` to merge the renamed tag into it. Tags can be nested with `/`, for example `learning/rust`.

## Notes

Press `n` on a bookmark to add or edit its private note. Notes are not shown in the bookmark list, but you can search them with `Alt+n`.

| Key | What it does |
| --- | --- |
| `Ctrl+s` | Save the note (asks for confirmation). |
| `Esc` | Cancel. If the note changed, bmm asks before discarding it. |
| `N` from the bookmark list | Delete the selected bookmark's note (asks for confirmation). |

## Databases

Press `A` to open the database list. Each row shows a database name and, when available, how many bookmarks it contains. The active database is marked. Switching databases applies for this bmm session; a new session starts with `bmm.db`.

| Key | What it does |
| --- | --- |
| `Up` / `Down`, `j` / `k` | Move through databases. |
| `Enter` or `1`–`9` | Switch to the highlighted database, or directly to the numbered database. In a move flow, choose the destination instead. |
| `Alt+1`, `Alt+2`, etc. | Jump to database numbers 10 and above. For example, press `Alt+1`, then `Alt+2` for database 12. If the number could continue, pause briefly to select it. |
| `/` | Search database names. You can type matching letters in order; for example, `ledata` can find `learn_database.db`. |
| `C` | Create a database (when switching databases). bmm adds `.db` if needed and switches to it. |
| `R` | Rename the selected database. The default `bmm.db` cannot be renamed. |
| `v` | Cycle the database sort order. |
| `Alt+Super+1`–`Alt+Super+5` | Choose a sort order directly, if your terminal supports the `Super` key. |
| `a` | Arrange databases while using the custom order (view 1). Use `Up`/`Down`, then `Enter` or `a` to save; `Esc` cancels. Clear any database-name filter first. |
| `F` | Open database folders. |
| Mouse click / touch | Select a database. The scroll wheel moves through the list. |
| `Esc` | Go back. |

Database sort orders are: **1** your arranged order, **2** name A–Z (the default), **3** name Z–A, **4** fewest bookmarks first, and **5** most bookmarks first. Pressing `v` moves to the next order.

## Database folders

Folders are labels that group databases; they do not move or rename database files. From the database list, press `F` to open folders. The `(no folder)` row contains databases that are not in a folder.

### Folder list

| Key | What it does |
| --- | --- |
| `Enter` or `1`–`9` | Open the selected folder. |
| `C` | Create a folder. Emoji are allowed in names. |
| `R` | Rename the selected folder. |
| `D` | Delete the selected folder. Its databases stay safe and become unfiled. |
| `Up` / `Down`, `j` / `k` | Move through folders. |
| `Esc` or `q` | Go back. |

### Databases inside a folder

| Key | What it does |
| --- | --- |
| `Enter` or `1`–`9` | Switch to the selected database. |
| `Space` | Mark or unmark a database. |
| `m` | Move the marked databases, or just the selected database if none are marked, to another folder. Choose `(no folder)` to remove them from folders. |
| `Up` / `Down`, `j` / `k` | Move through databases. |
| Mouse click / touch | Select a row; the scroll wheel moves through the list. |
| `Esc` or `q` | Return to the folder list. |

When choosing a destination folder, `Enter` moves the databases there. Folders only organize databases in bmm; deleting a folder never deletes its databases or bookmarks.

## Mode switcher and help

Press `Alt+m` from any screen to open the mode switcher. Move with `Up`/`Down` or `j`/`k`, then press `Enter`; or press the number shown beside a mode to open it immediately.

| Number | Mode | Usual shortcut |
| --- | --- | --- |
| `1` | All bookmarks in the current database | `l` |
| `2` | Search in the current database | `Alt+s` |
| `3` | Browse tags | `t` |
| `4` | Starred bookmarks | `S` |
| `5` | Search across databases | `z` |
| `6` | Search notes | `Alt+n` |
| `7` | Switch databases | `A` |
| `8` | Help | `?` |
| `9` | Database folders | `F` |

Press `Alt+m`, `Esc`, or `q` to close the switcher without changing screens. The Help view (`?`) contains the in-app shortcut tables; scroll with `j`/`k` or the arrow keys, jump to the beginning/end with `g`/`G`, and close with `?`, `q`, or `Esc`.

## Back up and restore

`Alt+b` copies all local `.db` database files to a `links` folder outside bmm's data folder. `Alt+g` copies database files from that folder back into bmm. Both shortcuts work from any screen.

The folder is usually `~/Download(s)/links` on Linux/macOS, `C:\links` on Windows, and `~/sdcard/Download(s)/links` on Android. **Restore overwrites databases with the same file names.** Make a backup before restoring. If a database was open during restore, restart bmm to see restored data.

## Android / Termux

Clipboard and browser actions need the Termux:API app and the `termux-api` package. On Android, private/incognito browser actions may have platform-specific limits; bmm may copy a link so you can paste it into a private tab. See [TERMUX.md](TERMUX.md) for setup details.

## Need more help?

- Press `?` inside bmm for the built-in help tables.
- See [README.md](README.md) for installation, the command-line interface, and other project details.
- Run `bmm --help` for CLI commands.
