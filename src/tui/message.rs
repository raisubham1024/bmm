use super::common::ActivePane;
use super::model::Model;
use crate::domain::{SavedBookmark, TagStats};
use crate::persistence::DBError;
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
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
    ShowDuplicates,
    DuplicateBookmarksFetched(Result<Vec<SavedBookmark>, DBError>),
    ShowStarred,
    StarredBookmarksFetched(Result<Vec<SavedBookmark>, DBError>),
    StarredUrisFetched(Result<HashSet<String>, DBError>),
    RequestToggleStar,
    StarToggled(String, Result<bool, String>),
    ShowDatabaseList,
    RequestSwitchDatabase,
    StartNewDatabaseName,
    NewDatabaseNameGotEvent(Event),
    RequestCreateDatabase,
    DatabaseSwitched(Result<(Pool<Sqlite>, String), String>),
    ShowGlobalSearch,
    GlobalSearchFinished(Vec<(String, String, SavedBookmark)>, Vec<String>),
    RequestDeleteBookmark,
    BookmarkDeleted(Result<u64, DBError>),
    StartEditBookmark(bool),
    EditFieldGotEvent(Event),
    EditFieldNext,
    EditFieldPrev,
    RequestSaveBookmarkEdit,
    RequestExitEdit,
    BookmarkUpdated(Result<(), String>),
    StartNoteEdit,
    RequestDeleteNote,
    NoteFetched(String, Result<Option<String>, DBError>),
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
    ShowAllBookmarks,
}

pub enum UrlsOpenedResult {
    Success,
    /// Android-only: the incognito tab was opened, but couldn't be loaded
    /// with the url(s) directly (Chrome doesn't allow third-party apps to
    /// do that) — they were copied to the clipboard instead. The count is
    /// how many urls were copied, so the message shown can say "1 link" vs
    /// "N links".
    #[cfg(target_os = "android")]
    SuccessNeedsPaste(usize),
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
                        KeyCode::Char('a') => Some(Message::StartAddBookmark),
                        KeyCode::Char('t') | KeyCode::Tab => {
                            Some(Message::ShowView(ActivePane::TagsList))
                        }
                        KeyCode::Char('d') => Some(Message::ShowDuplicates),
                        KeyCode::Char('D') => Some(Message::RequestDeleteBookmark),
                        KeyCode::Char('S') => Some(Message::ShowStarred),
                        KeyCode::Char('*') => Some(Message::RequestToggleStar),
                        KeyCode::Char('A') => Some(Message::ShowDatabaseList),
                        KeyCode::Char('z') => Some(Message::ShowGlobalSearch),
                        KeyCode::Char('e') => Some(Message::StartEditBookmark(false)),
                        KeyCode::Char('E') => Some(Message::StartEditBookmark(true)),
                        KeyCode::Char('n') => Some(Message::StartNoteEdit),
                        KeyCode::Char('N') => Some(Message::RequestDeleteNote),
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
                        KeyCode::Esc | KeyCode::Char('q') => Some(Message::GoBackOrQuit),
                        KeyCode::Char('?') => Some(Message::ShowView(ActivePane::List)),
                        _ => None,
                    },
                    ActivePane::SearchInput => match key_event.code {
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
                        KeyCode::Enter => Some(Message::ShowBookmarksForTag),
                        KeyCode::Esc | KeyCode::Char('q') => Some(Message::GoBackOrQuit),
                        _ => None,
                    },
                    ActivePane::TagSearchInput => match key_event.code {
                        KeyCode::Esc => Some(Message::CancelTagSearch),
                        KeyCode::Enter => Some(Message::SubmitTagSearch),
                        KeyCode::Down => Some(Message::GoToNextListItem),
                        KeyCode::Up => Some(Message::GoToPreviousListItem),
                        _ => Some(Message::TagSearchInputGotEvent(event)),
                    },
                    ActivePane::EditBookmark => match key_event.code {
                        KeyCode::Esc => Some(Message::RequestExitEdit),
                        KeyCode::Tab | KeyCode::Down => Some(Message::EditFieldNext),
                        KeyCode::BackTab | KeyCode::Up => Some(Message::EditFieldPrev),
                        KeyCode::Char('s')
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            Some(Message::RequestSaveBookmarkEdit)
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
                    ActivePane::DatabaseList => match key_event.code {
                        KeyCode::Char('j') | KeyCode::Down => Some(Message::GoToNextListItem),
                        KeyCode::Char('k') | KeyCode::Up => Some(Message::GoToPreviousListItem),
                        KeyCode::Char('g') => Some(Message::GoToFirstListItem),
                        KeyCode::Char('G') => Some(Message::GoToLastListItem),
                        KeyCode::Enter => Some(Message::RequestSwitchDatabase),
                        KeyCode::Char('C') => Some(Message::StartNewDatabaseName),
                        KeyCode::Esc | KeyCode::Char('q') => Some(Message::GoBackOrQuit),
                        _ => None,
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
                        KeyCode::Esc | KeyCode::Char('q') => Some(Message::GoBackOrQuit),
                        _ => None,
                    },
                    }
                }
                _ => None,
            },
        },
        Event::Resize(w, h) => Some(Message::TerminalResize(w, h)),
        _ => None,
    }
}
