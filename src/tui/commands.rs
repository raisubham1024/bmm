use crate::persistence::{SearchScope, SearchTerms};

#[derive(Clone, Debug)]
pub(super) enum Command {
    OpenInBrowser(String),
    OpenInBrowserIncognito(String),
    OpenMultipleInBrowser(Vec<String>),
    OpenMultipleInBrowserIncognito(Vec<String>),
    /// `scope` narrows the search down to just URLs, just descriptions
    /// (titles), or just tags - `SearchScope::All` (the default) keeps
    /// bmm's original "match any of them" behavior. Picked via the Alt+s
    /// search-scope popup.
    SearchBookmarks(SearchTerms, SearchScope),
    FetchAllBookmarks,
    FetchTags,
    FetchBookmarksForTag(String),
    /// Renames a tag - updating every bookmark that used it - either in
    /// just the active database, or (`global: true`, when triggered from
    /// the "all databases" Tags List view) across every local database at
    /// once. If `new_name` already exists as a tag, the two are merged
    /// rather than ending up with a duplicate.
    RenameTag {
        old_name: String,
        new_name: String,
        global: bool,
    },
    /// Fetches tag stats aggregated across every local database, for the
    /// "all databases" Tags List view (`T`).
    FetchGlobalTags,
    /// Fetches every bookmark tagged with the given tag across every local
    /// database - the "all databases" counterpart to `FetchBookmarksForTag`,
    /// used when a tag is selected from the `T` (global) Tags List view.
    FetchBookmarksForTagAcrossDatabases(String),
    FetchStarredBookmarks,
    FetchStarredUris,
    ToggleStar(String),
    SwitchDatabase {
        path: String,
        display_name: String,
    },
    /// Same `scope` idea as `SearchBookmarks`, applied to every local
    /// database at once.
    GlobalSearch(Option<SearchTerms>, SearchScope),
    SearchNotes(Option<SearchTerms>),
    DeleteBookmark(String, Option<String>),
    /// Deletes every bookmark in `items` - (uri, source database path,
    /// `None` meaning the currently active database). Used for "delete all
    /// currently listed bookmarks" (`D`).
    DeleteBookmarks(Vec<(String, Option<String>)>),
    FetchNote(String),
    FetchNoteExists(String),
    SaveNote {
        uri: String,
        note: Option<String>,
    },
    UpdateBookmark {
        uri: String,
        new_uri: Option<String>,
        title: Option<String>,
        tags: Vec<String>,
        is_new: bool,
        target_db_path: Option<String>,
    },
    MoveBookmarks {
        /// (uri, source database path - `None` means the currently active database)
        items: Vec<(String, Option<String>)>,
        target_db_path: String,
        target_display_name: String,
    },
    CopyContentToClipboard(String),
    BackupDatabases,
    RestoreDatabases,
    CheckForUpdate,
    /// Fetches `uri`'s page metadata (Alt+F, from the Title field of the
    /// add/edit bookmark screen) so its description can be auto-filled
    /// into the Title field.
    FetchDescription(String),
}
