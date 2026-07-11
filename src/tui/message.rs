use super::common::ActivePane;
use super::model::Model;
use crate::domain::{SavedBookmark, TagStats};
use crate::persistence::DBError;
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use std::io::Error as IOError;

pub enum Message {
    TerminalResize(u16, u16),
    GoToNextListItem,
    GoToPreviousListItem,
    GoToFirstListItem,
    GoToLastListItem,
    OpenInBrowser,
    RequestOpenAllInBrowser,
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
    RequestDeleteBookmark,
    BookmarkDeleted(Result<u64, DBError>),
    StartEditBookmark,
    EditFieldGotEvent(Event),
    EditFieldNext,
    EditFieldPrev,
    RequestSaveBookmarkEdit,
    RequestExitEdit,
    BookmarkUpdated(Result<(), String>),
    StartNoteEdit,
    NoteFetched(String, Result<Option<String>, DBError>),
    NoteInputGotEvent(Event),
    RequestSaveNote,
    RequestExitNote,
    NoteSaved(Result<(), String>),
    ConfirmYes,
    ConfirmNo,
    ContentCopiedToClipboard(Result<(), String>),
    GoBackOrQuit,
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
                KeyEventKind::Press => match model.active_pane {
                    ActivePane::List => match key_event.code {
                        KeyCode::Char('j') | KeyCode::Down => Some(Message::GoToNextListItem),
                        KeyCode::Char('k') | KeyCode::Up => Some(Message::GoToPreviousListItem),
                        KeyCode::Char('g') => Some(Message::GoToFirstListItem),
                        KeyCode::Char('G') => Some(Message::GoToLastListItem),
                        KeyCode::Char('o') => Some(Message::OpenInBrowser),
                        KeyCode::Char('O') => Some(Message::RequestOpenAllInBrowser),
                        KeyCode::Char('s') => Some(Message::ShowView(ActivePane::SearchInput)),
                        KeyCode::Char('t') | KeyCode::Tab => {
                            Some(Message::ShowView(ActivePane::TagsList))
                        }
                        KeyCode::Char('d') => Some(Message::ShowDuplicates),
                        KeyCode::Char('e') => Some(Message::StartEditBookmark),
                        KeyCode::Char('n') => Some(Message::StartNoteEdit),
                        KeyCode::Delete => Some(Message::RequestDeleteBookmark),
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
                    ActivePane::Confirm => match key_event.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => Some(Message::ConfirmYes),
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            Some(Message::ConfirmNo)
                        }
                        _ => None,
                    },
                },
                _ => None,
            },
        },
        Event::Resize(w, h) => Some(Message::TerminalResize(w, h)),
        _ => None,
    }
}
