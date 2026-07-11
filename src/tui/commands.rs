use crate::persistence::SearchTerms;

#[derive(Clone, Debug)]
pub(super) enum Command {
    OpenInBrowser(String),
    OpenMultipleInBrowser(Vec<String>),
    SearchBookmarks(SearchTerms),
    FetchAllBookmarks,
    FetchTags,
    FetchBookmarksForTag(String),
    FetchDuplicateBookmarks,
    DeleteBookmark(String),
    UpdateBookmark {
        uri: String,
        title: Option<String>,
        tags: Vec<String>,
    },
    CopyContentToClipboard(String),
}
