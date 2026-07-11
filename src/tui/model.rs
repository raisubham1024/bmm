use super::{commands::Command, common::*};
use crate::{
    domain::{SavedBookmark, TagStats},
    persistence::SearchTerms,
};
use ratatui::{
    style::Style,
    text::Line,
    widgets::{ListItem, ListState},
};
use sqlx::{Pool, Sqlite};
use tui_input::Input;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum RunningState {
    #[default]
    Running,
    Done,
}

#[derive(Debug)]
pub(crate) struct BookmarkItem {
    pub(crate) bookmark: SavedBookmark,
    pub(crate) status: bool,
}

#[derive(Debug)]
pub(crate) struct BookmarkItems {
    pub(crate) items: Vec<BookmarkItem>,
    pub(crate) state: ListState,
}

#[derive(Debug)]
pub(crate) struct TagItems {
    pub(crate) items: Vec<TagStats>,
    pub(crate) state: ListState,
}

#[derive(Debug)]
pub enum MessageKind {
    Info,
    Error,
}

pub struct UserMessage {
    pub frames_left: u8,
    pub value: String,
    pub kind: MessageKind,
}

impl UserMessage {
    pub(super) fn info(message: &str) -> Self {
        UserMessage {
            frames_left: 1,
            value: message.to_string(),
            kind: MessageKind::Info,
        }
    }
    pub(super) fn error(message: &str) -> Self {
        UserMessage {
            frames_left: 4,
            value: message.to_string(),
            kind: MessageKind::Error,
        }
    }

    pub(super) fn with_frames_left(mut self, frames_left: u8) -> Self {
        self.frames_left = frames_left;
        self
    }
}

pub enum TuiContext {
    Initial,
    Search(SearchTerms),
    Tags,
}

impl BookmarkItems {
    fn default() -> Self {
        let state = ListState::default().with_selected(None);

        Self {
            items: vec![],
            state,
        }
    }
}

impl TagItems {
    fn default() -> Self {
        let state = ListState::default().with_selected(None);

        Self {
            items: vec![],
            state,
        }
    }
}

impl From<Vec<SavedBookmark>> for BookmarkItems {
    fn from(bookmarks: Vec<SavedBookmark>) -> Self {
        let items = bookmarks
            .into_iter()
            .map(|bookmark| BookmarkItem::new(bookmark, false))
            .collect();
        let state = ListState::default().with_selected(Some(0));

        Self { items, state }
    }
}

impl From<(Vec<SavedBookmark>, usize)> for BookmarkItems {
    fn from(value: (Vec<SavedBookmark>, usize)) -> Self {
        let bookmarks = value.0;
        let index = value.1;
        let items = bookmarks
            .into_iter()
            .map(|bookmark| BookmarkItem::new(bookmark, false))
            .collect();
        let state = ListState::default().with_selected(Some(index));

        Self { items, state }
    }
}

impl From<Vec<TagStats>> for TagItems {
    fn from(tags: Vec<TagStats>) -> Self {
        let state = ListState::default().with_selected(Some(0));

        Self { items: tags, state }
    }
}

impl BookmarkItem {
    fn new(bookmark: SavedBookmark, status: bool) -> Self {
        Self { bookmark, status }
    }
}

impl From<&BookmarkItem> for ListItem<'_> {
    fn from(value: &BookmarkItem) -> Self {
        let line = match value.status {
            false => Line::from(value.bookmark.uri.clone()),
            true => Line::styled(
                format!("> {}", value.bookmark.uri.clone()),
                Style::new().fg(COLOR_TWO),
            ),
        };
        ListItem::new(line)
    }
}

impl From<&TagStats> for ListItem<'_> {
    fn from(tag_with_stats: &TagStats) -> Self {
        let line = Line::from(tag_with_stats.name.clone());
        ListItem::new(line)
    }
}

#[derive(Debug, Clone)]
pub(super) enum PendingConfirmation {
    DeleteBookmark(String),
    SaveEdit,
    DiscardEdit,
    TooManyLinksWarning(usize),
}

pub(super) struct Model {
    pub(super) pool: Pool<Sqlite>,
    pub(super) active_pane: ActivePane,
    pub(super) bookmark_items: BookmarkItems,
    pub(super) tag_items: TagItems,
    pub(super) all_tag_items: Vec<TagStats>,
    pub(super) running_state: RunningState,
    pub(super) user_message: Option<UserMessage>,
    pub(super) render_counter: u64,
    pub(super) event_counter: u64,
    pub(super) search_input: Input,
    pub(super) tag_search_input: Input,
    pub(super) initial: bool,
    pub(super) terminal_dimensions: TerminalDimensions,
    pub(super) terminal_too_small: bool,
    pub(super) debug: bool,
    pub(super) pending_confirmation: Option<PendingConfirmation>,
    pub(super) pane_before_confirm: ActivePane,
    pub(super) edit_uri: String,
    pub(super) edit_title_input: Input,
    pub(super) edit_tags_input: Input,
    pub(super) edit_focus: EditField,
    pub(super) edit_original_title: Option<String>,
    pub(super) edit_original_tags: Option<String>,
    pub(super) viewing_duplicates: bool,
}

impl Model {
    pub(crate) fn default(
        pool: &Pool<Sqlite>,
        context: TuiContext,
        terminal_dimensions: TerminalDimensions,
    ) -> Self {
        let debug = std::env::var("BMM_DEBUG").unwrap_or_default().trim() == "1";

        let active_pane = match context {
            TuiContext::Search(_) => ActivePane::List,
            TuiContext::Tags => ActivePane::TagsList,
            TuiContext::Initial => ActivePane::SearchInput,
        };

        let initial = matches!(context, TuiContext::Initial);

        let terminal_too_small = terminal_dimensions.width < MIN_TERMINAL_WIDTH
            || terminal_dimensions.height < MIN_TERMINAL_HEIGHT;

        Self {
            pool: pool.clone(),
            active_pane,
            running_state: RunningState::Running,
            bookmark_items: BookmarkItems::default(),
            tag_items: TagItems::default(),
            all_tag_items: vec![],
            user_message: None,
            render_counter: 0,
            event_counter: 0,
            search_input: Input::default(),
            tag_search_input: Input::default(),
            initial,
            terminal_dimensions,
            terminal_too_small,
            debug,
            pending_confirmation: None,
            pane_before_confirm: ActivePane::List,
            edit_uri: String::new(),
            edit_title_input: Input::default(),
            edit_tags_input: Input::default(),
            edit_focus: EditField::Title,
            edit_original_title: None,
            edit_original_tags: None,
            viewing_duplicates: false,
        }
    }

    pub(super) fn select_next_list_item(&mut self) {
        match self.active_pane {
            ActivePane::List => self.bookmark_items.state.select_next(),
            ActivePane::TagsList | ActivePane::TagSearchInput => {
                self.tag_items.state.select_next()
            }
            ActivePane::SearchInput => {}
            ActivePane::EditBookmark => {}
            ActivePane::Confirm => {}
            ActivePane::Help => {}
        }
    }

    pub(super) fn select_previous_list_item(&mut self) {
        match self.active_pane {
            ActivePane::List => self.bookmark_items.state.select_previous(),
            ActivePane::TagsList | ActivePane::TagSearchInput => {
                self.tag_items.state.select_previous()
            }
            ActivePane::SearchInput => {}
            ActivePane::EditBookmark => {}
            ActivePane::Confirm => {}
            ActivePane::Help => {}
        }
    }

    pub(super) fn select_first_list_item(&mut self) {
        match self.active_pane {
            ActivePane::List => self.bookmark_items.state.select_first(),
            ActivePane::TagsList | ActivePane::TagSearchInput => {
                self.tag_items.state.select_first()
            }
            ActivePane::SearchInput => {}
            ActivePane::EditBookmark => {}
            ActivePane::Confirm => {}
            ActivePane::Help => {}
        }
    }
    pub(super) fn select_last_list_item(&mut self) {
        match self.active_pane {
            ActivePane::List => self.bookmark_items.state.select_last(),
            ActivePane::TagsList | ActivePane::TagSearchInput => {
                self.tag_items.state.select_last()
            }
            ActivePane::SearchInput => {}
            ActivePane::EditBookmark => {}
            ActivePane::Confirm => {}
            ActivePane::Help => {}
        }
    }

    pub(super) fn show_view(&mut self, view: ActivePane) -> Option<Command> {
        self.active_pane = match self.active_pane {
            ActivePane::Help => ActivePane::List,
            ActivePane::List => view,
            ActivePane::TagsList => view,
            ActivePane::TagSearchInput => view,
            ActivePane::SearchInput => view,
            ActivePane::EditBookmark => view,
            ActivePane::Confirm => view,
        };

        match view {
            ActivePane::Help => None,
            ActivePane::List => None,
            ActivePane::TagsList => {
                if self.tag_items.items.is_empty() {
                    Some(Command::FetchTags)
                } else {
                    None
                }
            }
            ActivePane::TagSearchInput => None,
            ActivePane::SearchInput => None,
            ActivePane::EditBookmark => None,
            ActivePane::Confirm => None,
        }
    }

    pub(super) fn go_back_or_quit(&mut self) {
        if self.terminal_too_small {
            self.running_state = RunningState::Done;
            return;
        }

        match self.active_pane {
            ActivePane::List => self.running_state = RunningState::Done,
            ActivePane::Help => self.active_pane = ActivePane::List,
            ActivePane::SearchInput => {
                self.search_input.reset();
                self.active_pane = ActivePane::List;
            }
            ActivePane::TagSearchInput => {
                self.cancel_tag_search();
            }
            ActivePane::EditBookmark => {
                self.cancel_edit();
            }
            ActivePane::Confirm => {
                self.cancel_confirm();
            }
            ActivePane::TagsList => {
                if self.bookmark_items.items.is_empty() {
                    self.running_state = RunningState::Done;
                } else {
                    self.active_pane = ActivePane::List;
                }
            }
        };
    }

    pub(super) fn filter_tags(&mut self) {
        let query = self.tag_search_input.value().trim().to_lowercase();

        let filtered: Vec<TagStats> = if query.is_empty() {
            self.all_tag_items.clone()
        } else {
            self.all_tag_items
                .iter()
                .filter(|t| t.name.to_lowercase().contains(&query))
                .cloned()
                .collect()
        };

        self.tag_items = TagItems::from(filtered);
    }

    pub(super) fn cancel_tag_search(&mut self) {
        self.tag_search_input.reset();
        self.tag_items = TagItems::from(self.all_tag_items.clone());
        self.active_pane = ActivePane::TagsList;
    }

    pub(super) fn confirm_tag_search(&mut self) {
        self.active_pane = ActivePane::TagsList;
    }

    pub(super) fn get_cmd_to_open_selection_in_browser(&self) -> Option<Command> {
        let url = match self.bookmark_items.state.selected() {
            Some(i) => match self.bookmark_items.items.get(i) {
                Some(bi) => bi.bookmark.uri.clone(),
                None => return None,
            },
            None => return None,
        };

        Some(Command::OpenInBrowser(url))
    }

    pub(super) fn request_open_all_in_browser(&mut self) -> Option<Command> {
        if self.active_pane != ActivePane::List {
            return None;
        }

        let uris: Vec<String> = self
            .bookmark_items
            .items
            .iter()
            .map(|bi| bi.bookmark.uri.clone())
            .collect();

        if uris.is_empty() {
            return None;
        }

        if uris.len() > MAX_BULK_OPEN_LINKS {
            self.pending_confirmation = Some(PendingConfirmation::TooManyLinksWarning(uris.len()));
            self.pane_before_confirm = ActivePane::List;
            self.active_pane = ActivePane::Confirm;
            None
        } else {
            Some(Command::OpenMultipleInBrowser(uris))
        }
    }

    pub(super) fn get_uri_under_cursor(&self) -> Option<String> {
        if let ActivePane::List = self.active_pane {
            self.bookmark_items
                .state
                .selected()
                .and_then(|i| self.bookmark_items.items.get(i))
                .map(|bi| bi.bookmark.uri.clone())
        } else {
            None
        }
    }

    //-------------------------------//
    //  delete (with confirmation)   //
    //-------------------------------//

    pub(super) fn request_delete_selected_bookmark(&mut self) {
        if self.active_pane != ActivePane::List {
            return;
        }

        let uri = self
            .bookmark_items
            .state
            .selected()
            .and_then(|i| self.bookmark_items.items.get(i))
            .map(|item| item.bookmark.uri.clone());

        if let Some(uri) = uri {
            self.pending_confirmation = Some(PendingConfirmation::DeleteBookmark(uri));
            self.pane_before_confirm = ActivePane::List;
            self.active_pane = ActivePane::Confirm;
        }
    }

    pub(super) fn remove_bookmark_by_uri(&mut self, uri: &str) {
        if let Some(pos) = self
            .bookmark_items
            .items
            .iter()
            .position(|item| item.bookmark.uri == uri)
        {
            self.bookmark_items.items.remove(pos);

            let len = self.bookmark_items.items.len();
            if len == 0 {
                self.bookmark_items.state.select(None);
            } else if let Some(selected) = self.bookmark_items.state.selected()
                && selected >= len
            {
                self.bookmark_items.state.select(Some(len - 1));
            }
        }
    }

    //-------------------------------//
    //  editing a bookmark            //
    //-------------------------------//

    pub(super) fn start_edit_selected_bookmark(&mut self) {
        if self.active_pane != ActivePane::List {
            return;
        }

        let details: Option<(String, Option<String>, Option<String>)> =
            self.bookmark_items.state.selected().and_then(|i| {
                self.bookmark_items.items.get(i).map(|item| {
                    (
                        item.bookmark.uri.clone(),
                        item.bookmark.title.clone(),
                        item.bookmark.tags.clone(),
                    )
                })
            });

        if let Some((uri, title, tags)) = details {
            self.edit_uri = uri;
            self.edit_title_input = Input::new(title.clone().unwrap_or_default());
            self.edit_tags_input = Input::new(tags.clone().unwrap_or_default());
            self.edit_original_title = title;
            self.edit_original_tags = tags;
            self.edit_focus = EditField::Title;
            self.active_pane = ActivePane::EditBookmark;
        }
    }

    pub(super) fn edit_focus_next(&mut self) {
        self.edit_focus = match self.edit_focus {
            EditField::Title => EditField::Tags,
            EditField::Tags => EditField::Title,
        };
    }

    pub(super) fn edit_focus_previous(&mut self) {
        // there are only two fields, so moving to the "previous" one is the
        // same as moving to the "next" one
        self.edit_focus_next();
    }

    pub(super) fn edit_has_changes(&self) -> bool {
        let title_now = self.edit_title_input.value().trim();
        let tags_now = self.edit_tags_input.value().trim();

        let title_before = self.edit_original_title.as_deref().unwrap_or("").trim();
        let tags_before = self.edit_original_tags.as_deref().unwrap_or("").trim();

        title_now != title_before || tags_now != tags_before
    }

    pub(super) fn cancel_edit(&mut self) {
        self.edit_title_input.reset();
        self.edit_tags_input.reset();
        self.edit_uri.clear();
        self.edit_original_title = None;
        self.edit_original_tags = None;
        self.edit_focus = EditField::Title;
        self.active_pane = ActivePane::List;
    }

    pub(super) fn request_save_edit(&mut self) {
        if self.active_pane != ActivePane::EditBookmark {
            return;
        }

        if !self.edit_has_changes() {
            self.user_message = Some(UserMessage::info("nothing to save").with_frames_left(1));
            return;
        }

        self.pending_confirmation = Some(PendingConfirmation::SaveEdit);
        self.pane_before_confirm = ActivePane::EditBookmark;
        self.active_pane = ActivePane::Confirm;
    }

    pub(super) fn request_exit_edit(&mut self) {
        if self.active_pane != ActivePane::EditBookmark {
            return;
        }

        if self.edit_has_changes() {
            self.pending_confirmation = Some(PendingConfirmation::DiscardEdit);
            self.pane_before_confirm = ActivePane::EditBookmark;
            self.active_pane = ActivePane::Confirm;
        } else {
            self.cancel_edit();
        }
    }

    pub(super) fn apply_bookmark_edit_locally(&mut self, title: Option<String>, tags: Option<String>) {
        let target_uri = self.edit_uri.clone();
        if let Some(item) = self
            .bookmark_items
            .items
            .iter_mut()
            .find(|item| item.bookmark.uri == target_uri)
        {
            item.bookmark.title = title;
            item.bookmark.tags = tags;
        }
    }

    //-------------------------------//
    //  generic confirmation dialog  //
    //-------------------------------//

    pub(super) fn cancel_confirm(&mut self) {
        self.pending_confirmation = None;
        self.active_pane = self.pane_before_confirm;
    }

    pub(super) fn confirm_message(&self) -> String {
        match &self.pending_confirmation {
            Some(PendingConfirmation::DeleteBookmark(uri)) => {
                format!("delete this bookmark?\n\n{uri}")
            }
            Some(PendingConfirmation::SaveEdit) => "save changes to this bookmark?".to_string(),
            Some(PendingConfirmation::DiscardEdit) => {
                "discard unsaved changes to this bookmark?".to_string()
            }
            Some(PendingConfirmation::TooManyLinksWarning(count)) => {
                format!(
                    "Total links are {count}, which is more than {MAX_BULK_OPEN_LINKS}.\nOpening this many links at once could cause problems with your browser.\n\nPlease narrow your search/results and try again."
                )
            }
            None => String::new(),
        }
    }
}
