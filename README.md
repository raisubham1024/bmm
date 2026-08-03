<p align="center">
  <h1 align="center">bmm</h1>

</p>

**Why use bmm?**

Browser bookmarks get messy quickly. They're limited to a single browser, hard to search, and you don't know if a saved link is broken until you open it. `bmm` keeps all your bookmarks in one place, completely independent of any browser. You can instantly search for any bookmark from the terminal and open it in your default browser. It also lets you organize bookmarks into separate databases, add tags and notes, and automatically check for broken links. If you save a lot of links and want a fast, organized way to find, manage, and open them later, `bmm` is built for you.

`bmm` stores all your bookmarks and links on your own computer, so you can easily save, search, manage, and open them anytime. You can use simple terminal commands or the built-in terminal interface (TUI) to browse and manage your bookmarks in an easy way.


![tui-2](https://github.com/user-attachments/assets/a3dc5fb7-d258-461e-86b5-f2498dfbd4dc)

---

🤔 Motivation
---

I tried many different ways to save and organize bookmarks, but the GitHub repository **bmm** by **dhth** was the one I liked the most. It inspired me because of its simplicity and approach to bookmark management. However, I wanted a few additional features that would make it easier for everyday users. My goal is to create something that is simple to use, works well across different devices, and is easily accessible from Android phones as well. This project is inspired by **bmm**, but it is built with my own ideas and improvements. Finally, I'd like to thank my friend, whose encouragement motivated me to renew this project.

---


💾 Installation
---

### for Android
- first download `termux` from github [Click here to download Termux](https://github.com/termux/termux-app/releases/download/v0.118.3/termux-app_v0.118.3+github-debug_arm64-v8a.apk) OR you can download the appropriate release for your device or operating system from the GitHub Releases page (even it available in Playstore but that version is outdated, that's why download it from github)
- give permission to install 3rd party app (if asked). in installation time mobile give you warning but trust me everythings safe, if this security pop up then click on `install anyway`
- once install open Termux
- now, We run this command to stop the welcome message from appearing every time Termux opens.
  ```bash
  touch ~/.hushlogin
  ```
  &nbsp;
- now, run these command to update and upgrade the repository / pakages
  ```bash
  apt update && apt upgrade
  ```
  &nbsp;
- now time to download `bmm`, [Click here to download BMM for Android](https://github.com/raisubham1024/bmm/releases/download/final/bmm-android-aarch64)

**💁 Whether you're using Android or another device, you can download the appropriate version for your device or operating system from the GitHub Releases page.** => [Click here to see BMM for different devices](https://github.com/raisubham1024/bmm/releases/tag/final) 

---

💁 My Suggestion for normal users
---
To avoid the complexity of commands, you can use the `bmm tui` command to open the Terminal User Interface (TUI). It provides a simple and user-friendly experience, making bookmark management easy for everyone.

All TUI features you can see at the bottom of this page , see this section or click here to directly jump to that section  [[#TUI Reference Manual]]

---


⚡️ Usage
---

```text
Usage: bmm [OPTIONS] <COMMAND>

Commands:
  import    Import bookmarks from various sources
  delete    Delete bookmarks
  list      List bookmarks based on several kinds of queries
  save      Save/update a bookmark
  save-all  Save/update multiple bookmarks
  search    Search bookmarks by matching over terms
  show      Show bookmark details
  tags      Interact with tags
  check     Check bookmarks for broken/dead links
  notes     Add, edit, or view a note attached to a bookmark
  tui       Open bmm's TUI
  help      Print this message or the help of the given subcommand(s)

Options:
      --db-path <STRING>  Override bmm's database location (default: <DATA_DIR>/bmm/bmm.db)
      --debug             Output debug information without doing anything
  -h, --help              Print help (see more with '--help')
```


## Let's learn

there is 2 way 
1. [[#TUI]] (recommended)
2. [[#CLI]]

### TUI
first open it by command `bmm tui` 

*after that command, The screen you see is a default which is called a **Search mode** *
ℹ️ if you are on different mode and want to open a search mode again, press `s` (for search mode)


#### first understand that TUI (terminal User Interface) of bmm


> [!info] 
> Contents


- after running a command `bmm tui` the interface you see is called "search mode", where you can search, edit, delete the bookmarks/links and all you can see all saved links by just without writing something in search mode just press `enter` 
- when you are in search mode ,  press 'Esc' (Escape key) to switch to "Normal mode", where you can directly play with other functionalities like 'Databases, tags, etc'
- in normal mode


let's play with it :

1. let's save one bookmark/link in bmm
   - press 'a'  (you see )
```bash
#let's save one bookmark/link in bmm
press `a` to crera

```


### CLI


⌨ CLI mode
---

`bmm` allows every action it supports to be performed via its CLI. As such, it
can be easily integrated with other search tools (eg.
[Alfred](https://www.alfredapp.com/), [fzf](https://github.com/junegunn/fzf),
etc.)

![cli](https://github.com/user-attachments/assets/f8493e7c-8286-4fa4-8d49-6f34b5c5044b)

### Importing existing bookmarks

`bmm` allows importing bookmarks from various sources. It supports the following
input formats:

- HTML (These are bookmark files exported by browsers like Firefox, Chrome, etc,
  in the NETSCAPE-Bookmark-file-1 format.)
- JSON
- TXT

```bash
bmm import firefox.html
bmm import bookmarks.json --dry-run

# overwrite already saved attributes (title and tags) while importing
bmm import bookmarks.txt --reset-missing-details

# ignore errors related to bookmark title and tags
# if title is too long, it'll be trimmed, some invalid tags will be corrected
bmm import bookmarks.txt --ignore-attribute-errors
```

<details><summary> An example HTML file</summary>

```html
<!DOCTYPE NETSCAPE-Bookmark-file-1>
<!-- This is an automatically generated file.
     It will be read and overwritten.
     DO NOT EDIT! -->
<META HTTP-EQUIV="Content-Type" CONTENT="text/html; charset=UTF-8">
<meta http-equiv="Content-Security-Policy"
      content="default-src 'self'; script-src 'none'; img-src data: *; object-src 'none'"></meta>
<TITLE>Bookmarks</TITLE>
<H1>Bookmarks Menu</H1>

<DL><p>
    <DT><H3 ADD_DATE="1736450822" LAST_MODIFIED="1739920697" PERSONAL_TOOLBAR_FOLDER="true">Bookmarks Toolbar</H3>
    <DL><p>
        <DT><H3 ADD_DATE="1739896938" LAST_MODIFIED="1739920670">productivity</H3>
        <DL><p>
            <DT><H3 ADD_DATE="1739896992" LAST_MODIFIED="1739920767">crates</H3>
            <DL><p>
                <DT><A HREF="https://crates.io/crates/sqlx" ADD_DATE="1739897020" LAST_MODIFIED="1739897041" ICON_URI="https://crates.io/favicon.ico" TAGS="crates,rust">sqlx - crates.io: Rust Package Registry</A>
            </DL><p>
            <DT><A HREF="https://github.com/dhth/omm" ADD_DATE="1739920615" LAST_MODIFIED="1739920646" ICON_URI="https://github.com/fluidicon.png" TAGS="productivity,tools">GitHub - dhth/omm: on-my-mind: a keyboard-driven task manager for the command line</A>
            <DT><A HREF="https://github.com/dhth/hours" ADD_DATE="1739920661" LAST_MODIFIED="1739920670" ICON_URI="https://github.com/fluidicon.png" TAGS="productivity,tools">GitHub - dhth/hours: A no-frills time tracking toolkit for command line nerds</A>
        </DL><p>
        <DT><A HREF="https://github.com/dhth/bmm" ADD_DATE="1739920697" LAST_MODIFIED="1739920739" ICON_URI="https://github.com/fluidicon.png" TAGS="tools">GitHub - dhth/bmm: get to your bookmarks in a flash</A>
    </DL><p>
</DL>
```
</details>

<details><summary> An example JSON file</summary>

```json
[
  {
    "uri": "https://github.com/dhth/bmm",
    "title": null,
    "tags": "tools,bookmarks"
  },
  {
    "uri": "https://github.com/dhth/omm",
    "title": "on-my-mind: a keyboard-driven task manager for the command line",
    "tags": null
  }
]
```
</details>

<details><summary> An example TXT file</summary>

```text
https://github.com/dhth/bmm
https://github.com/dhth/omm
https://github.com/dhth/hours
```
</details>

### Saving/updating a bookmark

```bash
# save a new URI
bmm save https://github.com/dhth/bmm

# save a new URI with title and tags
bmm save https://github.com/dhth/omm \
    --title 'a keyboard-driven task manager for the command line' \
    --tags 'tools,productivity'

# update the title of a previously saved bookmark
bmm save https://github.com/dhth/bmm \
    --title 'yet another bookmarking tool'

# append to the tags of a previously saved bookmark
bmm save https://github.com/dhth/omm \
    --tags 'task-manager'

# use your editor to provide details
bmm save https://github.com/dhth/bmm -e

# mark a bookmark as starred/favorite while saving it
bmm save https://github.com/dhth/bmm --star
```

Note: if you leave out the scheme (eg. `github.com/dhth/bmm` instead of
`https://github.com/dhth/bmm`), `bmm` assumes `https://` by default.

### Saving/updating several bookmarks at a time

```bash
# save/update multiple bookmarks via arguments
bmm save \
    'https://github.com/dhth/bmm' \
    'https://github.com/dhth/omm' \
    --tags 'cli,bookmarks'

# save/update multiple bookmarks via stdin
cat << EOF | bmm save --tags tools --reset-missing-details -s
https://github.com/dhth/bmm
https://github.com/dhth/omm
https://github.com/dhth/hours
EOF
```

### Listing bookmarks based on several queries

`bmm` allows listing bookmarks based on queries on bookmark uri/title/tags. The
first two are pattern matched, while the last is matched exactly.

```bash
bmm list --uri 'github.com' \
    --title 'command line' \
    --tags 'tools,productivity' \
    --format json
```

### Searching bookmarks by terms

Sometimes you want to search for bookmarks without being very granular. The
`search` command allows you to do so. It accepts a list of terms, and will
return bookmarks where all of the terms are matched over any attribute or tag
belonging to a bookmark. You can also open the results in `bmm`'s TUI.

```bash
# search bookmarks based on search terms
bmm search cli rust tool bookmarks --format delimited

# open search results in bmm's TUI
bmm search cli rust tool bookmarks --tui
```

### Show bookmark details

```bash
bmm show 'https://github.com/dhth/bmm'
```

### Interaction with tags

```bash
# Show saved tags
bmm tags list \
    --format json \
    --show-stats

# open saved tags in bmm's TUI
bmm tags list --tui

# rename tag
bmm tags rename old-tag new-tag

# delete tags 
bmm tags delete tag1 tag2 tag3
```

### Delete bookmarks

```bash
bmm delete 'https://github.com/dhth/bmm' 'https://github.com/dhth/omm'

# skip confirmation
bmm delete --yes 'https://github.com/dhth/bmm'
```

### Check bookmarks for broken links

`bmm` can visit your saved bookmarks and report which ones are no longer
reachable (eg. the site is down, the page was removed, etc.)

```bash
# check all bookmarks
bmm check

# only check bookmarks that have a given tag
bmm check --tags golang

# by default, only broken links are shown; pass --show-all to see everything
bmm check --show-all

# check more links at the same time, and give up on slow ones sooner
bmm check --concurrency 20 --timeout 5
```

### Notes for a bookmark

`bmm` lets you attach a note to any saved bookmark — a couple of words, or
a full multi-paragraph writeup. Running `bmm notes <uri>` opens the note in
your text editor (the same one used for `bmm save -e`), pre-filled with
whatever's already there.

```bash
# add/edit a note (opens your $EDITOR)
bmm notes 'https://github.com/dhth/bmm'

# just print the current note, without opening an editor
bmm notes 'https://github.com/dhth/bmm' --print
```

📟 TUI mode
---

To allow for easy browsing, `bmm` ships with its own TUI. It can be launched
either in a generic mode (via `bmm tui`) or in the context of a specific command
(e.g., `bmm search tools --tui`).

The TUI lets you do the following:

- Search bookmarks based on terms
- List all tags
- View bookmarks that hold a tag

Feature requests for the TUI can be submitted via `bmm's` [issues
page](https://github.com/raisubham1024/bmm/issues).

![tui](https://github.com/user-attachments/assets/6ca63039-8872-4520-93da-1576cc0cf8ec)

### Opening bookmarks in incognito/private mode (TUI)

Inside the TUI, `i` opens the bookmark under the cursor in an incognito/
private window, and `I` does the same for every bookmark currently listed.
`bmm` tries to detect a supported browser automatically (Chrome, Chromium,
Brave, Edge, or Firefox). If it can't find one, or you want to force a
specific browser, set these environment variables:

```bash
# executable name (desktop) or app name (macOS) of your browser
export BMM_BROWSER=brave-browser

# only needed if your browser isn't chromium/firefox-style
export BMM_BROWSER_INCOGNITO_FLAG=--incognito
```

Note: on Android, incognito-opening only works if Chrome is your installed
browser, since Android has no generic "open incognito" mechanism.

### TUI Reference Manual

```text
bmm has eight views.

- Bookmarks List View
- Tags List View
- Edit Bookmark View
- Notes View
- Database List View
- New Database Name View
- Confirm View
- Help View (this one)


#you can see these views by pressing "Alt + m" in a TUI mode (bmm tui)

---


```
in TUI 


> [!info] bydefault TUI mode in 'Search mode', to use other functionality switch to 'Normal mode' by press 'Esc'


|Most important     |     |
| --- | --- |
|   Alt + m  |  to switch between different modes (like )   |


|General    |     |
| --- | --- |
|   ?   |   show/hide help view  |
|   Esc   |   go to normal mode  |
| q | quit the TUI mode |
| j / Down arrow | go down in a list |
| k / Up arrow | go up in a list |


|   Bookmarks related  |     |
| --- | --- |
|   s  |  show search box   |
|  Enter   |   submit search query  OR if you leave search box empty then press Enter, it shows all bookmarks  |
|   a  |   add a new bookmark (URL, title, tags)  |
|  t   |   show Tags List View for the active database (when search is not active)  |
|  T   |   show Tags List View across all databases at once - tag counts are summed across databases, and results from picking a tag show which database each bookmark is from  |
|   e  |   edit the title/tags of the bookmark under cursor  |
|  E   |  edit the URL (as well as title/tags) of the bookmark under cursor   |
|   n  |   add/edit a note for the bookmark under cursor (hidden)  |
|   N  |  delete the note for the bookmark under cursor, if it  has one (asks for confirmation) |
|   \*  |   toggle star on the bookmark under cursor  |
|   S  |   show only starred bookmarks  |
|   A  |  show/switch between databases   |
|  z   |   search across all databases at once (results show  which database each bookmark is from) |
|  d/delete   |   delete the bookmark under cursor (asks for confirmation)  |
|  D   |   delete every bookmark currently listed, all at once (asks for confirmation, and always shows exactly how many links will be deleted)  |
|  o   |   open URL in browser  |
|  i   |   open URL in browser (incognito/private window)  |
|  O   |  open all listed bookmarks in browser (max 30; warns if more)   |
|  I   |   open all listed bookmarks in browser, incognito/private  (max 30; warns if more) |
|  y   |   copy selected URL  |
|  Y   |    copy all the resulted URLs to system clipboard |


|  Tags List View    |     |
| --- | --- |
|  /   |  show tag search input   |
|   Enter  |  show bookmarks that are tagged with the one under cursor (across every database, if the Tags List View was opened with "T")   |
|  Alt+e  |  rename the tag under cursor - updates every bookmark that used it (across every database, if opened with "T"); if the new name you type already exists as a tag, it's offered as a suggestion so you can merge into it instead of creating a duplicate; Ctrl+s to save, Esc to cancel   |


|Edit Bookmark View     |     |
| --- | --- |
|  Tab / Down   |  move to the next field   |
|   Shift+Tab / Up  |  move to the previous field   |
|  Ctrl+s   |  save changes (asks for confirmation)   |
|   Esc  |   cancel editing (asks for confirmation if there are unsaved changes) |


|  Notes View   |     |
| --- | --- |
|   Ctrl+s  |  save the note (asks for confirmation)   |
|   Esc  |   cancel (asks for confirmation if there are  unsaved changes)|


|   Database List View  |     |
| --- | --- |
|   Enter  |   switch to the database under cursor for this session (resets to bmm.db next time you start bmm) |
|  C   |  create a new database  and then "ctrl+s" to save |
|  Esc   |   go back  |




