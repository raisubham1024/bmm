


<p align="center">
  <h1 align="center">bmm</h1>
&nbsp;

**🧙 Why use bmm?**
---
Browser bookmarks get messy quickly. They're limited to a single browser, hard to search, and you don't know if a saved link is broken until you open it. `bmm` keeps all your bookmarks in one place, completely independent of any browser. You can instantly search for any bookmark from the terminal and open it in your default browser. It also lets you organize bookmarks into separate databases, add tags and notes, automatically check for broken links, and detect duplicate bookmarks. If you save a lot of links and want a fast, organized way to find, manage, and open them later, `bmm` is built for you.

`bmm` stores all your bookmarks and links on your own computer, so you can easily save, search, manage, and open them anytime. You can use simple terminal commands or the built-in terminal interface (TUI) to browse and manage your bookmarks in an easy way.

All TUI features you can see at the bottom of this page , see this section or click here to directly jump to that section  [[#TUI Reference Manual]]

&nbsp;


🤔 Motivation
---

I tried many different ways to save and organize bookmarks, but the GitHub repository **bmm** by **dhth** was the one I liked the most. It inspired me because of its simplicity and approach to bookmark management. However, I wanted a few additional features that would make it easier for everyday users. My goal is to create something that is simple to use, works well across different devices, and is easily accessible from Android phones as well. This project is inspired by **bmm**, but it is built with my own ideas and improvements. Finally, I'd like to thank my friend, whose encouragement motivated me to renew this project.

&nbsp;

💾 Installation
---

## for Android
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

💁 Whether you're using Android or another device, you can download the appropriate version for your device or operating system from the GitHub Releases page => [Click here to see BMM for different devices](https://github.com/raisubham1024/bmm/releases/tag/final) 

&nbsp;
&nbsp;
&nbsp;
## for Linux
- download this file "bmm-linux-x86_64" from this link [click here](https://github.com/raisubham1024/bmm/releases/tag/final)
- rename that file 'bmm-linux-x86_64' to **bmm**
- move that file to this location `~/usr/bin` as a superuser. (so you can directly access this tool to any location)
- now run `bmm` command, it's working now.


&nbsp;
&nbsp;
&nbsp;


## for other system

- first install `rust` and `git` in your system
- then, install though this command (⏳️ it takes 5-10 min)
  ```bash
   cargo install --git https://github.com/raisubham1024/bmm.git
  ```
  &nbsp;
- now run `bmm` command, it's working now.

&nbsp;
&nbsp;
&nbsp;

# 💁 Let's learn


first open it by command `bmm` and press enter.
*after that command, The screen you see is a default which is called a **Search mode** *

> ℹ️ if you are on different mode and want to open a search mode again, press `s` (for search mode)

&nbsp;
## first understand that interface of bmm

- **Search Mode:**  
    When you run the `bmm` command, the interface you see is called **Search Mode**.
    
    - You can search for your saved bookmarks/links.
        
    - You can edit or delete bookmarks/links.
        
    - If you press `Enter` on an empty search, it shows **all your saved bookmarks/links**.
        
    - In **All Bookmarks**, you can see the **total number of saved links**.
        
- **Using `Esc` in Search Mode:**
    
    - When you are in Search Mode, press `Esc` to see **all your saved bookmarks**.
        
    - Press `Esc` again to **exit BMM**.
        
    - You can also press **`q`** to **quit/exit BMM**.
        
- **`Alt + m` – Show All Modes:**  
    Press `Alt + m` to see **all available modes/options** in BMM.
    
    - Each mode has a **number**.
        
    - Pressing that number takes you **directly to that mode**.
        
- **Switch Database:**  
    In the `Alt + m` menu, there is a **Switch Database** mode.
    
    - You can use it to create and switch between **different databases** for different types of links.
        
    - For example, you can have separate databases for study links, work links, or personal links.
        
    - **Important:** Even if you switch to another database, when you close BMM and open it again, it will start with the default **`bmm` database**.

&nbsp;

## 📌 1️⃣ create, 2️⃣ modify, 3️⃣ search, 4️⃣ delete a bookmark/link

&nbsp

### 1️⃣ create a bookmark
let's create a bookmark of any link
- first open bmm by `bmm` command
- press enter to show all bookmarks (currently we don't have any so it show blank)
- lets add the new links by pressing `a`
- you see this window, where you add 'URL', 'Description', and 'Tags'
  <img width="1460" height="246" alt="Image" src="https://github.com/user-attachments/assets/9180c060-fa1d-4a29-a246-a9b2b060ac06" />
&nbsp;
#### LET'S UNDERSTAND ABOUT THESE SECTIONS  :

> ℹ️ to switch between section : 
> press `Tab` to next section (top to bottom) and press `Shift + Tab` to previous section (bottom to top)

> 💁 if you want to erase something in section try this shorcut to done quickly

| Key        | Description                              |
| ---------- | ---------------------------------------- |
| `Ctrl + u` | Deletes the whole line before the cursor |
| `Ctrl + k` | Deletes the whole line after the cursor  |

&nbsp;
&nbsp;
&nbsp;

### 1. URL section
here you copy paste the link of your website (for example : `https://wikipedia.com` )
ℹ️ even if you type directly like this  `wikipedia.com` and it directly save as `https://wikipedia.com`

💁 No matter where the link comes from, a website, Instagram, Reddit, a video, a photo, or any other platform, **a link is a link**. Just paste or manually type it into the URL section.

**now, press `Tab` to switch to 'Title section'**

&nbsp;
&nbsp;
### 2. Title section
here you can manually type description about of the link.

OR

every website have there description, so if you want to extract that description of that website, just press `Alt + f` and it automatically extract / fetch that description and automatically write in 'Title section' (it takes 3-4 seconds to extract description)

**now, press `Tab` to switch to 'Tags section'**

&nbsp;
&nbsp;

### 3. Tags section
here you can give a Tag name. 
Tags let you group your bookmarks into categories (like "work", "coding", "reading"), making it easy to quickly search or filter bookmarks by topic, which you can see in Tag mode (empty search press enter to see all bookmarks then press `t` to see all tags).

> ℹ️ you can add multiple tags, so it mean a single link contain more than one tag.

&nbsp;
- let give that link `<link>` , some tags 
- give tags like `knowledge` and `wiki` seprated by comma `,`

&nbsp;
**Now press `Ctrl + s` to save the bookmark/link. it asks `y` for 'yes' and 'n' for no**

&nbsp;
&nbsp;
&nbsp;
### 2️⃣ Modify a bookmark
- run `bmm` command, you see search mode
- without typing anything, just press `enter` to see all bookmarks
- choose that link by up arrow or down arrow
- if you want to modify "Description + Tags" for this press `e` OR if you want to modify all section "URL + Description + Tags" then press `E`
- after that `ctrl + s` for save the modification.

> ℹ️ I built these two separate functionalities, `e` and `E`, to prevent you from accidentally modifying the URL.

&nbsp;
&nbsp;
&nbsp;

### 3️⃣ Search a bookmark

There is lots of way to get your result (it's better to understand all ways)  :

### (default) Search mode
you saved multiple bookmarks and now it's time to search it.
when you open `bmm` tool, default search mode you see. in that search mode whatever you type it searches on URL + Description + Tags. 
so if you search like `wiki demo rust` so suppose you saved a bookmark, where **wiki** word in 'Description',  **demo** word in Tags , **rust** word in 'URL' (no matter the search order)

> ℹ️ search order doesn't matter, if any link/bookmark contain that word, you get the result

> ℹ️ half searching functionality. suppose you wanna search `mountain knowledge` (which contain large size words), so instead of typing every single word, you can half search it like this  `mou kno` (you get the same result)  😉





----


### BMM Reference Manual

```text
#you can see these views by pressing "Alt + m" in a TUI mode (bmm tui)

1. all bookmarks of current database [l]
2. Separate searching mode [Alt+s]
3. show all Tags [t]
4. show Star/imp bookmarks [S]
5. search across all databases [z]
6. search notes [Alt+n]
7. switch database [A]
8. help [?]
```

| key     | Description                       |
| ------- | --------------------------------- |
| Alt + m | to switch between different modes |

---

**General**

> [!info] again reminder 💁
> when you open `bmm` tool, default you see a search mode, so these 'key' not work you have to switch to different mode or you do these things
> - let empty the search box and press enter OR press 'Esc', both do the same things it show you list of saved bookmarks of bmm
>   
>   now you can use these key bindings 👇️



| key            | Description                                                                          |
| -------------- | ------------------------------------------------------------------------------------ |
| ?              | show/hide help view                                                                  |
| Esc            | go back one step through the views you visited (e.g. links of a tag -> tags list -> the view you started from); exits only when there is nothing left to go back to |
| Alt + B (capital) | check the links shown in the current list; shows only the broken ones in red with the reason (404, DNS error, timeout...). `Esc` goes back |
| q              | exit from the bmm tool                                                               |
| Down arrow / j | go down in a list                                                                    |
| Up arrow / k   | go up in a list                                                                      |

---

**Bookmarks related**

| key      | Description                                                                                                |
| -------- | ---------------------------------------------------------------------------------------------------------- |
| s        | show search box                                                                                            |
| Enter    | submit search query  OR if you leave search box empty then press Enter, it shows all bookmarks             |
| a        | add a new bookmark (URL, title, tags)                                                                      |
| t        | show Tags List View (when search is not active)                                                            |
| T        | show Tags List View across all databases                                                                   |
| d        | delete the selected bookmark/link                                                                          |
| D        | delete all the resulted bookmarks/links                                                                    |
| e        | edit the title/tags of the bookmark under cursor                                                           |
| E        | edit the URL (as well as title/tags) of the bookmark under cursor                                          |
| n        | add/edit a note for the bookmark under cursor (hidden)                                                     |
| N        | delete the note for the bookmark under cursor, if it  has one (asks for confirmation)                      |
| \*       | toggle star on the bookmark under cursor                                                                   |
| S        | show only starred bookmarks                                                                                |
| A        | show/switch between databases                                                                              |
| z        | search across all databases at once (results show  which database each bookmark is from)                   |
| D/delete | delete the selected bookmark  (asks for confirmation)                                                      |
| o        | open URL in browser                                                                                        |
| i        | open URL in browser (incognito/private window)  `🤧 not works on Android`                                  |
| O        | open all listed bookmarks in browser (max 30; warns if more) `🤧 not works on Android`                     |
| I        | open all listed bookmarks in browser, incognito/private  (max 30; warns if more) `🤧 not works on Android` |
| y        | copy selected URL                                                                                          |
| Y        | copy all the resulted URLs to system clipboard                                                             |


> [!tip] Tip
> Links won't open directly in incognito on Android. Press `y` to copy the link, then paste manually into incognito. Use this workaround until the issue is resolved.


---

**Tags List View**


> [!info] 
> to show all tags :
> - default you open search mode , then press `enter` to show all bookmarks then press `t` to show all tags
>   
>   OR
>   
> - press `Alt + m` to switch mode and press the number  `3` to show all bookmarks

|  key    | Description    |
| --- | --- |
|  /   |  show tag search input   |
|   Enter  |  show bookmarks that are tagged with the one under cursor   |

---

**Edit Bookmark View**
> [!info] 
> search and select the link by pressing enter, then press `e` for edit "Title  + Tag" OR `E` for edit "URL + Title + Tag"
> 
> 💁 * `e` and `E` , two different functionalty, So that you don't accidentally change the URL while editing.*

|key     |  Description  |
| --- | --- |
|  Tab / Down   |  move to the next field   |
|   Shift+Tab / Up  |  move to the previous field   |
|  Ctrl+s   |  save changes (asks for confirmation)   |
|   Esc  |   cancel editing (asks for confirmation if there are unsaved changes) |

---

**Notes View**
> [!info] 
> switch to Notes search mode by => `Alt + n`


|  key   | Description    |
| --- | --- |
|   Ctrl+s  |  save the note (asks for confirmation)   |
|   Esc  |   cancel (asks for confirmation if there are  unsaved changes)|

---

**Database List View**

> [!info] 
> switch to different database => `A`

|   key  | Description    |
| --- | --- |
|   Enter  |   switch to the database under cursor for this session ( ℹ️ when you open bmm it bydefault select the `bmm.db` database ) |
|  C   |  create a new database  and then "ctrl+s" to save |
|  1-9   |  jump to that numbered database and switch to it |
|  Alt + number   |  hold Alt and type a database number to jump to it (e.g. `Alt+1` `Alt+2` = 12); applies right away when no longer number is possible, otherwise after a short pause |
|  Alt + Super + 1 .. 5   |  order of the list: 1 your own order (default), 2 names A-Z, 3 names Z-A, 4 fewest links first, 5 most links first. Needs a terminal that reports the Super key (kitty, WezTerm, foot, Ghostty...) and a desktop that doesn't grab the shortcut |
|  v   |  same, but works everywhere: switch to the next order, going round the five |
|  a   |  arrange: Up/Down choose the new number, Enter/`a` save, Esc cancel (the database that had that number takes the moved one's old number) |
|  R   |  rename the database under cursor (`bmm.db`, the default, can't be renamed) |
|  F   |  folders view: group databases into folders (names can have emoji). `C` new folder, `R` rename, `D` delete (databases are kept), `Enter` open; inside a folder `Space` marks databases, `m` moves them to another folder, `Enter` switches to the database |
|  Ctrl + u / Ctrl + k   |  in any text field: delete everything left / right of the cursor |

> Each database shows `[ <count> ]` (its number of links), and the header shows the total across all databases.
|  Esc   |   go back  |


> [!info] Note
I didn't add a delete option for databases to avoid accidental deletion. But if you need to delete of those database, you can do it this way:
> ```bash
> #go to database location
> cd ~/.local/share/bmm
> 
> #run 'ls' commands to see the list of database
> ls
> 
> #to remove the database name like for example a dabase name 'example.db', to remove it, do this
> rm example.db
---






## Using a specific database from the command line

Every command works on `bmm.db` unless told otherwise. Use `--db <NAME>` to
run it on any of your databases, e.g. `bmm tags rename cyber cyber_security --db learning_linux`
(`bmm list --db work`, `bmm save <url> --db work`, ...). If a tag isn't found in the
current database, bmm tells you which other databases have it.


## Mouse / touch

In the database list and in the folders view (`F`), click (or tap) a row to jump to it - a database
switches to it, a folder opens - and the scroll wheel moves the cursor. To select text with the mouse
in your terminal, hold `Shift` while dragging.

## Version

`bmm -v` (or `bmm --version`) prints `bmm 3.0`.


## Tag suggestions

While typing tags (add/edit a link, tag search, tag rename) bmm suggests your existing tags. Besides
tags that start with what you typed, it also finds tags containing your letters in the same order,
starting from the first letter: with a tag `learn_networking`, typing `lea`, `leanet`, `lework` or
`learking` all suggest it. Tags that start with what you typed are listed first.

In the tags field of the add/edit screen, `Alt+z` lists *all* your tags as suggestions (the ones
matching what you typed come first); pick with Up/Down + Enter, and the list stays open so you can pick
several. Press `Alt+z` again (or `Esc`) to go back to only the matching ones. The tag search in the tags
list (`t`, then `/`) uses the same matching, so `leanet` finds `learn_networking` there too.
