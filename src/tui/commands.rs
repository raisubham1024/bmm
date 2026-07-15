use crate::persistence::SearchTerms;

#[derive(Clone, Debug)]
pub(super) enum Command {
    OpenInBrowser(String),
    OpenInBrowserIncognito(String),
    OpenMultipleInBrowser(Vec<String>),
    OpenMultipleInBrowserIncognito(Vec<String>),
    SearchBookmarks(SearchTerms),
    FetchAllBookmarks,
    FetchTags,
    FetchBookmarksForTag(String),
    FetchDuplicateBookmarks,
    FetchStarredBookmarks,
    FetchStarredUris,
    ToggleStar(String),
    SwitchDatabase {
        path: String,
        display_name: String,
    },
    GlobalSearch(Option<SearchTerms>),
    DeleteBookmark(String, Option<String>),
    FetchNote(String),
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
    CopyContentToClipboard(String),
}
