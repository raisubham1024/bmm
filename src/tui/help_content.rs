//! Full, self-contained reference for bmm's TUI, in plain English - think
//! of it as this tool's "man page". Rendered by the Help view (`?`) as a
//! set of tables, one per heading: an overview of what bmm is and what it
//! can do, environment variables, every view it has, and every key/
//! shortcut for each view.
//!
//! To add or change a shortcut (or any other entry): find (or add) the
//! right [`HelpSection`] below and add a [`HelpRow`] to it. Keep
//! descriptions short, in simple English, and wrap long ones by hand with
//! `\n` (about 44 characters per line) so they read well in the table's
//! description column.

/// One row of a help table: a key (or view name) and what it does.
pub(super) struct HelpRow {
    pub(super) key: &'static str,
    pub(super) description: &'static str,
}

/// A named group of [`HelpRow`]s, rendered as its own table with a heading
/// above it.
pub(super) struct HelpSection {
    pub(super) title: &'static str,
    pub(super) rows: &'static [HelpRow],
}

macro_rules! row {
    ($key:expr, $desc:expr) => {
        HelpRow {
            key: $key,
            description: $desc,
        }
    };
}

pub(super) const HELP_SECTIONS: &[HelpSection] = &[
    HelpSection {
        title: "About bmm",
        rows: &[
            row!(
                "What it is",
                "bmm (\"bookmarks manager\") is a fast,\nkeyboard-driven tool for saving and\nfinding your bookmarks from the\nterminal."
            ),
            row!(
                "Where data lives",
                "Everything is stored locally in a\nSQLite database on your machine\n(default: <DATA_DIR>/bmm/bmm.db) -\nno account, no cloud, works offline."
            ),
            row!(
                "Two ways to use it",
                "1) As a command-line tool - \"bmm\nsave\", \"bmm search\", \"bmm list\", etc -\nfor scripting or wiring into other\ntools like fzf or Alfred.\n\n2) As this TUI (\"bmm tui\") - an\ninteractive, full-screen view for\nbrowsing, searching, and managing\nbookmarks with the keyboard.\nEvery action available in the TUI is\nalso available as a CLI command, and\nvice versa - the two are just\ndifferent front ends onto the same\ndatabase."
            ),
        ],
    },
    HelpSection {
        title: "Core Features",
        rows: &[
            row!(
                "search",
                "Search your bookmarks by title, URI,\nor tags as you type - see \"s\" under\nBookmarks List View below."
            ),
            row!(
                "Tags",
                "Organise bookmarks with comma-\nseparated tags; browse the Tags List\nView to see every tag and how many\nbookmarks use it."
            ),
            row!(
                "Notes",
                "Attach a private, freeform note to\nany bookmark (\"n\"), and optionally\nsearch only inside notes\n(Alt+n)."
            ),
            row!(
                "Starring",
                "Mark bookmarks as favourites with\n\"*\", then filter down to just your\nstarred bookmarks with \"S\"."
            ),
            row!(
                "Duplicate detection",
                "Find bookmarks that share the same\ntitle with \"d\", so you can clean\nthem up."
            ),
            row!(
                "Multiple databases",
                "Keep separate bookmark databases\n(e.g. work vs personal), switch\nbetween them, create new ones, and\nmove individual or marked bookmarks\nbetween them (\"A\", \"m\", \"M\")."
            ),
            row!(
                "Search across databases",
                "Run one search over every database\nat once with \"z\" - each result shows\nwhich database it came from."
            ),
            row!(
                "Importing",
                "Bring in existing bookmarks from a\nbrowser export or another tool.\nSupported file formats: HTML\n(Netscape-Bookmark-file-1, exported\nby Firefox/Chrome/etc), JSON, and\nplain TXT (one URI per line). Use\n\"bmm import <file>\" from the CLI\n(a --dry-run flag lets you preview\nfirst)."
            ),
            row!(
                "Checking for dead links",
                "Use \"bmm check\" from the CLI to test\nyour saved bookmarks and find broken\nor dead links."
            ),
            row!(
                "Opening links",
                "Open the bookmark under your cursor,\nor every bookmark currently listed,\nin your default browser - normally\nor in a private/incognito window\n(\"o\", \"O\", \"i\", \"I\")."
            ),
            row!(
                "Clipboard",
                "Copy a single link, or every listed\nlink, straight to your system\nclipboard (\"y\", \"Y\")."
            ),
        ],
    },
    HelpSection {
        title: "Environment Variables",
        rows: &[
            row!(
                "BMM_BROWSER",
                "The executable name (Linux/Windows)\nor app name (macOS) of the browser\nto use when opening links. Only\nneeded if bmm can't detect one on\nits own - it automatically looks for\nChrome, Chromium, Brave, Edge, or\nFirefox."
            ),
            row!(
                "BMM_BROWSER_INCOGNITO_FLAG",
                "The command-line flag that puts\nBMM_BROWSER into private/incognito\nmode. Only needed if your browser\nisn't Chromium- or Firefox-style\n(bmm defaults to \"--incognito\").\nOn Android, private opening only\nworks if Chrome is your installed\nbrowser."
            ),
            row!(
                "BMM_EDITOR / EDITOR",
                "The text editor bmm opens when you\nrun \"bmm save -e\" from the CLI to\nwrite a bookmark's title and tags by\nhand. BMM_EDITOR takes priority over\nEDITOR if both are set."
            ),
            row!(
                "XDG_DATA_HOME",
                "On Linux, overrides the folder bmm\nstores its database files in.\nYou can also override the exact\ndatabase file with the CLI's\n\"--db-path\" option."
            ),
        ],
    },
    HelpSection {
        title: "bmm has nine views",
        rows: &[
            row!(
                "Bookmarks List",
                "The main screen. Shows your saved\nbookmarks."
            ),
            row!(
                "Tags List",
                "Shows every tag you use, and how many\nbookmarks each one has."
            ),
            row!(
                "Edit Bookmark",
                "Lets you change a bookmark's title,\ntags, or link."
            ),
            row!(
                "Notes",
                "Lets you write or read a private note\nfor one bookmark."
            ),
            row!(
                "Database List",
                "Shows all your bookmark databases, so\nyou can switch between them."
            ),
            row!(
                "New Database Name",
                "Lets you type a name for a brand-new\ndatabase."
            ),
            row!(
                "Confirm",
                "A yes/no popup that checks you really\nwant to do something risky."
            ),
            row!(
                "Mode Switcher",
                "A quick menu that jumps straight to any\nother view."
            ),
            row!("Help", "This view - the list you're reading now."),
        ],
    },
    HelpSection {
        title: "General",
        rows: &[
            row!("?", "Open or close this Help view."),
            row!(
                "Alt+m",
                "Open the Mode Switcher, to jump to any\nview: all bookmarks, search, tags,\nduplicates, starred, search across all\ndatabases, note search, databases, or\nhelp. Works from anywhere, even while\nyou're typing. Press it again (or\nEsc/q) to close it."
            ),
            row!(
                "Alt+n",
                "Turn \"note search\" on or off - only\nshows bookmarks that have a note, and\nsearches inside the note text. Works\nfrom anywhere, even while you're\ntyping."
            ),
            row!(
                "Esc / q",
                "Go back a step, clear the current\ninput, or (from the main list) exit\nbmm."
            ),
            row!("j / Down", "Move down in a list."),
            row!("k / Up", "Move up in a list."),
            row!("g", "Jump to the first item in a list."),
            row!("G", "Jump to the last item in a list."),
        ],
    },
    HelpSection {
        title: "Bookmarks List View",
        rows: &[
            row!(
                "s",
                "Open the search box. Results update as\nyou type - no need to press Enter."
            ),
            row!("a", "Add a new bookmark (link, title, tags)."),
            row!(
                "t / Tab",
                "Open the Tags List View (only while\nyou are not already searching)."
            ),
            row!(
                "d",
                "Show only bookmarks that share a title\nwith another one (duplicates)."
            ),
            row!(
                "e",
                "Edit the title and tags of the\nbookmark under your cursor."
            ),
            row!(
                "E",
                "Edit the link (and title/tags too) of\nthe bookmark under your cursor."
            ),
            row!(
                "n",
                "Add or edit a hidden note for the\nbookmark under your cursor."
            ),
            row!(
                "N",
                "Delete the note on the bookmark under\nyour cursor, if it has one (asks\nfirst)."
            ),
            row!("*", "Star or unstar the bookmark under your\ncursor."),
            row!("S", "Show only your starred bookmarks."),
            row!(
                "A",
                "Open the Database List View, to see or\nswitch databases."
            ),
            row!(
                "z",
                "Search every database at once. Each\nresult shows which database it's\nfrom."
            ),
            row!(
                "Space",
                "Mark or unmark the bookmark under your\ncursor, to move it later (see \"M\")."
            ),
            row!(
                "m",
                "Move the bookmark under your cursor to\nanother database (asks first)."
            ),
            row!(
                "M",
                "Move every marked bookmark to another\ndatabase (asks first) - mark them\nfirst with Space."
            ),
            row!(
                "Delete / D",
                "Delete the bookmark under your cursor\n(asks first)."
            ),
            row!("o", "Open the link under your cursor in\nyour browser."),
            row!(
                "i",
                "Open the link under your cursor in a\nprivate/incognito window (see\n\"Environment Variables\" above if bmm\npicks the wrong browser)."
            ),
            row!(
                "O",
                "Open every listed bookmark in your\nbrowser (up to 30 at once - warns you\nif there are more)."
            ),
            row!(
                "I",
                "Same as \"O\", but opens them privately/\nincognito (up to 30 at once)."
            ),
            row!("y", "Copy the link under your cursor to the\nclipboard."),
            row!("Y", "Copy every listed link to the\nclipboard."),
        ],
    },
    HelpSection {
        title: "Search Input (opened with \"s\")",
        rows: &[
            row!(
                "Enter",
                "Lock in the search text and close the\nbox (leave it empty + Enter to see\nevery bookmark)."
            ),
            row!("Esc", "Cancel and close the search box."),
            row!(
                "Down / Up",
                "Move the highlighted bookmark while\nyou keep typing."
            ),
        ],
    },
    HelpSection {
        title: "Tags List View",
        rows: &[
            row!("/", "Open the tag search box, to filter the\ntag list."),
            row!(
                "Enter",
                "Show every bookmark that has the tag\nunder your cursor."
            ),
        ],
    },
    HelpSection {
        title: "Tag Search Input (opened with \"/\")",
        rows: &[
            row!(
                "Enter",
                "Lock in the filter and go back to the\ntag list."
            ),
            row!("Esc", "Cancel and go back to the tag list."),
            row!(
                "Down / Up",
                "Move the highlighted tag while you\nkeep typing."
            ),
        ],
    },
    HelpSection {
        title: "Edit Bookmark View",
        rows: &[
            row!("Tab / Down", "Move to the next field."),
            row!("Shift+Tab / Up", "Move to the previous field."),
            row!("Ctrl+s", "Save your changes (asks first)."),
            row!(
                "Esc",
                "Cancel editing (asks first if you\nchanged something)."
            ),
        ],
    },
    HelpSection {
        title: "Notes View",
        rows: &[
            row!("Ctrl+s", "Save the note (asks first)."),
            row!(
                "Esc",
                "Cancel (asks first if you changed\nsomething)."
            ),
        ],
    },
    HelpSection {
        title: "Database List View",
        rows: &[
            row!(
                "Enter",
                "Switch to the database under your\ncursor for this session (goes back to\nbmm.db next time you start bmm) - or,\nif you got here via \"m\"/\"M\", move the\nqueued bookmark(s) into it instead."
            ),
            row!(
                "/",
                "Open the database search box, to\nfilter the list by name."
            ),
            row!(
                "C",
                "Create a new database (only shown\nwhile switching databases, not while\nmoving bookmarks)."
            ),
            row!("Esc", "Go back."),
        ],
    },
    HelpSection {
        title: "Database Search Input (opened with \"/\")",
        rows: &[
            row!(
                "Enter",
                "Lock in the filter and go back to the\ndatabase list."
            ),
            row!(
                "Esc",
                "Cancel and go back to the bookmarks\nlist."
            ),
            row!(
                "Down / Up",
                "Move the highlighted database while\nyou keep typing."
            ),
        ],
    },
    HelpSection {
        title: "New Database Name View",
        rows: &[
            row!(
                "Enter / Ctrl+s",
                "Create the database and switch to it."
            ),
            row!("Esc", "Cancel."),
        ],
    },
    HelpSection {
        title: "Mode Switcher View (opened with Alt+m)",
        rows: &[
            row!(
                "j / k / Down / Up",
                "Move the highlighted option."
            ),
            row!("g / G", "Jump to the first / last option."),
            row!("Enter", "Jump to the view you picked."),
            row!("Alt+m / Esc / q", "Close without switching."),
        ],
    },
    HelpSection {
        title: "Confirm View",
        rows: &[
            row!("y", "Confirm (\"yes\")."),
            row!("n / Esc", "Cancel (\"no\")."),
        ],
    },
    HelpSection {
        title: "Help View (this one)",
        rows: &[
            row!("j / k / Down / Up", "Scroll the list."),
            row!("g / G", "Jump to the top / bottom."),
            row!("Esc / q / ?", "Close and go back."),
        ],
    },
    HelpSection {
        title: "See Also",
        rows: &[
            row!(
                "Full CLI reference",
                "This view only covers the TUI. For\nevery command-line command and\noption (save, save-all, list,\nsearch, show, delete, import, check,\ntags, notes, ...), run \"bmm --help\",\nor \"bmm <command> --help\" for one\ncommand (e.g. \"bmm save --help\")."
            ),
            row!(
                "Source & issues",
                "https://github.com/raisubham1024/bmm - browse\nthe code, report a bug, or request a\nfeature."
            ),
        ],
    },
];
