use super::common::{ActivePane, DbListPurpose, DbView, EditField, FolderListPurpose};
use super::model::{Model, ModeOption, SearchScopeOption};
use crate::domain::{SavedBookmark, TagStats};
use crate::persistence::DBError;
use ratatui::crossterm::event::{
    Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;
use std::io::Error as IOError;

pub enum Message {
    TerminalResize(u16, u16),
    GoToNextListItem,
    GoToPreviousListItem,
    GoToFirstListItem,
    GoToLastListItem,
    OpenInBrowser,
    OpenInBrowserIncognito,
    RequestOpenAllInBrowser,
    RequestOpenAllInBrowserIncognito,
    StartAddBookmark,
    UrlsOpenedInBrowser(UrlsOpenedResult),
    SearchFinished(Result<Vec<SavedBookmark>, DBError>),
    AllBookmarksFetched(Result<Vec<SavedBookmark>, DBError>),
    TagsFetched(Result<Vec<TagStats>, DBError>),
    ShowView(ActivePane),
    SearchInputGotEvent(Event),
    TagSearchInputGotEvent(Event),
    SubmitTagSearch,
    CancelTagSearch,
    CopyURIToClipboard,
    CopyURIsToClipboard,
    SubmitSearch,
    ShowBookmarksForTag,
    BookmarksForTagFetched(Result<Vec<SavedBookmark>, DBError>),
    /// Switches the Tags List view into "all databases" mode (`T`) and
    /// kicks off fetching tag stats aggregated across every local database.
    ShowGlobalTagsList,
    GlobalTagsFetched(Vec<TagStats>, Vec<String>),
    /// Tag names from every local database - feeds the tag suggestions in
    /// the add/edit bookmark screen (see `Command::FetchSuggestionTags`).
    SuggestionTagsFetched(Vec<String>),
    /// Fired when a tag is selected from the Tags List view while it's in
    /// "all databases" mode - the cross-database counterpart to
    /// `BookmarksForTagFetched`. Carries (database display name, database
    /// path, bookmark) triples, same shape as `GlobalSearchFinished`, plus
    /// any per-database errors.
    BookmarksForTagAcrossDatabasesFetched(Vec<(String, String, SavedBookmark)>, Vec<String>),
    /// Opens the rename-tag screen (Alt+e) for whichever tag is currently
    /// highlighted in the Tags List / Tag Search views.
    StartRenameTag,
    RenameTagInputGotEvent(Event),
    RenameTagSuggestionNext,
    RenameTagSuggestionPrev,
    AcceptRenameTagSuggestion,
    DismissRenameTagSuggestions,
    RequestSaveRenameTag,
    RequestExitRenameTag,
    /// (old name, new name, bookmarks affected).
    TagRenamed(String, String, Result<u64, String>),
    ShowStarred,
    StarredBookmarksFetched(Result<Vec<SavedBookmark>, DBError>),
    StarredUrisFetched(Result<HashSet<String>, DBError>),
    RequestToggleStar,
    StarToggled(String, Result<bool, String>),
    ShowDatabaseList,
    RequestSwitchDatabase,
    /// Jumps straight to and switches to (or picks, when moving bookmarks)
    /// the database at this 0-based index into `filtered_dbs` - fired by
    /// pressing its number key (1-9) in the database list, same idea as
    /// `SelectModeByNumber`.
    SelectDatabaseByNumber(usize),
    /// A digit typed with Alt held down in the Database List view - used
    /// to jump to databases numbered 10 and above (typing several digits
    /// builds up the number, eg. Alt+1 then Alt+2 for 12).
    AltDigitInDbList(char),
    /// Fired by the event loop shortly after the last Alt+digit, which is
    /// the point the typed number gets applied (terminals don't report
    /// the Alt key being released, so a short pause stands in for it).
    AltDigitTimeout,
    /// Counts of bookmarks per database (name, count - `None` if that
    /// database couldn't be read).
    DatabaseCountsFetched(Vec<(String, Option<i64>)>),
    /// Alt+B in a list of bookmarks: check which of the shown links are
    /// broken.
    StartLinkCheck,
    LinkCheckProgress {
        id: u64,
        done: usize,
    },
    LinkCheckFinished(LinkCheckOutcome),
    /// A row of the database / folder list was clicked (or tapped).
    ClickListRow(usize),
    /// Alt+Super+1..5 / `v` in the Database List view.
    SetDbView(DbView),
    CycleDbView,
    /// Opens the folder list (`F`).
    ShowFolders,
    /// Enter in the folder list: open the folder, or (when picking a
    /// destination) move the waiting databases into it.
    EnterSelectedFolder,
    SelectFolderByNumber(usize),
    StartCreateFolder,
    StartRenameFolder,
    DeleteSelectedFolder,
    /// Space in a folder: mark/unmark the database under cursor.
    ToggleMarkDatabase,
    /// `m` in a folder: choose the folder to move database(s) to.
    StartMoveDatabases,
    /// Enter in a folder: switch to the database under cursor.
    SwitchToFolderDatabase,
    SelectFolderDatabaseByNumber(usize),
    StartArrangeDatabases,
    ConfirmArrangeDatabases,
    CancelArrangeDatabases,
    StartRenameDatabase,
    DatabaseRenamed(DatabaseRenameOutcome),
    StartNewDatabaseName,
    NewDatabaseNameGotEvent(Event),
    RequestCreateDatabase,
    DatabaseSwitched(Result<(Pool<Sqlite>, String), String>),
    DatabaseSearchInputGotEvent(Event),
    SubmitDatabaseSearch,
    CancelDatabaseSearch,
    ToggleNoteSearch,
    RequestMoveSelectedBookmark,
    RequestMoveMarkedBookmarks,
    ToggleMarkForMove,
    BookmarksMoved(Result<(usize, String), String>),
    ShowGlobalSearch,
    GlobalSearchFinished(Vec<(String, String, SavedBookmark)>, Vec<String>),
    /// Opens the search-scope popup (Alt+s), letting the user restrict
    /// the search (plain "s" or cross-database "z") to just URLs, just
    /// descriptions (titles), or just tags.
    ShowSearchScopePicker,
    /// Applies whichever scope is highlighted in the popup and drops
    /// back into the search box.
    ConfirmSearchScopeSelection,
    /// Jumps straight to and picks the scope option at this 0-based
    /// index into `SearchScopeOption::ALL` - fired by pressing its
    /// number key, same idea as `SelectModeByNumber`.
    SelectSearchScopeByNumber(usize),
    /// Moves the highlighted suggestion in the search box's tag-name
    /// suggestions popup (only shown while `search_scope` is `Tag`) -
    /// the search-box counterpart to `TagSuggestionNext`.
    SearchTagSuggestionNext,
    SearchTagSuggestionPrev,
    AcceptSearchTagSuggestion,
    DismissSearchTagSuggestions,
    RequestDeleteBookmark,
    BookmarkDeleted(String, Result<u64, DBError>),
    /// Asks to delete every bookmark currently listed (`D`) - shows a
    /// confirmation naming exactly how many links will be deleted before
    /// anything happens.
    RequestDeleteAllVisible,
    BookmarksDeleted(Result<u64, String>),
    StartEditBookmark(bool),
    EditFieldGotEvent(Event),
    EditFieldNext,
    EditFieldPrev,
    /// Alt+z in the tags field.
    ToggleAllTagSuggestions,
    /// Alt+t in the tags field: include tags from every database.
    ToggleOtherDbTagSuggestions,
    TagSuggestionNext,
    TagSuggestionPrev,
    AcceptTagSuggestion,
    DismissTagSuggestions,
    RequestSaveBookmarkEdit,
    RequestExitEdit,
    BookmarkUpdated(Result<(), String>),
    StartNoteEdit,
    RequestDeleteNote,
    NoteFetched(String, Result<Option<String>, DBError>),
    NoteExistenceFetched(String, Result<bool, DBError>),
    NoteInputGotEvent(Event),
    RequestSaveNote,
    RequestExitNote,
    NoteSaved(Result<(), String>),
    ConfirmYes,
    ConfirmNo,
    ContentCopiedToClipboard(Result<(), String>),
    GoBackOrQuit,
    ToggleModeSwitcher,
    ConfirmModeSelection,
    /// Jumps straight to and opens the mode at this 0-based index in
    /// `ModeOption::ALL` - fired by pressing its number key (1-9) in the
    /// mode switcher, so the user doesn't have to move the selection down
    /// to it first and then press Enter.
    SelectModeByNumber(usize),
    ShowAllBookmarks,
    RequestBackupDatabases,
    DatabasesBackedUp(Result<(usize, std::path::PathBuf), String>),
    RequestRestoreDatabases,
    DatabasesRestored(Result<(usize, std::path::PathBuf), String>),
    RequestCheckForUpdate,
    UpdateCheckFinished(Result<crate::self_update::UpdateOutcome, String>),
    /// Fired by Alt+F while the Title field is focused in the add/edit
    /// bookmark screen - fetches the URL's page and auto-fills the Title
    /// field with its description.
    RequestFetchDescription,
    /// `Ok(Some(description))` fills the Title field; `Ok(None)` means the
    /// page was reachable but had no description to use, in which case
    /// nothing happens.
    DescriptionFetched(Result<Option<String>, String>),
}

/// Result of a `Command::RenameDatabase`. `pool` is only set when the
/// renamed database was the active one: it's the freshly opened pool on the
/// new file if the rename worked, or a re-opened pool on the old file if it
/// didn't - either way, the model must start using it, since the old pool
/// was closed.
pub struct DatabaseRenameOutcome {
    pub old_name: String,
    pub old_path: String,
    pub new_name: String,
    pub new_path: String,
    pub error: Option<String>,
    pub pool: Option<Pool<Sqlite>>,
}

/// Result of a `Command::CheckLinks`.
pub struct LinkCheckOutcome {
    pub id: u64,
    pub checked: usize,
    /// (uri, why it's broken) for every link that can't be opened.
    pub broken: Vec<(String, String)>,
    pub cancelled: bool,
}

pub enum UrlsOpenedResult {
    Success,
    Failure(IOError),
}

pub fn get_event_handling_msg(model: &Model, event: Event) -> Option<Message> {
    match event {
        Event::Key(key_event) => match model.terminal_too_small {
            true => match key_event.kind {
                KeyEventKind::Press => match key_event.code {
                    KeyCode::Esc | KeyCode::Char('q') => Some(Message::GoBackOrQuit),
                    _ => None,
                },
                _ => None,
            },
            false => match key_event.kind {
                KeyEventKind::Press => {
                    // Alt+m opens/closes the mode switcher from anywhere,
                    // regardless of which pane is currently active - this
                    // has to be checked before the per-pane match below, or
                    // panes that treat any unmatched key as text input
                    // (search, edit, notes, ...) would swallow it as a
                    // literal 'm' instead.
                    if key_event.modifiers.contains(KeyModifiers::ALT)
                        && key_event.code == KeyCode::Char('m')
                    {
                        return if model.active_pane == ActivePane::ModeSwitcher {
                            Some(Message::GoBackOrQuit)
                        } else {
                            Some(Message::ToggleModeSwitcher)
                        };
                    }

                    // Alt+n toggles "note search mode" from anywhere, same
                    // as Alt+m above - plain 'n' (no Alt) still means "add/
                    // edit note for the bookmark under cursor" in the List
                    // view, so this has to be checked first.
                    if key_event.modifiers.contains(KeyModifiers::ALT)
                        && key_event.code == KeyCode::Char('n')
                    {
                        return Some(Message::ToggleNoteSearch);
                    }

                    // Alt+g restores every database from the platform's
                    // backup/"links" folder into bmm's data directory,
                    // same as Alt+m/Alt+n above - works from anywhere,
                    // even while typing, since it's an Alt combo rather
                    // than a plain letter.
                    if key_event.modifiers.contains(KeyModifiers::ALT)
                        && key_event.code == KeyCode::Char('g')
                    {
                        return Some(Message::RequestRestoreDatabases);
                    }

                    // Alt+b backs up every local database to that same
                    // backup/"links" folder - the reverse of Alt+g above,
                    // same reasoning: works from anywhere, even while
                    // typing.
                    if key_event.modifiers.contains(KeyModifiers::ALT)
                        && key_event.code == KeyCode::Char('b')
                    {
                        return Some(Message::RequestBackupDatabases);
                    }

                    // Alt+B (capital B) checks which of the links shown in
                    // the list are broken. Alt+b (lowercase) is still the
                    // backup shortcut above.
                    if key_event.modifiers.contains(KeyModifiers::ALT)
                        && key_event.code == KeyCode::Char('B')
                        && model.active_pane == ActivePane::List
                    {
                        return Some(Message::StartLinkCheck);
                    }

                    // Alt+u checks for (and installs, if available) a
                    // newer bmm binary at every location bmm is found on
                    // PATH, same reasoning as Alt+m/Alt+n/Alt+g/Alt+b
                    // above: works from anywhere, even while typing.
                    if key_event.modifiers.contains(KeyModifiers::ALT)
                        && key_event.code == KeyCode::Char('u')
                    {
                        return Some(Message::RequestCheckForUpdate);
                    }

                    // Alt+<digit> in the Database List view builds up a
                    // database number (Alt+1 then Alt+2 = 12) and jumps to
                    // that database - this is how databases numbered 10
                    // and above are reached, since plain digit keys only
                    // cover 1-9. Works from wherever the cursor happens
                    // to be, and not while a database is being arranged.
                    // Alt+Super+1..5 pick how the databases are ordered
                    // (needs a terminal that reports the Super key, eg.
                    // kitty, WezTerm, foot, Ghostty - `v` does the same
                    // everywhere, one view at a time).
                    if key_event
                        .modifiers
                        .contains(KeyModifiers::ALT | KeyModifiers::SUPER)
                        && model.active_pane == ActivePane::DatabaseList
                        && model.db_arrange_origin.is_none()
                        && let KeyCode::Char(c) = key_event.code
                        && let Some(view) = DbView::from_digit(c)
                    {
                        return Some(Message::SetDbView(view));
                    }

                    if key_event.modifiers.contains(KeyModifiers::ALT)
                        && !key_event.modifiers.contains(KeyModifiers::SUPER)
                        && model.active_pane == ActivePane::DatabaseList
                        && model.db_arrange_origin.is_none()
                        && let KeyCode::Char(c) = key_event.code
                        && c.is_ascii_digit()
                    {
                        return Some(Message::AltDigitInDbList(c));
                    }

                    // Alt+s opens the search-scope popup, letting the
                    // current (or about-to-start) search be narrowed down
                    // to just URLs, just descriptions (titles), or just
                    // tags - works for a plain search ("s") as well as
                    // the cross-database search ("z"), from the List view
                    // (before typing anything) or from the search box
                    // itself (while typing). Doesn't apply to note search
                    // (Alt+n), so it's guarded by pane/mode rather than
                    // being truly global like the combos above.
                    if key_event.modifiers.contains(KeyModifiers::ALT)
                        && key_event.code == KeyCode::Char('s')
                        && !model.note_search_mode
                        && matches!(model.active_pane, ActivePane::List | ActivePane::SearchInput)
                    {
                        return Some(Message::ShowSearchScopePicker);
                    }

                    match model.active_pane {
                    ActivePane::List => match key_event.code {
                        KeyCode::Char('j') | KeyCode::Down => Some(Message::GoToNextListItem),
                        KeyCode::Char('k') | KeyCode::Up => Some(Message::GoToPreviousListItem),
                        KeyCode::Char('g') => Some(Message::GoToFirstListItem),
                        KeyCode::Char('G') => Some(Message::GoToLastListItem),
                        KeyCode::Char('o') => Some(Message::OpenInBrowser),
                        KeyCode::Char('i') => Some(Message::OpenInBrowserIncognito),
                        KeyCode::Char('O') => Some(Message::RequestOpenAllInBrowser),
                        KeyCode::Char('I') => Some(Message::RequestOpenAllInBrowserIncognito),
                        KeyCode::Char('s') => Some(Message::ShowView(ActivePane::SearchInput)),
                        // Shows all bookmarks of the current database -
                        // same target as the mode switcher's 1st option
                        // ("all bookmarks of current database [l]"), and
                        // as pressing Enter on a blank query in search mode.
                        KeyCode::Char('l') => Some(Message::ShowAllBookmarks),
                        KeyCode::Char('a') => Some(Message::StartAddBookmark),
                        KeyCode::Char('t') | KeyCode::Tab => {
                            Some(Message::ShowView(ActivePane::TagsList))
                        }
                        KeyCode::Char('T') => Some(Message::ShowGlobalTagsList),
                        KeyCode::Char('d') => Some(Message::RequestDeleteBookmark),
                        KeyCode::Char('D') => Some(Message::RequestDeleteAllVisible),
                        KeyCode::Char('S') => Some(Message::ShowStarred),
                        KeyCode::Char('*') => Some(Message::RequestToggleStar),
                        KeyCode::Char('A') => Some(Message::ShowDatabaseList),
                        KeyCode::Char('F') => Some(Message::ShowFolders),
                        KeyCode::Char('z') => Some(Message::ShowGlobalSearch),
                        KeyCode::Char('e') => Some(Message::StartEditBookmark(false)),
                        KeyCode::Char('E') => Some(Message::StartEditBookmark(true)),
                        KeyCode::Char('n') => Some(Message::StartNoteEdit),
                        KeyCode::Char('N') => Some(Message::RequestDeleteNote),
                        KeyCode::Char(' ') => Some(Message::ToggleMarkForMove),
                        KeyCode::Char('m') => Some(Message::RequestMoveSelectedBookmark),
                        KeyCode::Char('M') => Some(Message::RequestMoveMarkedBookmarks),
                        KeyCode::Delete => Some(Message::RequestDeleteBookmark),
                        KeyCode::Char('y')
                            if key_event.modifiers.contains(KeyModifiers::SHIFT) =>
                        {
                            Some(Message::CopyURIsToClipboard)
                        }
                        KeyCode::Char('y') => Some(Message::CopyURIToClipboard),
                        KeyCode::Char('Y') => Some(Message::CopyURIsToClipboard),
                        KeyCode::Esc | KeyCode::Char('q') => Some(Message::GoBackOrQuit),
                        KeyCode::Char('?') => Some(Message::ShowView(ActivePane::Help)),
                        _ => None,
                    },
                    ActivePane::Help => match key_event.code {
                        KeyCode::Char('j') | KeyCode::Down => Some(Message::GoToNextListItem),
                        KeyCode::Char('k') | KeyCode::Up => Some(Message::GoToPreviousListItem),
                        KeyCode::Char('g') => Some(Message::GoToFirstListItem),
                        KeyCode::Char('G') => Some(Message::GoToLastListItem),
                        KeyCode::Esc | KeyCode::Char('q') => Some(Message::GoBackOrQuit),
                        KeyCode::Char('?') => Some(Message::ShowView(ActivePane::List)),
                        _ => None,
                    },
                    ActivePane::SearchInput => match key_event.code {
                        KeyCode::Down if !model.search_tag_suggestions.is_empty() => {
                            Some(Message::SearchTagSuggestionNext)
                        }
                        KeyCode::Up if !model.search_tag_suggestions.is_empty() => {
                            Some(Message::SearchTagSuggestionPrev)
                        }
                        KeyCode::Enter if !model.search_tag_suggestions.is_empty() => {
                            Some(Message::AcceptSearchTagSuggestion)
                        }
                        KeyCode::Esc if !model.search_tag_suggestions.is_empty() => {
                            Some(Message::DismissSearchTagSuggestions)
                        }
                        KeyCode::Esc => Some(Message::GoBackOrQuit),
                        KeyCode::Enter => Some(Message::SubmitSearch),
                        KeyCode::Down => Some(Message::GoToNextListItem),
                        KeyCode::Up => Some(Message::GoToPreviousListItem),
                        _ => Some(Message::SearchInputGotEvent(event)),
                    },
                    ActivePane::TagsList => match key_event.code {
                        KeyCode::Char('j') | KeyCode::Down => Some(Message::GoToNextListItem),
                        KeyCode::Char('k') | KeyCode::Up => Some(Message::GoToPreviousListItem),
                        KeyCode::Char('g') => Some(Message::GoToFirstListItem),
                        KeyCode::Char('G') => Some(Message::GoToLastListItem),
                        KeyCode::Char('/') => Some(Message::ShowView(ActivePane::TagSearchInput)),
                        KeyCode::Char('e')
                            if key_event.modifiers.contains(KeyModifiers::ALT) =>
                        {
                            Some(Message::StartRenameTag)
                        }
                        KeyCode::Enter => Some(Message::ShowBookmarksForTag),
                        KeyCode::Esc | KeyCode::Char('q') => Some(Message::GoBackOrQuit),
                        _ => None,
                    },
                    ActivePane::TagSearchInput => match key_event.code {
                        KeyCode::Esc => Some(Message::CancelTagSearch),
                        KeyCode::Enter => Some(Message::SubmitTagSearch),
                        KeyCode::Down => Some(Message::GoToNextListItem),
                        KeyCode::Up => Some(Message::GoToPreviousListItem),
                        KeyCode::Char('e')
                            if key_event.modifiers.contains(KeyModifiers::ALT) =>
                        {
                            Some(Message::StartRenameTag)
                        }
                        _ => Some(Message::TagSearchInputGotEvent(event)),
                    },
                    ActivePane::RenameTag => match key_event.code {
                        // While the suggestion list is showing, Up/Down/
                        // Enter/Esc control it instead of their usual
                        // text-input/exit behavior - checked first, so
                        // they only take over when there's actually a
                        // suggestion list to control (same pattern as the
                        // Tags field in the Edit Bookmark screen).
                        KeyCode::Down if !model.rename_tag_suggestions.is_empty() => {
                            Some(Message::RenameTagSuggestionNext)
                        }
                        KeyCode::Up if !model.rename_tag_suggestions.is_empty() => {
                            Some(Message::RenameTagSuggestionPrev)
                        }
                        KeyCode::Enter if !model.rename_tag_suggestions.is_empty() => {
                            Some(Message::AcceptRenameTagSuggestion)
                        }
                        KeyCode::Esc if !model.rename_tag_suggestions.is_empty() => {
                            Some(Message::DismissRenameTagSuggestions)
                        }
                        KeyCode::Esc => Some(Message::RequestExitRenameTag),
                        KeyCode::Char('s')
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            Some(Message::RequestSaveRenameTag)
                        }
                        _ => Some(Message::RenameTagInputGotEvent(event)),
                    },
                    ActivePane::EditBookmark => match key_event.code {
                        // Alt+z in the tags field: every existing tag as a
                        // suggestion; pressed again, back to matching ones
                        KeyCode::Char('z')
                            if key_event.modifiers.contains(KeyModifiers::ALT)
                                && model.edit_focus == EditField::Tags =>
                        {
                            Some(Message::ToggleAllTagSuggestions)
                        }
                        // Alt+t in the tags field: also suggest tags from
                        // every other database; again, back to just this one
                        KeyCode::Char('t')
                            if key_event.modifiers.contains(KeyModifiers::ALT)
                                && model.edit_focus == EditField::Tags =>
                        {
                            Some(Message::ToggleOtherDbTagSuggestions)
                        }
                        // While the tags field has live suggestions
                        // showing, Up/Down/Enter/Esc control the
                        // suggestion list instead of their usual
                        // field-switching/exit behavior - checked first,
                        // so they only take over when there's actually a
                        // suggestion list to control.
                        KeyCode::Down
                            if model.edit_focus == EditField::Tags
                                && !model.tag_suggestions.is_empty() =>
                        {
                            Some(Message::TagSuggestionNext)
                        }
                        KeyCode::Up
                            if model.edit_focus == EditField::Tags
                                && !model.tag_suggestions.is_empty() =>
                        {
                            Some(Message::TagSuggestionPrev)
                        }
                        KeyCode::Enter
                            if model.edit_focus == EditField::Tags
                                && !model.tag_suggestions.is_empty() =>
                        {
                            Some(Message::AcceptTagSuggestion)
                        }
                        KeyCode::Esc
                            if model.edit_focus == EditField::Tags
                                && !model.tag_suggestions.is_empty() =>
                        {
                            Some(Message::DismissTagSuggestions)
                        }
                        KeyCode::Esc => Some(Message::RequestExitEdit),
                        KeyCode::Tab | KeyCode::Down => Some(Message::EditFieldNext),
                        KeyCode::BackTab | KeyCode::Up => Some(Message::EditFieldPrev),
                        KeyCode::Char('s')
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            Some(Message::RequestSaveBookmarkEdit)
                        }
                        // Alt+F, while the Title field is focused, fetches
                        // the bookmark's URL and auto-fills the Title
                        // field with the page's description, if it has one
                        // - the TUI counterpart to `bmm fetch <URI>`. If
                        // there's no description, nothing happens.
                        KeyCode::Char('f') | KeyCode::Char('F')
                            if key_event.modifiers.contains(KeyModifiers::ALT)
                                && model.edit_focus == EditField::Title =>
                        {
                            Some(Message::RequestFetchDescription)
                        }
                        _ => Some(Message::EditFieldGotEvent(event)),
                    },
                    ActivePane::Notes => match key_event.code {
                        KeyCode::Esc => Some(Message::RequestExitNote),
                        KeyCode::Char('s')
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            Some(Message::RequestSaveNote)
                        }
                        _ => Some(Message::NoteInputGotEvent(event)),
                    },
                    // While a database is being arranged (`a`), Up/Down
                    // choose the number to move it to, Enter/`a` saves
                    // and Esc cancels - nothing else is active.
                    ActivePane::FolderList => match key_event.code {
                        KeyCode::Char('j') | KeyCode::Down => Some(Message::GoToNextListItem),
                        KeyCode::Char('k') | KeyCode::Up => Some(Message::GoToPreviousListItem),
                        KeyCode::Char('g') => Some(Message::GoToFirstListItem),
                        KeyCode::Char('G') => Some(Message::GoToLastListItem),
                        KeyCode::Enter => Some(Message::EnterSelectedFolder),
                        KeyCode::Char('C') => Some(Message::StartCreateFolder),
                        KeyCode::Char('R')
                            if model.folder_list_purpose == FolderListPurpose::Browse =>
                        {
                            Some(Message::StartRenameFolder)
                        }
                        KeyCode::Char('D')
                            if model.folder_list_purpose == FolderListPurpose::Browse =>
                        {
                            Some(Message::DeleteSelectedFolder)
                        }
                        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                            let index = (c as usize) - ('1' as usize);
                            if index < model.folder_rows().len() {
                                Some(Message::SelectFolderByNumber(index))
                            } else {
                                None
                            }
                        }
                        KeyCode::Esc | KeyCode::Char('q') => Some(Message::GoBackOrQuit),
                        _ => None,
                    },
                    ActivePane::FolderContents => match key_event.code {
                        KeyCode::Char('j') | KeyCode::Down => Some(Message::GoToNextListItem),
                        KeyCode::Char('k') | KeyCode::Up => Some(Message::GoToPreviousListItem),
                        KeyCode::Char('g') => Some(Message::GoToFirstListItem),
                        KeyCode::Char('G') => Some(Message::GoToLastListItem),
                        KeyCode::Enter => Some(Message::SwitchToFolderDatabase),
                        KeyCode::Char(' ') => Some(Message::ToggleMarkDatabase),
                        KeyCode::Char('m') => Some(Message::StartMoveDatabases),
                        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                            let index = (c as usize) - ('1' as usize);
                            if index < model.folder_dbs.len() {
                                Some(Message::SelectFolderDatabaseByNumber(index))
                            } else {
                                None
                            }
                        }
                        KeyCode::Esc | KeyCode::Char('q') => Some(Message::GoBackOrQuit),
                        _ => None,
                    },
                    ActivePane::DatabaseList if model.db_arrange_origin.is_some() => {
                        match key_event.code {
                            KeyCode::Char('j') | KeyCode::Down => Some(Message::GoToNextListItem),
                            KeyCode::Char('k') | KeyCode::Up => {
                                Some(Message::GoToPreviousListItem)
                            }
                            KeyCode::Char('g') => Some(Message::GoToFirstListItem),
                            KeyCode::Char('G') => Some(Message::GoToLastListItem),
                            KeyCode::Enter | KeyCode::Char('a') => {
                                Some(Message::ConfirmArrangeDatabases)
                            }
                            KeyCode::Esc | KeyCode::Char('q') => {
                                Some(Message::CancelArrangeDatabases)
                            }
                            _ => None,
                        }
                    }
                    ActivePane::DatabaseList => match key_event.code {
                        KeyCode::Char('j') | KeyCode::Down => Some(Message::GoToNextListItem),
                        KeyCode::Char('k') | KeyCode::Up => Some(Message::GoToPreviousListItem),
                        KeyCode::Char('g') => Some(Message::GoToFirstListItem),
                        KeyCode::Char('G') => Some(Message::GoToLastListItem),
                        KeyCode::Char('/') => {
                            Some(Message::ShowView(ActivePane::DatabaseSearchInput))
                        }
                        KeyCode::Enter => Some(Message::RequestSwitchDatabase),
                        // creating a new database only makes sense while
                        // switching the active database, not while picking
                        // a destination to move bookmark(s) into
                        KeyCode::Char('C') if model.db_list_purpose == DbListPurpose::Switch => {
                            Some(Message::StartNewDatabaseName)
                        }
                        // `a` arranges (re-orders) the databases and `R`
                        // renames the one under cursor - both only make
                        // sense while switching databases, not while
                        // picking a destination to move bookmark(s) into.
                        KeyCode::Char('a') if model.db_list_purpose == DbListPurpose::Switch => {
                            Some(Message::StartArrangeDatabases)
                        }
                        KeyCode::Char('R') if model.db_list_purpose == DbListPurpose::Switch => {
                            Some(Message::StartRenameDatabase)
                        }
                        KeyCode::Char('F') if model.db_list_purpose == DbListPurpose::Switch => {
                            Some(Message::ShowFolders)
                        }
                        KeyCode::Char('v') => Some(Message::CycleDbView),
                        // Number keys 1-9 jump straight to (and pick) the
                        // database shown next to that number in the list,
                        // same shortcut style as the mode switcher below -
                        // only fires while that many databases are actually
                        // listed, so a stray digit past the end falls
                        // through to `_ => None` instead of doing nothing
                        // silently.
                        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                            let index = (c as usize) - ('1' as usize);
                            if index < model.filtered_dbs.len() {
                                Some(Message::SelectDatabaseByNumber(index))
                            } else {
                                None
                            }
                        }
                        KeyCode::Esc | KeyCode::Char('q') => Some(Message::GoBackOrQuit),
                        _ => None,
                    },
                    ActivePane::DatabaseSearchInput => match key_event.code {
                        KeyCode::Esc => Some(Message::CancelDatabaseSearch),
                        KeyCode::Enter => Some(Message::SubmitDatabaseSearch),
                        KeyCode::Down => Some(Message::GoToNextListItem),
                        KeyCode::Up => Some(Message::GoToPreviousListItem),
                        _ => Some(Message::DatabaseSearchInputGotEvent(event)),
                    },
                    ActivePane::NewDatabaseName => match key_event.code {
                        KeyCode::Esc => Some(Message::GoBackOrQuit),
                        KeyCode::Enter => Some(Message::RequestCreateDatabase),
                        KeyCode::Char('s')
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            Some(Message::RequestCreateDatabase)
                        }
                        _ => Some(Message::NewDatabaseNameGotEvent(event)),
                    },
                    ActivePane::Confirm => match key_event.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => Some(Message::ConfirmYes),
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            Some(Message::ConfirmNo)
                        }
                        _ => None,
                    },
                    ActivePane::ModeSwitcher => match key_event.code {
                        KeyCode::Char('j') | KeyCode::Down => Some(Message::GoToNextListItem),
                        KeyCode::Char('k') | KeyCode::Up => Some(Message::GoToPreviousListItem),
                        KeyCode::Char('g') => Some(Message::GoToFirstListItem),
                        KeyCode::Char('G') => Some(Message::GoToLastListItem),
                        KeyCode::Enter => Some(Message::ConfirmModeSelection),
                        // Number keys 1-8 jump straight to and open the
                        // mode shown next to that number, without needing
                        // to move the selection down to it first.
                        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                            let index = (c as usize) - ('1' as usize);
                            if index < ModeOption::ALL.len() {
                                Some(Message::SelectModeByNumber(index))
                            } else {
                                None
                            }
                        }
                        KeyCode::Esc | KeyCode::Char('q') => Some(Message::GoBackOrQuit),
                        _ => None,
                    },
                    ActivePane::SearchScopePicker => match key_event.code {
                        KeyCode::Char('j') | KeyCode::Down => Some(Message::GoToNextListItem),
                        KeyCode::Char('k') | KeyCode::Up => Some(Message::GoToPreviousListItem),
                        KeyCode::Char('g') => Some(Message::GoToFirstListItem),
                        KeyCode::Char('G') => Some(Message::GoToLastListItem),
                        KeyCode::Enter => Some(Message::ConfirmSearchScopeSelection),
                        // Number keys 1-4 jump straight to and pick the
                        // scope shown next to that number, without
                        // needing to move the selection down to it first.
                        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                            let index = (c as usize) - ('1' as usize);
                            if index < SearchScopeOption::ALL.len() {
                                Some(Message::SelectSearchScopeByNumber(index))
                            } else {
                                None
                            }
                        }
                        KeyCode::Esc | KeyCode::Char('q') => Some(Message::GoBackOrQuit),
                        _ => None,
                    },
                    }
                }
                _ => None,
            },
        },
        Event::Mouse(mouse) => {
            if model.terminal_too_small {
                return None;
            }

            let clickable = matches!(
                model.active_pane,
                ActivePane::DatabaseList
                    | ActivePane::DatabaseSearchInput
                    | ActivePane::FolderList
                    | ActivePane::FolderContents
            );
            if !clickable {
                return None;
            }

            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => model
                    .clicked_row(mouse.column, mouse.row)
                    .map(Message::ClickListRow),
                MouseEventKind::ScrollUp => Some(Message::GoToPreviousListItem),
                MouseEventKind::ScrollDown => Some(Message::GoToNextListItem),
                _ => None,
            }
        }
        Event::Resize(w, h) => Some(Message::TerminalResize(w, h)),
        _ => None,
    }
}
