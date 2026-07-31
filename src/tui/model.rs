use super::{commands::Command, common::*, message::Message};
use crate::{
    domain::{SavedBookmark, TagStats},
    persistence::SearchTerms,
};
use ratatui::{
    style::Style,
    text::Line,
    widgets::{ListItem, ListState, TableState},
};
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;
use tui_input::Input;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum RunningState {
    #[default]
    Running,
    Done,
}

/// The list of "modes" shown by the Alt+m mode switcher. Each one maps to
/// the exact same [`Message`] that its existing single-key shortcut already
/// sends, so selecting one from the switcher behaves identically to typing
/// that shortcut from the list view - no new/duplicate transition logic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ModeOption {
    AllBookmarks,
    Search,
    Tags,
    Duplicates,
    Starred,
    GlobalSearch,
    NoteSearch,
    Databases,
    Help,
}

impl ModeOption {
    pub(super) const ALL: [ModeOption; 9] = [
        ModeOption::AllBookmarks,
        ModeOption::Search,
        ModeOption::Tags,
        ModeOption::Duplicates,
        ModeOption::Starred,
        ModeOption::GlobalSearch,
        ModeOption::NoteSearch,
        ModeOption::Databases,
        ModeOption::Help,
    ];

    pub(super) fn label(&self) -> &'static str {
        match self {
            ModeOption::AllBookmarks => "all bookmarks",
            ModeOption::Search => "search this database",
            ModeOption::Tags => "browse by tag",
            ModeOption::Duplicates => "duplicate bookmarks",
            ModeOption::Starred => "starred bookmarks",
            ModeOption::GlobalSearch => "search across all databases",
            ModeOption::NoteSearch => "search notes",
            ModeOption::Databases => "switch database",
            ModeOption::Help => "help",
        }
    }

    pub(super) fn key_hint(&self) -> &'static str {
        match self {
            ModeOption::AllBookmarks => "",
            ModeOption::Search => "s",
            ModeOption::Tags => "t",
            ModeOption::Duplicates => "d",
            ModeOption::Starred => "S",
            ModeOption::GlobalSearch => "z",
            ModeOption::NoteSearch => "Alt+n",
            ModeOption::Databases => "A",
            ModeOption::Help => "?",
        }
    }

    pub(super) fn into_message(self) -> Message {
        match self {
            ModeOption::AllBookmarks => Message::ShowAllBookmarks,
            ModeOption::Search => Message::ShowView(ActivePane::SearchInput),
            ModeOption::Tags => Message::ShowView(ActivePane::TagsList),
            ModeOption::Duplicates => Message::ShowDuplicates,
            ModeOption::Starred => Message::ShowStarred,
            ModeOption::GlobalSearch => Message::ShowGlobalSearch,
            ModeOption::NoteSearch => Message::ToggleNoteSearch,
            ModeOption::Databases => Message::ShowDatabaseList,
            ModeOption::Help => Message::ShowView(ActivePane::Help),
        }
    }
}

#[derive(Debug)]
pub(crate) struct BookmarkItem {
    pub(crate) bookmark: SavedBookmark,
    pub(crate) status: bool,
    pub(crate) starred: bool,
    pub(crate) source_db: Option<(String, String)>,
    /// Whether this bookmark has a note attached. `None` means it hasn't
    /// been checked yet - fetched lazily (only for the selected item, one
    /// bookmark at a time) rather than upfront for the whole list, to keep
    /// list loads cheap.
    pub(crate) has_note: Option<bool>,
    /// Whether this bookmark is marked for a bulk "move to another
    /// database" operation (toggled with Space, acted on with `M`).
    pub(crate) marked: bool,
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
    pub(super) fn default() -> Self {
        let state = ListState::default().with_selected(None);

        Self {
            items: vec![],
            state,
        }
    }
}

impl TagItems {
    pub(super) fn default() -> Self {
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

impl From<Vec<BookmarkItem>> for BookmarkItems {
    fn from(items: Vec<BookmarkItem>) -> Self {
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
        Self {
            bookmark,
            status,
            starred: false,
            source_db: None,
            has_note: None,
            marked: false,
        }
    }

    pub(super) fn with_source_db(bookmark: SavedBookmark, name: String, path: String) -> Self {
        Self {
            bookmark,
            status: false,
            starred: false,
            source_db: Some((name, path)),
            has_note: None,
            marked: false,
        }
    }
}

impl From<&BookmarkItem> for ListItem<'_> {
    fn from(value: &BookmarkItem) -> Self {
        let mark_prefix = if value.marked { "[x] " } else { "" };
        let star_prefix = if value.starred { "\u{2605} " } else { "" };
        let db_suffix = match &value.source_db {
            Some((name, _)) => format!("  [{name}]"),
            None => String::new(),
        };
        let line = match value.status {
            false => Line::from(format!(
                "{mark_prefix}{star_prefix}{}{db_suffix}",
                value.bookmark.uri
            )),
            true => Line::styled(
                format!(
                    "> {mark_prefix}{star_prefix}{}{db_suffix}",
                    value.bookmark.uri
                ),
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
    DeleteBookmark(String, Option<String>),
    SaveEdit,
    DiscardEdit,
    SaveNote,
    DiscardNote,
    DeleteNote(String),
    TooManyLinksWarning(usize),
    MoveBookmarks {
        items: Vec<(String, Option<String>)>,
        target_db_path: String,
        target_display_name: String,
        skipped: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum NoteAction {
    Edit,
    Delete,
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
    pub(super) edit_uri_input: Input,
    pub(super) edit_original_uri: String,
    pub(super) edit_uri_editable: bool,
    pub(super) edit_is_new: bool,
    pub(super) edit_target_db_path: Option<String>,
    pub(super) edit_title_input: Input,
    pub(super) edit_tags_input: Input,
    pub(super) edit_focus: EditField,
    /// Tag names matching whatever's currently being typed after the last
    /// comma in `edit_tags_input` (empty when there's nothing to suggest,
    /// e.g. the fragment being typed is empty or matches nothing).
    pub(super) tag_suggestions: Vec<String>,
    /// Index into `tag_suggestions` that's currently highlighted (via
    /// Up/Down), if any.
    pub(super) tag_suggestion_selected: Option<usize>,
    pub(super) edit_original_title: Option<String>,
    pub(super) edit_original_tags: Option<String>,
    pub(super) viewing_duplicates: bool,
    pub(super) note_uri: String,
    pub(super) note_input: Input,
    pub(super) note_original: Option<String>,
    pub(super) note_action: NoteAction,
    pub(super) starred_uris: HashSet<String>,
    pub(super) showing_starred: bool,
    pub(super) active_db_name: String,
    pub(super) available_dbs: Vec<String>,
    /// The subset of `available_dbs` currently shown/selectable in the
    /// Database List view - equal to `available_dbs` unless a database
    /// search (`/`) is narrowing it down.
    pub(super) filtered_dbs: Vec<String>,
    pub(super) db_list_state: ListState,
    pub(super) db_search_input: Input,
    /// Whether the database list is currently being used to switch the
    /// active database, or to pick a destination for a bookmark move.
    pub(super) db_list_purpose: DbListPurpose,
    pub(super) new_db_name_input: Input,
    pub(super) global_search_mode: bool,
    pub(super) note_search_mode: bool,
    pub(super) mode_switcher_state: ListState,
    pub(super) pane_before_mode_switch: ActivePane,
    /// Bookmarks marked (Space) for a bulk move-to-database operation (`M`).
    pub(super) marked_uris: HashSet<String>,
    /// The bookmark(s) queued for the move currently in progress (picked
    /// via `m`/`M`), paired with the path of the database they currently
    /// live in (`None` means the active database).
    pub(super) move_pending: Vec<(String, Option<String>)>,
    /// Uris of the move that was just confirmed and sent off as a command -
    /// used to remove them from the list locally once the move succeeds.
    pub(super) move_last_uris: Vec<String>,
    /// Scroll/selection state for the Help view's tables - reused purely
    /// for scrolling (nothing is ever visibly "selected"), the same way
    /// `ListState` is used to scroll the other list-based views.
    pub(super) help_table_state: TableState,
}

/// Splits a tags-input value like "foo, bar, ba" into the already-
/// completed tags before the last comma (trimmed, original casing kept -
/// `["foo", "bar"]`) and the in-progress fragment after it (trimmed -
/// `"ba"`). A value with no comma at all is treated as a single
/// in-progress fragment (no completed tags yet).
fn split_tags_input(tags_value: &str) -> (Vec<String>, String) {
    let mut parts: Vec<&str> = tags_value.split(',').collect();
    let fragment = parts.pop().unwrap_or("").trim().to_string();
    let completed: Vec<String> = parts
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    (completed, fragment)
}

impl Model {
    pub(crate) fn default(
        pool: &Pool<Sqlite>,
        context: TuiContext,
        terminal_dimensions: TerminalDimensions,
        db_name: String,
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
            edit_uri_input: Input::default(),
            edit_original_uri: String::new(),
            edit_uri_editable: false,
            edit_is_new: false,
            edit_target_db_path: None,
            edit_title_input: Input::default(),
            edit_tags_input: Input::default(),
            edit_focus: EditField::Title,
            tag_suggestions: vec![],
            tag_suggestion_selected: None,
            edit_original_title: None,
            edit_original_tags: None,
            viewing_duplicates: false,
            note_uri: String::new(),
            note_input: Input::default(),
            note_original: None,
            note_action: NoteAction::Edit,
            starred_uris: HashSet::new(),
            showing_starred: false,
            active_db_name: db_name,
            available_dbs: vec![],
            filtered_dbs: vec![],
            db_list_state: ListState::default(),
            db_search_input: Input::default(),
            db_list_purpose: DbListPurpose::Switch,
            new_db_name_input: Input::default(),
            global_search_mode: false,
            note_search_mode: false,
            mode_switcher_state: ListState::default(),
            pane_before_mode_switch: active_pane,
            marked_uris: HashSet::new(),
            move_pending: vec![],
            move_last_uris: vec![],
            help_table_state: TableState::default(),
        }
    }

    pub(super) fn select_next_list_item(&mut self) {
        match self.active_pane {
            ActivePane::List => self.bookmark_items.state.select_next(),
            ActivePane::TagsList | ActivePane::TagSearchInput => {
                self.tag_items.state.select_next()
            }
            ActivePane::DatabaseList | ActivePane::DatabaseSearchInput => {
                self.db_list_state.select_next()
            }
            ActivePane::ModeSwitcher => self.mode_switcher_state.select_next(),
            ActivePane::SearchInput => {}
            ActivePane::EditBookmark => {}
            ActivePane::Notes => {}
            ActivePane::NewDatabaseName => {}
            ActivePane::Confirm => {}
            ActivePane::Help => self.help_table_state.select_next(),
        }
    }

    pub(super) fn select_previous_list_item(&mut self) {
        match self.active_pane {
            ActivePane::List => self.bookmark_items.state.select_previous(),
            ActivePane::TagsList | ActivePane::TagSearchInput => {
                self.tag_items.state.select_previous()
            }
            ActivePane::DatabaseList | ActivePane::DatabaseSearchInput => {
                self.db_list_state.select_previous()
            }
            ActivePane::ModeSwitcher => self.mode_switcher_state.select_previous(),
            ActivePane::SearchInput => {}
            ActivePane::EditBookmark => {}
            ActivePane::Notes => {}
            ActivePane::NewDatabaseName => {}
            ActivePane::Confirm => {}
            ActivePane::Help => self.help_table_state.select_previous(),
        }
    }

    pub(super) fn select_first_list_item(&mut self) {
        match self.active_pane {
            ActivePane::List => self.bookmark_items.state.select_first(),
            ActivePane::TagsList | ActivePane::TagSearchInput => {
                self.tag_items.state.select_first()
            }
            ActivePane::DatabaseList | ActivePane::DatabaseSearchInput => {
                self.db_list_state.select_first()
            }
            ActivePane::ModeSwitcher => self.mode_switcher_state.select_first(),
            ActivePane::SearchInput => {}
            ActivePane::EditBookmark => {}
            ActivePane::Notes => {}
            ActivePane::NewDatabaseName => {}
            ActivePane::Confirm => {}
            ActivePane::Help => self.help_table_state.select_first(),
        }
    }
    pub(super) fn select_last_list_item(&mut self) {
        match self.active_pane {
            ActivePane::List => self.bookmark_items.state.select_last(),
            ActivePane::TagsList | ActivePane::TagSearchInput => {
                self.tag_items.state.select_last()
            }
            ActivePane::DatabaseList | ActivePane::DatabaseSearchInput => {
                self.db_list_state.select_last()
            }
            ActivePane::ModeSwitcher => self.mode_switcher_state.select_last(),
            ActivePane::SearchInput => {}
            ActivePane::EditBookmark => {}
            ActivePane::Notes => {}
            ActivePane::NewDatabaseName => {}
            ActivePane::Confirm => {}
            ActivePane::Help => self.help_table_state.select_last(),
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
            ActivePane::Notes => view,
            ActivePane::DatabaseList => view,
            ActivePane::DatabaseSearchInput => view,
            ActivePane::NewDatabaseName => view,
            ActivePane::Confirm => view,
            ActivePane::ModeSwitcher => view,
        };

        match view {
            ActivePane::Help => {
                self.help_table_state = TableState::default();
                None
            }
            ActivePane::List => None,
            ActivePane::TagsList => {
                // Always refetch, rather than only when the cached list is
                // empty - otherwise deleting/editing/moving a bookmark
                // (which can change tag counts, or make a tag disappear
                // entirely) wouldn't be reflected here until the list
                // happened to be empty again (e.g. after restarting bmm).
                Some(Command::FetchTags)
            }
            ActivePane::TagSearchInput => None,
            ActivePane::SearchInput => {
                // this is the plain "s" entry point into search - make sure
                // no leftover global/note-search mode from a previous
                // session carries over into what looks like a normal search
                self.global_search_mode = false;
                self.note_search_mode = false;
                None
            }
            ActivePane::EditBookmark => None,
            ActivePane::Notes => None,
            ActivePane::DatabaseList => None,
            ActivePane::DatabaseSearchInput => None,
            ActivePane::NewDatabaseName => None,
            ActivePane::Confirm => None,
            ActivePane::ModeSwitcher => None,
        }
    }

    pub(super) fn go_back_or_quit(&mut self) -> Option<Command> {
        if self.terminal_too_small {
            self.running_state = RunningState::Done;
            return None;
        }

        match self.active_pane {
            ActivePane::List => self.running_state = RunningState::Done,
            ActivePane::Help => self.active_pane = ActivePane::List,
            ActivePane::SearchInput => {
                self.search_input.reset();
                self.initial = false;
                self.active_pane = ActivePane::List;

                // Live search may have left the list showing filtered
                // results from a query that was never confirmed with
                // Enter - restore the unfiltered view on cancel, same
                // as what an empty confirmed search would show.
                let was_global = self.global_search_mode;
                self.global_search_mode = false;
                self.note_search_mode = false;

                return Some(if was_global {
                    Command::GlobalSearch(None)
                } else {
                    Command::FetchAllBookmarks
                });
            }
            ActivePane::TagSearchInput => {
                self.cancel_tag_search();
            }
            ActivePane::EditBookmark => {
                self.cancel_edit();
            }
            ActivePane::Notes => {
                self.cancel_note_edit();
            }
            ActivePane::DatabaseList => {
                self.move_pending.clear();
                self.db_list_purpose = DbListPurpose::Switch;
                self.active_pane = ActivePane::List;
            }
            ActivePane::DatabaseSearchInput => {
                self.cancel_database_search();
            }
            ActivePane::NewDatabaseName => {
                self.new_db_name_input.reset();
                self.active_pane = ActivePane::List;
            }
            ActivePane::Confirm => {
                self.cancel_confirm();
            }
            ActivePane::TagsList => {
                self.active_pane = ActivePane::List;
            }
            ActivePane::ModeSwitcher => {
                self.active_pane = self.pane_before_mode_switch;
            }
        };

        None
    }

    /// Builds the command to re-run the search with whatever is currently
    /// typed into the search box, so results update live as the user types
    /// instead of waiting for Enter. Returns `None` when the current text
    /// can't form a valid query yet (eg. too many terms) - in that case the
    /// list just keeps showing its last valid results until the input
    /// becomes valid again.
    pub(super) fn live_search_command(&self) -> Option<Command> {
        let query = self.search_input.value();

        if query.trim().is_empty() {
            return Some(if self.global_search_mode {
                Command::GlobalSearch(None)
            } else if self.note_search_mode {
                Command::SearchNotes(None)
            } else {
                Command::FetchAllBookmarks
            });
        }

        match SearchTerms::try_from(query) {
            Ok(terms) => Some(if self.global_search_mode {
                Command::GlobalSearch(Some(terms))
            } else if self.note_search_mode {
                Command::SearchNotes(Some(terms))
            } else {
                Command::SearchBookmarks(terms)
            }),
            Err(_) => None,
        }
    }

    /// If a bookmark is selected in the List pane and we don't yet know
    /// whether it has a note, builds the command to check - so the details
    /// box can show a "Notes" line. Only ever checks one bookmark (the
    /// selected one) at a time, and only once per bookmark, to keep this
    /// cheap.
    pub(super) fn maybe_fetch_note_existence_for_selected(&self) -> Option<Command> {
        if self.active_pane != ActivePane::List {
            return None;
        }

        let index = self.bookmark_items.state.selected()?;
        let item = self.bookmark_items.items.get(index)?;

        if item.has_note.is_some() {
            return None;
        }

        Some(Command::FetchNoteExists(item.bookmark.uri.clone()))
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
        self.active_pane = ActivePane::List;
    }

    pub(super) fn confirm_tag_search(&mut self) {
        self.active_pane = ActivePane::TagsList;
    }

    pub(super) fn get_cmd_to_open_selection_in_browser(&self, incognito: bool) -> Option<Command> {
        let url = match self.bookmark_items.state.selected() {
            Some(i) => match self.bookmark_items.items.get(i) {
                Some(bi) => bi.bookmark.uri.clone(),
                None => return None,
            },
            None => return None,
        };

        if incognito {
            Some(Command::OpenInBrowserIncognito(url))
        } else {
            Some(Command::OpenInBrowser(url))
        }
    }

    pub(super) fn request_open_all_in_browser(&mut self, incognito: bool) -> Option<Command> {
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
        } else if incognito {
            Some(Command::OpenMultipleInBrowserIncognito(uris))
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
            let target_db_path = self.get_selected_source_db_path();
            self.pending_confirmation =
                Some(PendingConfirmation::DeleteBookmark(uri, target_db_path));
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

    pub(super) fn start_add_new_bookmark(&mut self) {
        if self.active_pane != ActivePane::List {
            return;
        }

        self.edit_uri_input = Input::default();
        self.edit_original_uri = String::new();
        self.edit_uri_editable = true;
        self.edit_is_new = true;
        self.edit_target_db_path = None;
        self.edit_title_input = Input::default();
        self.edit_tags_input = Input::default();
        self.edit_original_title = None;
        self.edit_original_tags = None;
        self.edit_focus = EditField::Uri;
        self.dismiss_tag_suggestions();
        self.active_pane = ActivePane::EditBookmark;
    }

    pub(super) fn start_edit_selected_bookmark(&mut self, uri_editable: bool) {
        if self.active_pane != ActivePane::List {
            return;
        }

        let details: Option<(String, Option<String>, Option<String>, Option<String>)> =
            self.bookmark_items.state.selected().and_then(|i| {
                self.bookmark_items.items.get(i).map(|item| {
                    (
                        item.bookmark.uri.clone(),
                        item.bookmark.title.clone(),
                        item.bookmark.tags.clone(),
                        item.source_db.as_ref().map(|(_, path)| path.clone()),
                    )
                })
            });

        if let Some((uri, title, tags, target_db_path)) = details {
            self.edit_uri_input = Input::new(uri.clone());
            self.edit_original_uri = uri;
            self.edit_uri_editable = uri_editable;
            self.edit_is_new = false;
            self.edit_target_db_path = target_db_path;
            self.edit_title_input = Input::new(title.clone().unwrap_or_default());
            self.edit_tags_input = Input::new(tags.clone().unwrap_or_default());
            self.edit_original_title = title;
            self.edit_original_tags = tags;
            self.edit_focus = EditField::Title;
            self.dismiss_tag_suggestions();
            self.active_pane = ActivePane::EditBookmark;
        }
    }

    pub(super) fn edit_focus_next(&mut self) {
        self.dismiss_tag_suggestions();
        self.edit_focus = match self.edit_focus {
            EditField::Uri => EditField::Title,
            EditField::Title => EditField::Tags,
            EditField::Tags => {
                if self.edit_uri_editable {
                    EditField::Uri
                } else {
                    EditField::Title
                }
            }
        };
    }

    pub(super) fn edit_focus_previous(&mut self) {
        self.dismiss_tag_suggestions();
        self.edit_focus = match self.edit_focus {
            EditField::Title => {
                if self.edit_uri_editable {
                    EditField::Uri
                } else {
                    EditField::Tags
                }
            }
            EditField::Tags => EditField::Title,
            EditField::Uri => EditField::Tags,
        };
    }

    pub(super) fn edit_has_changes(&self) -> bool {
        if self.edit_is_new {
            return !self.edit_uri_input.value().trim().is_empty();
        }

        let title_now = self.edit_title_input.value().trim();
        let tags_now = self.edit_tags_input.value().trim();
        let uri_now = self.edit_uri_input.value().trim();

        let title_before = self.edit_original_title.as_deref().unwrap_or("").trim();
        let tags_before = self.edit_original_tags.as_deref().unwrap_or("").trim();
        let uri_before = self.edit_original_uri.trim();

        title_now != title_before || tags_now != tags_before || uri_now != uri_before
    }

    //-------------------------------//
    //  tag autocomplete suggestions  //
    //-------------------------------//

    /// Recomputes `tag_suggestions` from whatever's currently being typed
    /// after the last comma in the tags input - called after every
    /// keystroke while the Tags field is focused, so suggestions stay
    /// live as the user types (matching bmm's existing already-saved
    /// tags, via `all_tag_items`, which is kept fresh by `Command::FetchTags`).
    pub(super) fn recompute_tag_suggestions(&mut self) {
        let (completed, fragment) = split_tags_input(self.edit_tags_input.value());

        if fragment.is_empty() {
            self.dismiss_tag_suggestions();
            return;
        }

        let fragment_lower = fragment.to_lowercase();
        let completed_lower: HashSet<String> =
            completed.iter().map(|t| t.to_lowercase()).collect();

        let mut matches: Vec<String> = self
            .all_tag_items
            .iter()
            .map(|t| t.name.clone())
            .filter(|name| {
                let name_lower = name.to_lowercase();
                name_lower.starts_with(&fragment_lower)
                    && name_lower != fragment_lower
                    && !completed_lower.contains(&name_lower)
            })
            .collect();

        matches.sort();
        matches.truncate(MAX_TAG_SUGGESTIONS);

        self.tag_suggestion_selected = if matches.is_empty() { None } else { Some(0) };
        self.tag_suggestions = matches;
    }

    pub(super) fn tag_suggestion_next(&mut self) {
        if self.tag_suggestions.is_empty() {
            return;
        }

        let len = self.tag_suggestions.len();
        self.tag_suggestion_selected = Some(match self.tag_suggestion_selected {
            Some(i) => (i + 1) % len,
            None => 0,
        });
    }

    pub(super) fn tag_suggestion_previous(&mut self) {
        if self.tag_suggestions.is_empty() {
            return;
        }

        let len = self.tag_suggestions.len();
        self.tag_suggestion_selected = Some(match self.tag_suggestion_selected {
            Some(0) | None => len - 1,
            Some(i) => i - 1,
        });
    }

    pub(super) fn dismiss_tag_suggestions(&mut self) {
        self.tag_suggestions.clear();
        self.tag_suggestion_selected = None;
    }

    /// Replaces the in-progress fragment (after the last comma) in the
    /// tags input with the currently-highlighted suggestion, followed by
    /// ", " so the cursor's ready for the next tag - then clears the
    /// suggestion list, since the fragment that drove it is gone.
    pub(super) fn accept_selected_tag_suggestion(&mut self) {
        let Some(i) = self.tag_suggestion_selected else {
            return;
        };
        let Some(chosen) = self.tag_suggestions.get(i).cloned() else {
            return;
        };

        let (mut completed, _fragment) = split_tags_input(self.edit_tags_input.value());
        completed.push(chosen);

        let new_value = format!("{}, ", completed.join(", "));
        self.edit_tags_input = Input::new(new_value);
        self.dismiss_tag_suggestions();
    }

    pub(super) fn cancel_edit(&mut self) {
        self.edit_title_input.reset();
        self.edit_tags_input.reset();
        self.edit_uri_input.reset();
        self.edit_original_uri.clear();
        self.edit_uri_editable = false;
        self.edit_is_new = false;
        self.edit_target_db_path = None;
        self.edit_original_title = None;
        self.edit_original_tags = None;
        self.edit_focus = EditField::Title;
        self.dismiss_tag_suggestions();
        self.active_pane = ActivePane::List;
    }

    pub(super) fn request_save_edit(&mut self) {
        if self.active_pane != ActivePane::EditBookmark {
            return;
        }

        if self.edit_is_new && self.edit_uri_input.value().trim().is_empty() {
            self.user_message = Some(UserMessage::error("uri can't be empty"));
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

    pub(super) fn apply_bookmark_edit_locally(
        &mut self,
        new_uri: Option<String>,
        title: Option<String>,
        tags: Option<String>,
    ) {
        if self.edit_is_new {
            let uri = new_uri.unwrap_or_else(|| self.edit_uri_input.value().trim().to_string());
            let bookmark = SavedBookmark { uri, title, tags };
            self.bookmark_items
                .items
                .insert(0, BookmarkItem::new(bookmark, false));
            self.bookmark_items.state.select(Some(0));
            self.sync_starred_markers();
            return;
        }

        let target_uri = self.edit_original_uri.clone();
        if let Some(item) = self
            .bookmark_items
            .items
            .iter_mut()
            .find(|item| item.bookmark.uri == target_uri)
        {
            if let Some(new_uri) = new_uri {
                item.bookmark.uri = new_uri;
            }
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

    //-------------------------------//
    //  mode switcher (Alt+m)         //
    //-------------------------------//

    /// Opens the mode switcher overlay, remembering the pane it was opened
    /// from so Esc/q can return to it unchanged. Safe to call from any
    /// pane, including the mode switcher itself (in which case it's a
    /// no-op - the caller is expected to handle "already open" as a
    /// toggle-close instead, see `Message::ToggleModeSwitcher`).
    pub(super) fn open_mode_switcher(&mut self) {
        if self.active_pane == ActivePane::ModeSwitcher {
            return;
        }
        self.pane_before_mode_switch = self.active_pane;
        self.mode_switcher_state.select(Some(0));
        self.active_pane = ActivePane::ModeSwitcher;
    }

    //-------------------------------//
    //  starred bookmarks             //
    //-------------------------------//

    /// Refreshes each visible item's star/mark markers based on
    /// `starred_uris`/`marked_uris`. Call this any time `bookmark_items` is
    /// (re)populated, or either of those sets changes.
    pub(super) fn sync_starred_markers(&mut self) {
        for item in self.bookmark_items.items.iter_mut() {
            item.starred = self.starred_uris.contains(&item.bookmark.uri);
            item.marked = self.marked_uris.contains(&item.bookmark.uri);
        }
    }

    pub(super) fn request_toggle_star(&mut self) -> Option<Command> {
        let uri = self.get_selected_bookmark_uri()?;
        Some(Command::ToggleStar(uri))
    }

    pub(super) fn apply_star_toggle(&mut self, uri: &str, starred: bool) {
        if starred {
            self.starred_uris.insert(uri.to_string());
        } else {
            self.starred_uris.remove(uri);

            // if we're currently only showing starred bookmarks, an item
            // that just got unstarred should disappear from view
            if self.showing_starred {
                self.remove_bookmark_by_uri(uri);
            }
        }

        self.sync_starred_markers();
    }

    //-------------------------------//
    //  marking bookmarks for move    //
    //-------------------------------//

    /// Toggles whether the bookmark under cursor in the List pane is
    /// marked for a bulk move (`M`).
    pub(super) fn toggle_mark_for_move(&mut self) {
        let Some(uri) = self.get_selected_bookmark_uri() else {
            return;
        };

        if self.marked_uris.contains(&uri) {
            self.marked_uris.remove(&uri);
        } else {
            self.marked_uris.insert(uri);
        }

        self.sync_starred_markers();
    }

    //-------------------------------//
    //  multiple databases            //
    //-------------------------------//

    /// Scans `~/.local/share/bmm/` (bmm's default data directory) for
    /// `.db` files, and populates `available_dbs` with their filenames.
    /// This is a synchronous, local directory listing, so it's done
    /// directly rather than via an async Command.
    pub(super) fn scan_available_databases(&mut self) {
        let mut dbs: Vec<String> = Vec::new();

        if let Ok(data_dir) = crate::utils::get_data_dir() {
            let bmm_dir = data_dir.join("bmm");

            if let Ok(entries) = std::fs::read_dir(&bmm_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("db")
                        && let Some(name) = path.file_name().and_then(|n| n.to_str())
                    {
                        dbs.push(name.to_string());
                    }
                }
            }
        }

        if !dbs.iter().any(|d| d == &self.active_db_name) {
            dbs.push(self.active_db_name.clone());
        }

        dbs.sort();
        self.available_dbs = dbs;
        self.filtered_dbs = self.available_dbs.clone();
    }

    fn show_database_list_for(&mut self, purpose: DbListPurpose) {
        self.scan_available_databases();
        self.db_search_input.reset();
        self.db_list_purpose = purpose;

        let selected_index = self
            .filtered_dbs
            .iter()
            .position(|d| d == &self.active_db_name)
            .unwrap_or(0);

        self.db_list_state.select(Some(selected_index));
        self.active_pane = ActivePane::DatabaseList;
    }

    pub(super) fn show_database_list(&mut self) {
        self.show_database_list_for(DbListPurpose::Switch);
    }

    /// Filters `available_dbs` by whatever's typed into `db_search_input`,
    /// storing the result in `filtered_dbs` - mirrors `filter_tags`.
    pub(super) fn filter_databases(&mut self) {
        let query = self.db_search_input.value().trim().to_lowercase();

        self.filtered_dbs = if query.is_empty() {
            self.available_dbs.clone()
        } else {
            self.available_dbs
                .iter()
                .filter(|d| d.to_lowercase().contains(&query))
                .cloned()
                .collect()
        };

        self.db_list_state
            .select(if self.filtered_dbs.is_empty() {
                None
            } else {
                Some(0)
            });
    }

    pub(super) fn cancel_database_search(&mut self) {
        self.db_search_input.reset();
        self.filtered_dbs = self.available_dbs.clone();
        self.move_pending.clear();
        self.db_list_purpose = DbListPurpose::Switch;
        self.active_pane = ActivePane::List;
    }

    pub(super) fn confirm_database_search(&mut self) {
        self.active_pane = ActivePane::DatabaseList;
    }

    pub(super) fn request_switch_to_selected_database(&mut self) -> Option<Command> {
        let name = self
            .db_list_state
            .selected()
            .and_then(|i| self.filtered_dbs.get(i))?
            .clone();

        if name == self.active_db_name {
            self.user_message =
                Some(UserMessage::info("already using this database").with_frames_left(1));
            return None;
        }

        let data_dir = crate::utils::get_data_dir().ok()?;
        let path = data_dir.join("bmm").join(&name);
        let path = path.to_str()?.to_string();

        Some(Command::SwitchDatabase {
            path,
            display_name: name,
        })
    }

    //-------------------------------//
    //  moving bookmark(s) to another //
    //  database ('m' / 'M')          //
    //-------------------------------//

    /// Queues the bookmark under cursor for a move, and opens the database
    /// picker to choose its destination.
    pub(super) fn start_move_selected_bookmark(&mut self) {
        if self.active_pane != ActivePane::List {
            return;
        }

        let Some(index) = self.bookmark_items.state.selected() else {
            return;
        };
        let Some(item) = self.bookmark_items.items.get(index) else {
            return;
        };

        let uri = item.bookmark.uri.clone();
        let source_db_path = item.source_db.as_ref().map(|(_, path)| path.clone());

        self.move_pending = vec![(uri, source_db_path)];
        self.show_database_list_for(DbListPurpose::Move);
    }

    /// Queues every marked bookmark for a move, and opens the database
    /// picker to choose their destination. Shows an info message instead
    /// if nothing is currently marked.
    pub(super) fn start_move_marked_bookmarks(&mut self) {
        if self.active_pane != ActivePane::List {
            return;
        }

        if self.marked_uris.is_empty() {
            self.user_message = Some(
                UserMessage::info("no bookmarks marked - press space to mark bookmarks first")
                    .with_frames_left(2),
            );
            return;
        }

        let items: Vec<(String, Option<String>)> = self
            .bookmark_items
            .items
            .iter()
            .filter(|item| self.marked_uris.contains(&item.bookmark.uri))
            .map(|item| {
                (
                    item.bookmark.uri.clone(),
                    item.source_db.as_ref().map(|(_, path)| path.clone()),
                )
            })
            .collect();

        if items.is_empty() {
            self.user_message = Some(
                UserMessage::info("marked bookmarks aren't in the current list")
                    .with_frames_left(2),
            );
            return;
        }

        self.move_pending = items;
        self.show_database_list_for(DbListPurpose::Move);
    }

    /// Builds the confirmation prompt for the database currently selected
    /// in the picker, skipping any queued bookmark(s) that are already in
    /// that database. Sets up `pending_confirmation` rather than returning
    /// a `Command` directly, since moving is destructive enough to warrant
    /// asking first (same as delete).
    pub(super) fn request_move_to_selected_database(&mut self) {
        let Some(name) = self
            .db_list_state
            .selected()
            .and_then(|i| self.filtered_dbs.get(i))
            .cloned()
        else {
            return;
        };

        let Some(data_dir) = crate::utils::get_data_dir().ok() else {
            self.user_message = Some(UserMessage::error("couldn't determine data directory"));
            return;
        };
        let target_path = data_dir.join("bmm").join(&name);
        let Some(target_path) = target_path.to_str().map(|s| s.to_string()) else {
            self.user_message = Some(UserMessage::error("invalid database path"));
            return;
        };

        let mut skipped = 0usize;
        let items: Vec<(String, Option<String>)> = self
            .move_pending
            .iter()
            .filter(|(_, source_db_path)| {
                let already_there = match source_db_path {
                    Some(path) => path == &target_path,
                    None => name == self.active_db_name,
                };
                if already_there {
                    skipped += 1;
                }
                !already_there
            })
            .cloned()
            .collect();

        if items.is_empty() {
            self.user_message = Some(UserMessage::error("already in this database"));
            return;
        }

        self.pending_confirmation = Some(PendingConfirmation::MoveBookmarks {
            items,
            target_db_path: target_path,
            target_display_name: name,
            skipped,
        });
        self.pane_before_confirm = ActivePane::DatabaseList;
        self.active_pane = ActivePane::Confirm;
    }

    /// Called after a move succeeds: drops the moved bookmarks from the
    /// current list/marks and resets the move-in-progress state.
    pub(super) fn apply_bookmarks_moved(&mut self) {
        for uri in std::mem::take(&mut self.move_last_uris) {
            self.remove_bookmark_by_uri(&uri);
            self.marked_uris.remove(&uri);
        }
        self.move_pending.clear();
        self.db_list_purpose = DbListPurpose::Switch;
        self.sync_starred_markers();
    }

    pub(super) fn start_new_database_name(&mut self) {
        self.new_db_name_input.reset();
        self.active_pane = ActivePane::NewDatabaseName;
    }

    pub(super) fn request_create_database(&mut self) -> Option<Command> {
        let raw = self.new_db_name_input.value().trim();
        if raw.is_empty() {
            self.user_message = Some(UserMessage::error("database name can't be empty"));
            return None;
        }

        let name = if raw.ends_with(".db") {
            raw.to_string()
        } else {
            format!("{raw}.db")
        };

        let data_dir = crate::utils::get_data_dir().ok()?;
        let path = data_dir.join("bmm").join(&name);
        let path_str = path.to_str()?.to_string();

        if path.exists() {
            self.user_message = Some(UserMessage::error("a database with this name already exists"));
            return None;
        }

        self.new_db_name_input.reset();

        Some(Command::SwitchDatabase {
            path: path_str,
            display_name: name,
        })
    }

    pub(super) fn apply_database_switch(&mut self, display_name: String) {
        self.active_db_name = display_name;
        self.active_pane = ActivePane::List;
        self.bookmark_items = BookmarkItems::default();
        self.tag_items = TagItems::default();
        self.all_tag_items = vec![];
        self.starred_uris = HashSet::new();
        self.showing_starred = false;
        self.viewing_duplicates = false;
        self.marked_uris = HashSet::new();
        self.move_pending = vec![];
    }

    pub(super) fn confirm_message(&self) -> String {
        match &self.pending_confirmation {
            Some(PendingConfirmation::DeleteBookmark(uri, target_db_path)) => match target_db_path
            {
                Some(_) => format!("delete this bookmark (from another database)?\n\n{uri}"),
                None => format!("delete this bookmark?\n\n{uri}"),
            },
            Some(PendingConfirmation::SaveEdit) => {
                if self.edit_is_new {
                    let uri = self.edit_uri_input.value().trim();
                    format!("save this new bookmark?\n\n{uri}")
                } else {
                    let new_uri = self.edit_uri_input.value().trim();
                    if self.edit_uri_editable && new_uri != self.edit_original_uri.trim() {
                        format!(
                            "save changes to this bookmark?\n\nuri will change to:\n{new_uri}"
                        )
                    } else {
                        "save changes to this bookmark?".to_string()
                    }
                }
            }
            Some(PendingConfirmation::DiscardEdit) => {
                "discard unsaved changes to this bookmark?".to_string()
            }
            Some(PendingConfirmation::SaveNote) => "save this note?".to_string(),
            Some(PendingConfirmation::DiscardNote) => {
                "discard unsaved changes to this note?".to_string()
            }
            Some(PendingConfirmation::DeleteNote(uri)) => {
                format!("delete the note for this bookmark?\n\n{uri}")
            }
            Some(PendingConfirmation::TooManyLinksWarning(count)) => {
                format!(
                    "Total links are {count}, which is more than {MAX_BULK_OPEN_LINKS}.\nOpening this many links at once could cause problems with your browser.\n\nPlease narrow your search/results and try again."
                )
            }
            Some(PendingConfirmation::MoveBookmarks {
                items,
                target_display_name,
                skipped,
                ..
            }) => {
                let n = items.len();
                let word = if n == 1 { "bookmark" } else { "bookmarks" };
                if *skipped > 0 {
                    format!(
                        "move {n} {word} to \"{target_display_name}\"?\n\n({skipped} already there and will be skipped)"
                    )
                } else {
                    format!("move {n} {word} to \"{target_display_name}\"?")
                }
            }
            None => String::new(),
        }
    }

    //-------------------------------//
    //  notes (hidden, per-bookmark) //
    //-------------------------------//

    /// Returns the uri under cursor if a bookmark is selected in the List
    /// pane; used to kick off fetching that bookmark's note.
    pub(super) fn get_selected_bookmark_uri(&self) -> Option<String> {
        if self.active_pane != ActivePane::List {
            return None;
        }

        self.bookmark_items
            .state
            .selected()
            .and_then(|i| self.bookmark_items.items.get(i))
            .map(|item| item.bookmark.uri.clone())
    }

    /// Returns the full path of the selected item's source database, if
    /// it's a cross-database search result (ie. not from the currently
    /// active database).
    pub(super) fn get_selected_source_db_path(&self) -> Option<String> {
        if self.active_pane != ActivePane::List {
            return None;
        }

        self.bookmark_items
            .state
            .selected()
            .and_then(|i| self.bookmark_items.items.get(i))
            .and_then(|item| item.source_db.as_ref())
            .map(|(_, path)| path.clone())
    }

    pub(super) fn request_delete_note_for_selected(&mut self) -> Option<Command> {
        let uri = self.get_selected_bookmark_uri()?;
        self.note_action = NoteAction::Delete;
        Some(Command::FetchNote(uri))
    }

    pub(super) fn handle_note_fetched(&mut self, uri: String, note: Option<String>) {
        match self.note_action {
            NoteAction::Edit => self.populate_note(uri, note),
            NoteAction::Delete => match note {
                Some(_) => {
                    self.pending_confirmation = Some(PendingConfirmation::DeleteNote(uri));
                    self.pane_before_confirm = ActivePane::List;
                    self.active_pane = ActivePane::Confirm;
                }
                None => {
                    self.user_message =
                        Some(UserMessage::info("no note to delete for this bookmark").with_frames_left(1));
                }
            },
        }
    }

    pub(super) fn populate_note(&mut self, uri: String, note: Option<String>) {
        self.note_uri = uri;
        self.note_input = Input::new(note.clone().unwrap_or_default());
        self.note_original = note;
        self.active_pane = ActivePane::Notes;
    }

    pub(super) fn note_has_changes(&self) -> bool {
        let now = self.note_input.value().trim();
        let before = self.note_original.as_deref().unwrap_or("").trim();
        now != before
    }

    pub(super) fn cancel_note_edit(&mut self) {
        self.note_input.reset();
        self.note_uri.clear();
        self.note_original = None;
        self.active_pane = ActivePane::List;
    }

    pub(super) fn request_save_note(&mut self) {
        if self.active_pane != ActivePane::Notes {
            return;
        }

        if !self.note_has_changes() {
            self.user_message = Some(UserMessage::info("nothing to save").with_frames_left(1));
            return;
        }

        self.pending_confirmation = Some(PendingConfirmation::SaveNote);
        self.pane_before_confirm = ActivePane::Notes;
        self.active_pane = ActivePane::Confirm;
    }

    pub(super) fn request_exit_note(&mut self) {
        if self.active_pane != ActivePane::Notes {
            return;
        }

        if self.note_has_changes() {
            self.pending_confirmation = Some(PendingConfirmation::DiscardNote);
            self.pane_before_confirm = ActivePane::Notes;
            self.active_pane = ActivePane::Confirm;
        } else {
            self.cancel_note_edit();
        }
    }
}
