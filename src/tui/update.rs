use super::commands::Command;
use super::common::*;
use super::message::{Message, UrlsOpenedResult};
use super::model::*;
use crate::persistence::SearchTerms;
use tui_input::backend::crossterm::EventHandler;

pub fn update(model: &mut Model, msg: Message) -> Vec<Command> {
    let mut cmds = Vec::new();
    match msg {
        Message::GoToNextListItem => model.select_next_list_item(),
        Message::GoToPreviousListItem => model.select_previous_list_item(),
        Message::OpenInBrowser => {
            if let Some(c) = model.get_cmd_to_open_selection_in_browser() {
                cmds.push(c)
            }
        }
        Message::RequestOpenAllInBrowser => {
            if let Some(c) = model.request_open_all_in_browser() {
                cmds.push(c);
            }
        }
        Message::UrlsOpenedInBrowser(result) => {
            if let UrlsOpenedResult::Failure(e) = result {
                model.user_message =
                    Some(UserMessage::error(&format!("urls couldn't be opened: {e}")));
            }
        }
        Message::GoBackOrQuit => model.go_back_or_quit(),
        Message::ShowView(view) => {
            if let Some(c) = model.show_view(view) {
                cmds.push(c);
            }
        }
        Message::GoToFirstListItem => model.select_first_list_item(),
        Message::GoToLastListItem => model.select_last_list_item(),
        Message::SearchFinished(result) => match result {
            Ok(bookmarks) => {
                model.viewing_duplicates = false;
                if bookmarks.is_empty() {
                    model.user_message = Some(UserMessage::info("no bookmarks found for query"));
                    model.bookmark_items = BookmarkItems::from(vec![]);
                } else {
                    let bookmarks_len = bookmarks.len();
                    if let Some(current_index) = model.bookmark_items.state.selected() {
                        if current_index < bookmarks_len {
                            model.bookmark_items = BookmarkItems::from((bookmarks, current_index));
                        } else {
                            model.bookmark_items =
                                BookmarkItems::from((bookmarks, bookmarks_len - 1));
                        }
                    } else {
                        model.bookmark_items = BookmarkItems::from(bookmarks);
                    }
                }
            }
            Err(e) => model.user_message = Some(UserMessage::error(&format!("{e}"))),
        },
        Message::AllBookmarksFetched(result) => match result {
            Ok(bookmarks) => {
                model.viewing_duplicates = false;
                if bookmarks.is_empty() {
                    model.user_message = Some(UserMessage::info("no bookmarks saved yet"));
                    model.bookmark_items = BookmarkItems::from(vec![]);
                } else {
                    model.bookmark_items = BookmarkItems::from(bookmarks);
                }
            }
            Err(e) => model.user_message = Some(UserMessage::error(&format!("{e}"))),
        },
        Message::TagsFetched(result) => match result {
            Ok(t) => {
                model.all_tag_items = t.clone();
                model.tag_items = TagItems::from(t);
                model.active_pane = ActivePane::TagsList;
            }
            Err(e) => model.user_message = Some(UserMessage::error(&format!("{e}"))),
        },
        Message::SearchInputGotEvent(event) => {
            model.search_input.handle_event(&event);
        }
        Message::TagSearchInputGotEvent(event) => {
            model.tag_search_input.handle_event(&event);
            model.filter_tags();
        }
        Message::SubmitTagSearch => {
            model.confirm_tag_search();
        }
        Message::CancelTagSearch => {
            model.cancel_tag_search();
        }
        Message::SubmitSearch => {
            let search_query = model.search_input.value();
            if search_query.trim().is_empty() {
                cmds.push(Command::FetchAllBookmarks);
                if model.initial {
                    model.initial = false;
                }
                model.search_input.reset();
                model.active_pane = ActivePane::List;
            } else {
                match SearchTerms::try_from(search_query) {
                    Ok(search_terms) => {
                        cmds.push(Command::SearchBookmarks(search_terms));
                        if model.initial {
                            model.initial = false;
                        }
                        model.search_input.reset();
                        model.active_pane = ActivePane::List;
                    }
                    Err(e) => model.user_message = Some(UserMessage::error(&format!("{e}"))),
                }
            }
        }
        Message::TerminalResize(width, height) => {
            model.terminal_dimensions = TerminalDimensions { width, height };
            model.terminal_too_small =
                !(width >= MIN_TERMINAL_WIDTH && height >= MIN_TERMINAL_HEIGHT);
        }
        Message::ShowBookmarksForTag => {
            if let Some(current_tag_index) = model.tag_items.state.selected()
                && let Some(selected_tag) = model.tag_items.items.get(current_tag_index)
            {
                cmds.push(Command::FetchBookmarksForTag(selected_tag.name.to_string()));
            }
        }
        Message::BookmarksForTagFetched(result) => match result {
            Ok(bookmarks) => {
                model.viewing_duplicates = false;
                model.bookmark_items = BookmarkItems::from(bookmarks);
                model.active_pane = ActivePane::List;
            }
            Err(e) => model.user_message = Some(UserMessage::error(&format!("{e}"))),
        },
        Message::ShowDuplicates => {
            cmds.push(Command::FetchDuplicateBookmarks);
        }
        Message::DuplicateBookmarksFetched(result) => match result {
            Ok(bookmarks) => {
                model.viewing_duplicates = true;
                if bookmarks.is_empty() {
                    model.user_message = Some(UserMessage::info("no duplicate bookmarks found"));
                    model.bookmark_items = BookmarkItems::from(vec![]);
                } else {
                    model.bookmark_items = BookmarkItems::from(bookmarks);
                }
                model.active_pane = ActivePane::List;
            }
            Err(e) => model.user_message = Some(UserMessage::error(&format!("{e}"))),
        },
        Message::RequestDeleteBookmark => {
            model.request_delete_selected_bookmark();
        }
        Message::BookmarkDeleted(result) => match result {
            Ok(_) => {
                if let Some(PendingConfirmation::DeleteBookmark(uri)) =
                    model.pending_confirmation.take()
                {
                    if model.viewing_duplicates {
                        // a bookmark that was part of a duplicate-title group
                        // was just deleted; the remaining bookmark(s) that
                        // shared that title may no longer be duplicates, so
                        // refresh the list from the database instead of
                        // just removing this one item locally.
                        cmds.push(Command::FetchDuplicateBookmarks);
                    } else {
                        model.remove_bookmark_by_uri(&uri);
                    }
                }
                model.user_message =
                    Some(UserMessage::info("bookmark deleted!").with_frames_left(1));
            }
            Err(e) => {
                model.pending_confirmation = None;
                model.user_message = Some(UserMessage::error(&format!(
                    "couldn't delete bookmark: {e}"
                )));
            }
        },
        Message::StartEditBookmark(uri_editable) => {
            model.start_edit_selected_bookmark(uri_editable);
        }
        Message::EditFieldGotEvent(event) => match model.edit_focus {
            EditField::Uri => {
                model.edit_uri_input.handle_event(&event);
            }
            EditField::Title => {
                model.edit_title_input.handle_event(&event);
            }
            EditField::Tags => {
                model.edit_tags_input.handle_event(&event);
            }
        },
        Message::EditFieldNext => model.edit_focus_next(),
        Message::EditFieldPrev => model.edit_focus_previous(),
        Message::RequestSaveBookmarkEdit => {
            model.request_save_edit();
        }
        Message::RequestExitEdit => {
            model.request_exit_edit();
        }
        Message::BookmarkUpdated(result) => match result {
            Ok(()) => {
                let title = {
                    let t = model.edit_title_input.value().trim();
                    if t.is_empty() { None } else { Some(t.to_string()) }
                };
                let tags = {
                    let t = model.edit_tags_input.value().trim();
                    if t.is_empty() { None } else { Some(t.to_string()) }
                };
                let new_uri = {
                    let u = model.edit_uri_input.value().trim();
                    if model.edit_uri_editable && u != model.edit_original_uri.trim() {
                        Some(u.to_string())
                    } else {
                        None
                    }
                };
                model.apply_bookmark_edit_locally(new_uri, title, tags);
                model.cancel_edit();
                model.user_message =
                    Some(UserMessage::info("bookmark updated!").with_frames_left(1));
            }
            Err(e) => {
                model.user_message =
                    Some(UserMessage::error(&format!("couldn't update bookmark: {e}")));
            }
        },
        Message::StartNoteEdit => {
            if let Some(uri) = model.get_selected_bookmark_uri() {
                model.note_action = NoteAction::Edit;
                cmds.push(Command::FetchNote(uri));
            }
        }
        Message::RequestDeleteNote => {
            if let Some(c) = model.request_delete_note_for_selected() {
                cmds.push(c);
            }
        }
        Message::NoteFetched(uri, result) => match result {
            Ok(note) => model.handle_note_fetched(uri, note),
            Err(e) => model.user_message = Some(UserMessage::error(&format!("{e}"))),
        },
        Message::NoteInputGotEvent(event) => {
            model.note_input.handle_event(&event);
        }
        Message::RequestSaveNote => {
            model.request_save_note();
        }
        Message::RequestExitNote => {
            model.request_exit_note();
        }
        Message::NoteSaved(result) => match result {
            Ok(()) => {
                let was_delete = model.note_action == NoteAction::Delete;
                model.cancel_note_edit();
                let msg = if was_delete { "note deleted!" } else { "note saved!" };
                model.user_message = Some(UserMessage::info(msg).with_frames_left(1));
            }
            Err(e) => {
                model.user_message =
                    Some(UserMessage::error(&format!("couldn't save note: {e}")));
            }
        },
        Message::ConfirmYes => {
            if let Some(confirmation) = model.pending_confirmation.take() {
                match confirmation {
                    PendingConfirmation::DeleteBookmark(uri) => {
                        cmds.push(Command::DeleteBookmark(uri));
                        model.active_pane = ActivePane::List;
                    }
                    PendingConfirmation::SaveEdit => {
                        let title = {
                            let t = model.edit_title_input.value().trim();
                            if t.is_empty() { None } else { Some(t.to_string()) }
                        };
                        let tags: Vec<String> = model
                            .edit_tags_input
                            .value()
                            .split(',')
                            .map(|t| t.trim().to_string())
                            .filter(|t| !t.is_empty())
                            .collect();
                        let new_uri = {
                            let u = model.edit_uri_input.value().trim();
                            if model.edit_uri_editable && u != model.edit_original_uri.trim() {
                                Some(u.to_string())
                            } else {
                                None
                            }
                        };

                        cmds.push(Command::UpdateBookmark {
                            uri: model.edit_original_uri.clone(),
                            new_uri,
                            title,
                            tags,
                        });
                        model.active_pane = ActivePane::EditBookmark;
                    }
                    PendingConfirmation::DiscardEdit => {
                        model.cancel_edit();
                    }
                    PendingConfirmation::SaveNote => {
                        let note = {
                            let n = model.note_input.value().trim();
                            if n.is_empty() { None } else { Some(n.to_string()) }
                        };

                        cmds.push(Command::SaveNote {
                            uri: model.note_uri.clone(),
                            note,
                        });
                        model.active_pane = ActivePane::Notes;
                    }
                    PendingConfirmation::DiscardNote => {
                        model.cancel_note_edit();
                    }
                    PendingConfirmation::DeleteNote(uri) => {
                        model.note_action = NoteAction::Delete;
                        cmds.push(Command::SaveNote { uri, note: None });
                        model.active_pane = ActivePane::List;
                    }
                    PendingConfirmation::TooManyLinksWarning(_) => {
                        model.active_pane = model.pane_before_confirm;
                    }
                }
            }
        }
        Message::ConfirmNo => {
            model.cancel_confirm();
        }
        Message::CopyURIToClipboard => {
            if let Some(uri) = model.get_uri_under_cursor() {
                cmds.push(Command::CopyContentToClipboard(uri));
            }
        }
        Message::CopyURIsToClipboard => {
            let uris = model
                .bookmark_items
                .items
                .iter()
                .map(|bi| bi.bookmark.uri.as_str())
                .collect::<Vec<_>>();

            if !uris.is_empty() {
                cmds.push(Command::CopyContentToClipboard(uris.join("\n")));
            }
        }
        Message::ContentCopiedToClipboard(result) => {
            if let Err(error) = result {
                model.user_message = Some(UserMessage::error(&format!(
                    "couldn't copy uri to clipboard: {error}"
                )));
            } else {
                model.user_message = Some(UserMessage::info("copied!").with_frames_left(1));
            }
        }
    }

    if let Some(message) = &mut model.user_message {
        let clear = if message.frames_left == 0 {
            true
        } else {
            message.frames_left -= 1;
            false
        };

        if clear {
            model.user_message = None;
        }
    }

    cmds
}
