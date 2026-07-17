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
            if let Some(c) = model.get_cmd_to_open_selection_in_browser(false) {
                cmds.push(c)
            }
        }
        Message::OpenInBrowserIncognito => {
            if let Some(c) = model.get_cmd_to_open_selection_in_browser(true) {
                cmds.push(c)
            }
        }
        Message::RequestOpenAllInBrowser => {
            if let Some(c) = model.request_open_all_in_browser(false) {
                cmds.push(c);
            }
        }
        Message::RequestOpenAllInBrowserIncognito => {
            if let Some(c) = model.request_open_all_in_browser(true) {
                cmds.push(c);
            }
        }
        Message::UrlsOpenedInBrowser(result) => match result {
            UrlsOpenedResult::Failure(e) => {
                model.user_message =
                    Some(UserMessage::error(&format!("urls couldn't be opened: {e}")));
            }
            #[cfg(target_os = "android")]
            UrlsOpenedResult::SuccessNeedsPaste(count) => {
                let msg = if count == 1 {
                    "opened a new incognito tab; link copied to clipboard, paste it in \
(android doesn't let apps load a url straight into incognito mode)"
                        .to_string()
                } else {
                    format!(
                        "opened a new incognito tab; {count} links copied to clipboard \
(one per line), paste to open them"
                    )
                };
                model.user_message = Some(UserMessage::info(&msg).with_frames_left(6));
            }
            UrlsOpenedResult::Success => {}
        },
        Message::GoBackOrQuit => model.go_back_or_quit(),
        Message::ToggleModeSwitcher => {
            if model.active_pane == ActivePane::ModeSwitcher {
                model.active_pane = model.pane_before_mode_switch;
            } else {
                model.open_mode_switcher();
            }
        }
        Message::ConfirmModeSelection => {
            let selected_mode = model
                .mode_switcher_state
                .selected()
                .and_then(|i| ModeOption::ALL.get(i).copied());

            // Go back to a "normal" pane before dispatching the chosen
            // mode's message, same as if the user had pressed Esc/q then
            // that mode's own shortcut key - this keeps every mode's
            // transition logic exactly as it already was (nothing here is
            // mode-switcher-specific), and avoids `show_view`'s "coming
            // from Help" special case misfiring for a pane that isn't
            // actually Help.
            model.active_pane = ActivePane::List;

            if let Some(mode) = selected_mode {
                let follow_up_cmds = update(model, mode.into_message());
                cmds.extend(follow_up_cmds);
            }
        }
        Message::ShowAllBookmarks => {
            model.global_search_mode = false;
            model.initial = false;
            model.active_pane = ActivePane::List;
            cmds.push(Command::FetchAllBookmarks);
        }
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
                model.showing_starred = false;
                model.initial = false;
                if bookmarks.is_empty() {
                    model.user_message = Some(UserMessage::info("no bookmarks found for query"));
                    model.bookmark_items = BookmarkItems::default();
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
                model.sync_starred_markers();
            }
            Err(e) => model.user_message = Some(UserMessage::error(&format!("{e}"))),
        },
        Message::AllBookmarksFetched(result) => match result {
            Ok(bookmarks) => {
                model.viewing_duplicates = false;
                model.showing_starred = false;
                model.initial = false;
                if bookmarks.is_empty() {
                    model.user_message = Some(UserMessage::info("no bookmarks saved yet"));
                    model.bookmark_items = BookmarkItems::default();
                } else {
                    model.bookmark_items = BookmarkItems::from(bookmarks);
                }
                model.sync_starred_markers();
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
            let is_global = model.global_search_mode;
            model.global_search_mode = false;

            if search_query.trim().is_empty() {
                if is_global {
                    cmds.push(Command::GlobalSearch(None));
                } else {
                    cmds.push(Command::FetchAllBookmarks);
                }
                if model.initial {
                    model.initial = false;
                }
                model.search_input.reset();
                model.active_pane = ActivePane::List;
            } else {
                match SearchTerms::try_from(search_query) {
                    Ok(search_terms) => {
                        if is_global {
                            cmds.push(Command::GlobalSearch(Some(search_terms)));
                        } else {
                            cmds.push(Command::SearchBookmarks(search_terms));
                        }
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
                model.showing_starred = false;
                model.initial = false;
                model.bookmark_items = BookmarkItems::from(bookmarks);
                model.sync_starred_markers();
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
                model.showing_starred = false;
                model.initial = false;
                if bookmarks.is_empty() {
                    model.user_message = Some(UserMessage::info("no duplicate bookmarks found"));
                    model.bookmark_items = BookmarkItems::default();
                } else {
                    model.bookmark_items = BookmarkItems::from(bookmarks);
                }
                model.sync_starred_markers();
                model.active_pane = ActivePane::List;
            }
            Err(e) => model.user_message = Some(UserMessage::error(&format!("{e}"))),
        },
        Message::ShowStarred => {
            cmds.push(Command::FetchStarredBookmarks);
        }
        Message::StarredBookmarksFetched(result) => match result {
            Ok(bookmarks) => {
                model.viewing_duplicates = false;
                model.showing_starred = true;
                model.initial = false;
                if bookmarks.is_empty() {
                    model.user_message = Some(UserMessage::info("no starred bookmarks yet"));
                    model.bookmark_items = BookmarkItems::default();
                } else {
                    model.bookmark_items = BookmarkItems::from(bookmarks);
                }
                model.sync_starred_markers();
                model.active_pane = ActivePane::List;
            }
            Err(e) => model.user_message = Some(UserMessage::error(&format!("{e}"))),
        },
        Message::StarredUrisFetched(result) => {
            if let Ok(uris) = result {
                model.starred_uris = uris;
                model.sync_starred_markers();
            }
        }
        Message::RequestToggleStar => {
            if let Some(c) = model.request_toggle_star() {
                cmds.push(c);
            }
        }
        Message::StarToggled(uri, result) => match result {
            Ok(new_state) => {
                model.apply_star_toggle(&uri, new_state);
                let msg = if new_state { "starred!" } else { "unstarred!" };
                model.user_message = Some(UserMessage::info(msg).with_frames_left(1));
            }
            Err(e) => {
                model.user_message =
                    Some(UserMessage::error(&format!("couldn't toggle star: {e}")));
            }
        },
        Message::ShowDatabaseList => {
            model.show_database_list();
        }
        Message::RequestSwitchDatabase => {
            if let Some(c) = model.request_switch_to_selected_database() {
                cmds.push(c);
            }
        }
        Message::StartNewDatabaseName => {
            model.start_new_database_name();
        }
        Message::NewDatabaseNameGotEvent(event) => {
            model.new_db_name_input.handle_event(&event);
        }
        Message::RequestCreateDatabase => {
            if let Some(c) = model.request_create_database() {
                cmds.push(c);
            }
        }
        Message::DatabaseSwitched(result) => match result {
            Ok((new_pool, display_name)) => {
                model.pool = new_pool;
                model.apply_database_switch(display_name);
                cmds.push(Command::FetchAllBookmarks);
                cmds.push(Command::FetchStarredUris);
                model.user_message =
                    Some(UserMessage::info("switched database!").with_frames_left(1));
            }
            Err(e) => {
                model.user_message =
                    Some(UserMessage::error(&format!("couldn't switch database: {e}")));
            }
        },
        Message::ShowGlobalSearch => {
            model.global_search_mode = true;
            model.active_pane = ActivePane::SearchInput;
        }
        Message::GlobalSearchFinished(results, errors) => {
            model.viewing_duplicates = false;
            model.showing_starred = false;
            model.initial = false;

            if results.is_empty() {
                model.bookmark_items = BookmarkItems::default();
                model.user_message = Some(UserMessage::info(
                    "no bookmarks found across any database",
                ));
            } else {
                let items: Vec<BookmarkItem> = results
                    .into_iter()
                    .map(|(db_name, db_path, bookmark)| {
                        BookmarkItem::with_source_db(bookmark, db_name, db_path)
                    })
                    .collect();
                model.bookmark_items = BookmarkItems::from(items);

                if !errors.is_empty() {
                    model.user_message = Some(UserMessage::error(&format!(
                        "some databases couldn't be searched: {}",
                        errors.join("; ")
                    )));
                }
            }

            model.sync_starred_markers();
            model.active_pane = ActivePane::List;
        }
        Message::RequestDeleteBookmark => {
            model.request_delete_selected_bookmark();
        }
        Message::BookmarkDeleted(result) => match result {
            Ok(_) => {
                if let Some(PendingConfirmation::DeleteBookmark(uri, _target_db_path)) =
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
        Message::StartAddBookmark => {
            model.start_add_new_bookmark();
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
                let new_uri = if model.edit_is_new {
                    let u = model.edit_uri_input.value().trim().to_string();
                    Some(crate::domain::normalize_uri_scheme(u))
                } else {
                    let u = model.edit_uri_input.value().trim();
                    if model.edit_uri_editable && u != model.edit_original_uri.trim() {
                        Some(crate::domain::normalize_uri_scheme(u.to_string()))
                    } else {
                        None
                    }
                };
                let was_new = model.edit_is_new;
                model.apply_bookmark_edit_locally(new_uri, title, tags);
                model.cancel_edit();
                let msg = if was_new { "bookmark saved!" } else { "bookmark updated!" };
                model.user_message = Some(UserMessage::info(msg).with_frames_left(1));
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
                    PendingConfirmation::DeleteBookmark(uri, target_db_path) => {
                        cmds.push(Command::DeleteBookmark(uri, target_db_path));
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

                        if model.edit_is_new {
                            let uri = model.edit_uri_input.value().trim().to_string();
                            cmds.push(Command::UpdateBookmark {
                                uri,
                                new_uri: None,
                                title,
                                tags,
                                is_new: true,
                                target_db_path: None,
                            });
                        } else {
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
                                is_new: false,
                                target_db_path: model.edit_target_db_path.clone(),
                            });
                        }
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
