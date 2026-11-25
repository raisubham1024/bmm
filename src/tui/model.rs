use super::{
    commands::Command,
    common::*,
    db_order::{self, AltNumber},
    folders::{Folders, UNFILED_LABEL},
    message::{DatabaseRenameOutcome, LinkCheckOutcome, Message},
};
use crate::{
    domain::{SavedBookmark, TagStats},
    persistence::{SearchScope, SearchTerms},
};
use ratatui::{
    style::Style,
    text::Line,
    widgets::{ListItem, ListState, TableState},
};
use sqlx::{Pool, Sqlite};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tui_input::Input;

/// How long after the last `Alt+<digit>` a possibly-unfinished database
/// number (eg. the "1" of "12") is applied. Terminals don't tell bmm when
/// Alt is released, so a short pause stands in for "Alt was let go".
const ALT_NUMBER_TIMEOUT: Duration = Duration::from_millis(700);

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
    ScopedSearch,
    Tags,
    Starred,
    GlobalSearch,
    NoteSearch,
    Databases,
    Help,
    Folders,
}

impl ModeOption {
    pub(super) const ALL: [ModeOption; 9] = [
        ModeOption::AllBookmarks,
        ModeOption::ScopedSearch,
        ModeOption::Tags,
        ModeOption::Starred,
        ModeOption::GlobalSearch,
        ModeOption::NoteSearch,
        ModeOption::Databases,
        ModeOption::Help,
        ModeOption::Folders,
    ];

    pub(super) fn label(&self) -> &'static str {
        match self {
            ModeOption::AllBookmarks => "all bookmarks of current database",
            ModeOption::ScopedSearch => "Separate searching mode",
            ModeOption::Tags => "show all Tags",
            ModeOption::Starred => "show Star/imp bookmarks",
            ModeOption::GlobalSearch => "search across all databases",
            ModeOption::NoteSearch => "search notes",
            ModeOption::Databases => "switch database",
            ModeOption::Help => "help",
            ModeOption::Folders => "folders of databases",
        }
    }

    pub(super) fn key_hint(&self) -> &'static str {
        match self {
            ModeOption::AllBookmarks => "l",
            ModeOption::ScopedSearch => "Alt+s",
            ModeOption::Tags => "t",
            ModeOption::Starred => "S",
            ModeOption::GlobalSearch => "z",
            ModeOption::NoteSearch => "Alt+n",
            ModeOption::Databases => "A",
            ModeOption::Help => "?",
            ModeOption::Folders => "F",
        }
    }

    pub(super) fn into_message(self) -> Message {
        match self {
            ModeOption::AllBookmarks => Message::ShowAllBookmarks,
            ModeOption::ScopedSearch => Message::ShowSearchScopePicker,
            ModeOption::Tags => Message::ShowView(ActivePane::TagsList),
            ModeOption::Starred => Message::ShowStarred,
            ModeOption::GlobalSearch => Message::ShowGlobalSearch,
            ModeOption::NoteSearch => Message::ToggleNoteSearch,
            ModeOption::Databases => Message::ShowDatabaseList,
            ModeOption::Help => Message::ShowView(ActivePane::Help),
            ModeOption::Folders => Message::ShowFolders,
        }
    }
}

/// The list of options shown by the Alt+s search-scope popup - lets a
/// search (plain "s" or cross-database "z") be narrowed down to just
/// URLs, just descriptions (titles), or just tags, instead of matching
/// any of them. Picking "tags" also pops up a live tag-name suggestions
/// list under the search box, same idea as the Tags field in the Edit
/// Bookmark screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum SearchScopeOption {
    All,
    Url,
    Description,
    Tag,
}

impl SearchScopeOption {
    pub(super) const ALL: [SearchScopeOption; 4] = [
        SearchScopeOption::All,
        SearchScopeOption::Url,
        SearchScopeOption::Description,
        SearchScopeOption::Tag,
    ];

    pub(super) fn label(&self) -> &'static str {
        match self {
            SearchScopeOption::All => "search everything (URL, description, tags)",
            SearchScopeOption::Url => "search URL only",
            SearchScopeOption::Description => "search description/title only",
            SearchScopeOption::Tag => "search tags only (with tag suggestions)",
        }
    }

    pub(super) fn into_scope(self) -> SearchScope {
        match self {
            SearchScopeOption::All => SearchScope::All,
            SearchScopeOption::Url => SearchScope::Url,
            SearchScopeOption::Description => SearchScope::Description,
            SearchScopeOption::Tag => SearchScope::Tag,
        }
    }

    pub(super) fn from_scope(scope: SearchScope) -> Self {
        match scope {
            SearchScope::All => SearchScopeOption::All,
            SearchScope::Url => SearchScopeOption::Url,
            SearchScope::Description => SearchScopeOption::Description,
            SearchScope::Tag => SearchScopeOption::Tag,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BookmarkItem {
    pub(crate) bookmark: SavedBookmark,
    pub(crate) status: bool,
    pub(crate) starred: bool,
    pub(crate) source_db: Option<(String, String)>,
    /// Why this link can't be opened, set only in the broken-links view
    /// (Alt+B); such items are drawn in red.
    pub(crate) broken_reason: Option<String>,
    /// Whether this bookmark has a note attached. `None` means it hasn't
    /// been checked yet - fetched lazily (only for the selected item, one
    /// bookmark at a time) rather than upfront for the whole list, to keep
    /// list loads cheap.
    pub(crate) has_note: Option<bool>,
    /// Whether this bookmark is marked for a bulk "move to another
    /// database" operation (toggled with Space, acted on with `M`).
    pub(crate) marked: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct BookmarkItems {
    pub(crate) items: Vec<BookmarkItem>,
    pub(crate) state: ListState,
}

#[derive(Debug, Clone)]
pub(crate) struct TagItems {
    pub(crate) items: Vec<TagStats>,
    pub(crate) state: ListState,
}

/// Everything needed to put a main view (bookmark list, tags list, ...)
/// back on screen exactly as it was - used by Esc to step back through
/// the views the user visited.
pub(crate) struct LinkCheckState {
    id: u64,
    pub(super) total: usize,
    pub(super) done: usize,
    /// The uris being checked, to tell whether the list on screen is
    /// still the one that was checked when the result arrives.
    uris: HashSet<String>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Clone)]
pub(crate) struct ViewSnapshot {
    pane: ActivePane,
    bookmark_items: BookmarkItems,
    tag_items: TagItems,
    all_tag_items: Vec<TagStats>,
    showing_starred: bool,
    initial: bool,
    global_search_mode: bool,
    note_search_mode: bool,
    search_scope: SearchScope,
    search_input: Input,
    global_tags_mode: bool,
}

impl ViewSnapshot {
    /// Identifies *what* is shown (not where the cursor is), so a no-op
    /// "navigation" to the very same view can be recognised and skipped.
    fn fingerprint(&self) -> String {
        format!(
            "{:?}|{}|{}|{}|{}|{}|{}|{:?}|{:?}",
            self.pane,
            self.showing_starred,
            self.initial,
            self.global_search_mode,
            self.note_search_mode,
            self.global_tags_mode,
            self.search_input.value(),
            self.bookmark_items
                .items
                .iter()
                .map(|b| (&b.bookmark.uri, &b.source_db, &b.broken_reason))
                .collect::<Vec<_>>(),
            self.tag_items
                .items
                .iter()
                .map(|t| &t.name)
                .collect::<Vec<_>>(),
        )
    }
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
            broken_reason: None,
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
            broken_reason: None,
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
        if let Some(reason) = &value.broken_reason {
            return ListItem::new(Line::styled(
                format!(
                    "[{reason}] {mark_prefix}{star_prefix}{}{db_suffix}",
                    value.bookmark.uri
                ),
                Style::new().fg(ratatui::style::Color::Red),
            ));
        }
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
    /// Delete every bookmark currently listed - (uri, source database path)
    /// pairs, same shape as `MoveBookmarks::items`, plus the count for the
    /// confirmation message (kept separate so the message doesn't need to
    /// re-derive it from `items.len()` in more than one place).
    DeleteAllVisible {
        items: Vec<(String, Option<String>)>,
        count: usize,
    },
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
    /// Alt+z in the tags field: list *every* existing tag instead of only
    /// the ones matching what's typed.
    pub(super) tag_suggest_all: bool,
    pub(super) edit_original_title: Option<String>,
    pub(super) edit_original_tags: Option<String>,
    /// Whether the Tags List view is currently showing tags aggregated
    /// across every local database (`T`) rather than just the active
    /// database's tags (`t`). Controls what `FetchTags`/`FetchGlobalTags`
    /// command gets fired on refresh, and what selecting a tag does.
    pub(super) global_tags_mode: bool,
    /// Name of the tag currently being renamed (Alt+e from the Tags List
    /// view), while `active_pane == ActivePane::RenameTag`.
    pub(super) rename_tag_original: String,
    pub(super) rename_tag_input: Input,
    /// Existing tag names matching whatever's currently typed in
    /// `rename_tag_input` - lets the user pick an already-saved tag to
    /// merge into, instead of accidentally creating a near-duplicate
    /// (same idea as `tag_suggestions` above, just matched against the
    /// whole field instead of a comma-separated fragment).
    pub(super) rename_tag_suggestions: Vec<String>,
    pub(super) rename_tag_suggestion_selected: Option<usize>,
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
    /// Whether the database name input is creating a new database or
    /// renaming an existing one.
    pub(super) db_name_purpose: DbNamePurpose,
    /// Set while a database rename is running, to ignore a second submit.
    pub(super) db_rename_in_flight: bool,
    /// Number of bookmarks per database (by file name; `None` = couldn't
    /// be counted), shown as "[ <count> ]" in the Database List view.
    pub(super) db_counts: HashMap<String, Option<i64>>,
    pub(super) db_counts_loaded: bool,
    /// `Some(index)` while a database is being arranged (`a`): the
    /// position the picked database started from. The cursor then marks
    /// the position it'll be moved to.
    pub(super) db_arrange_origin: Option<usize>,
    /// Where the list that can be clicked (databases / folders) was drawn
    /// in the last frame, so a mouse click can be turned into a row.
    pub(super) click_area: Option<ratatui::layout::Rect>,
    pub(super) folders: Folders,
    pub(super) folder_list_state: ListState,
    pub(super) folder_list_purpose: FolderListPurpose,
    /// The folder whose databases are listed in `FolderContents`
    /// (`None` = the databases that are in no folder).
    pub(super) folder_open: Option<String>,
    /// The databases listed in `FolderContents`.
    pub(super) folder_dbs: Vec<String>,
    pub(super) folder_dbs_state: ListState,
    /// Databases marked with Space in `FolderContents`.
    pub(super) marked_dbs: HashSet<String>,
    /// Databases waiting for a destination folder (`m`).
    pub(super) folder_move_names: Vec<String>,
    /// Current ordering of the database list (Alt+1..Alt+4).
    pub(super) db_view: DbView,
    /// The databases in the order the user arranged (what Alt+1 shows).
    /// `available_dbs` is this same list put in the current view's order.
    pub(super) custom_dbs: Vec<String>,
    /// Views the user came from (oldest first), so Esc can walk back
    /// through them one at a time before finally quitting.
    pub(super) nav_history: Vec<ViewSnapshot>,
    pub(super) skip_nav_push: bool,
    /// Set when search results were shown with nothing to go back to (eg.
    /// searching right after bmm opened): Esc then returns to the full
    /// list of bookmarks before it quits.
    pub(super) search_results_from_home: bool,
    /// A link check (Alt+B) that's running right now.
    pub(super) link_check: Option<LinkCheckState>,
    link_check_counter: u64,
    /// Digits typed so far with Alt held down in the Database List view.
    pub(super) alt_number_buffer: String,
    pub(super) alt_number_deadline: Option<Instant>,
    pub(super) global_search_mode: bool,
    pub(super) note_search_mode: bool,
    /// Which field(s) the search box currently matches against - `All`
    /// unless narrowed with the Alt+s search-scope popup. Applies to both
    /// the plain search ("s") and the cross-database search ("z").
    pub(super) search_scope: SearchScope,
    /// Which pane to return to when the search-scope popup is closed
    /// (Esc) without picking anything - always `List` or `SearchInput`,
    /// whichever it was opened from.
    pub(super) pane_before_scope_picker: ActivePane,
    pub(super) scope_picker_state: ListState,
    /// Tag names matching whatever's currently being typed after the
    /// last space in the search box, while `search_scope` is `Tag` -
    /// same idea as `tag_suggestions` above, just space-delimited
    /// (search terms are already space-separated) instead of comma-
    /// delimited.
    pub(super) search_tag_suggestions: Vec<String>,
    pub(super) search_tag_suggestion_selected: Option<usize>,
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
    /// Whether a description fetch (Alt+F, from the Title field of the
    /// add/edit bookmark screen) is currently in flight - guards against
    /// firing a second fetch while one's still running.
    pub(super) fetching_description: bool,
}

/// Splits a tags-input value like "foo, bar, ba" into the already-
/// completed tags before the last comma (trimmed, original casing kept -
/// `["foo", "bar"]`) and the in-progress fragment after it (trimmed -
/// `"ba"`). A value with no comma at all is treated as a single
/// in-progress fragment (no completed tags yet).
/// How well a typed fragment matches a tag name, for autocomplete: `Some(0)`
/// when the name starts with it, `Some(1)` when the fragment's letters
/// appear in the name in the same order *starting from its first letter*
/// (so "leanet" and "lework" both find "learn_networking", and "lea" +
/// anything further along in the name works too), `None` otherwise.
/// Both arguments must already be lowercase.
pub(super) fn tag_match_rank(name_lower: &str, fragment_lower: &str) -> Option<u8> {
    if name_lower.starts_with(fragment_lower) {
        return Some(0);
    }

    let mut name_chars = name_lower.chars();
    let mut fragment_chars = fragment_lower.chars().peekable();

    // the first letter must match the name's first letter
    if name_lower.chars().next() != fragment_lower.chars().next() {
        return None;
    }

    // every letter of the fragment must be found, in order
    for wanted in fragment_chars.by_ref() {
        loop {
            match name_chars.next() {
                Some(c) if c == wanted => break,
                Some(_) => continue,
                None => return None,
            }
        }
    }

    Some(1)
}

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
            tag_suggest_all: false,
            edit_original_title: None,
            edit_original_tags: None,
            global_tags_mode: false,
            rename_tag_original: String::new(),
            rename_tag_input: Input::default(),
            rename_tag_suggestions: vec![],
            rename_tag_suggestion_selected: None,
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
            db_name_purpose: DbNamePurpose::Create,
            db_rename_in_flight: false,
            db_counts: HashMap::new(),
            db_counts_loaded: false,
            db_arrange_origin: None,
            click_area: None,
            folders: Folders::default(),
            folder_list_state: ListState::default().with_selected(Some(0)),
            folder_list_purpose: FolderListPurpose::Browse,
            folder_open: None,
            folder_dbs: vec![],
            folder_dbs_state: ListState::default().with_selected(Some(0)),
            marked_dbs: HashSet::new(),
            folder_move_names: vec![],
            db_view: DbView::NameAsc,
            custom_dbs: vec![],
            nav_history: vec![],
            skip_nav_push: false,
            search_results_from_home: false,
            link_check: None,
            link_check_counter: 0,
            alt_number_buffer: String::new(),
            alt_number_deadline: None,
            global_search_mode: false,
            note_search_mode: false,
            search_scope: SearchScope::All,
            pane_before_scope_picker: ActivePane::List,
            scope_picker_state: ListState::default(),
            search_tag_suggestions: vec![],
            search_tag_suggestion_selected: None,
            mode_switcher_state: ListState::default(),
            pane_before_mode_switch: active_pane,
            marked_uris: HashSet::new(),
            move_pending: vec![],
            move_last_uris: vec![],
            help_table_state: TableState::default(),
            fetching_description: false,
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
            ActivePane::FolderList => self.folder_list_state.select_next(),
            ActivePane::FolderContents => self.folder_dbs_state.select_next(),
            ActivePane::ModeSwitcher => self.mode_switcher_state.select_next(),
            ActivePane::SearchScopePicker => self.scope_picker_state.select_next(),
            ActivePane::SearchInput => {}
            ActivePane::RenameTag => {}
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
            ActivePane::FolderList => self.folder_list_state.select_previous(),
            ActivePane::FolderContents => self.folder_dbs_state.select_previous(),
            ActivePane::ModeSwitcher => self.mode_switcher_state.select_previous(),
            ActivePane::SearchScopePicker => self.scope_picker_state.select_previous(),
            ActivePane::SearchInput => {}
            ActivePane::RenameTag => {}
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
            ActivePane::FolderList => self.folder_list_state.select_first(),
            ActivePane::FolderContents => self.folder_dbs_state.select_first(),
            ActivePane::ModeSwitcher => self.mode_switcher_state.select_first(),
            ActivePane::SearchScopePicker => self.scope_picker_state.select_first(),
            ActivePane::SearchInput => {}
            ActivePane::RenameTag => {}
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
            ActivePane::FolderList => self.folder_list_state.select_last(),
            ActivePane::FolderContents => self.folder_dbs_state.select_last(),
            ActivePane::ModeSwitcher => self.mode_switcher_state.select_last(),
            ActivePane::SearchScopePicker => self.scope_picker_state.select_last(),
            ActivePane::SearchInput => {}
            ActivePane::RenameTag => {}
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
            ActivePane::RenameTag => view,
            ActivePane::SearchInput => view,
            ActivePane::EditBookmark => view,
            ActivePane::Notes => view,
            ActivePane::DatabaseList => view,
            ActivePane::DatabaseSearchInput => view,
            ActivePane::FolderList => view,
            ActivePane::FolderContents => view,
            ActivePane::NewDatabaseName => view,
            ActivePane::Confirm => view,
            ActivePane::ModeSwitcher => view,
            ActivePane::SearchScopePicker => view,
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
                //
                // This is the plain "t" entry point (current database only)
                // - make sure no leftover "all databases" mode from a
                // previous visit via "T" carries over.
                self.global_tags_mode = false;
                Some(Command::FetchTags)
            }
            ActivePane::FolderList | ActivePane::FolderContents => None,
            ActivePane::TagSearchInput => None,
            ActivePane::RenameTag => None,
            ActivePane::SearchInput => {
                // this is the plain "s" entry point into search - make sure
                // no leftover global/note-search mode (or search scope) from
                // a previous session carries over into what looks like a
                // normal search
                self.global_search_mode = false;
                self.note_search_mode = false;
                self.search_scope = SearchScope::All;
                self.dismiss_search_tag_suggestions();
                None
            }
            ActivePane::EditBookmark => None,
            ActivePane::Notes => None,
            ActivePane::DatabaseList => None,
            ActivePane::DatabaseSearchInput => None,
            ActivePane::NewDatabaseName => None,
            ActivePane::Confirm => None,
            ActivePane::ModeSwitcher => None,
            ActivePane::SearchScopePicker => None,
        }
    }


    fn snapshot_view(&self, pane: ActivePane) -> ViewSnapshot {
        // only "main" views can be returned to; anything else (a search box,
        // a popup, an edit screen...) counts as the bookmark list
        let pane = match pane {
            ActivePane::List
            | ActivePane::TagsList
            | ActivePane::DatabaseList
            | ActivePane::FolderList
            | ActivePane::FolderContents
            | ActivePane::Help => pane,
            _ => ActivePane::List,
        };

        ViewSnapshot {
            pane,
            bookmark_items: self.bookmark_items.clone(),
            tag_items: self.tag_items.clone(),
            all_tag_items: self.all_tag_items.clone(),
            showing_starred: self.showing_starred,
            initial: self.initial,
            global_search_mode: self.global_search_mode,
            note_search_mode: self.note_search_mode,
            search_scope: self.search_scope,
            search_input: self.search_input.clone(),
            global_tags_mode: self.global_tags_mode,
        }
    }

    /// Remembers the view shown right now (as `pane`) before moving on to
    /// another one, so Esc can come back to it.
    pub(super) fn push_nav_snapshot(&mut self, pane: ActivePane) {
        self.search_results_from_home = false;
        let snapshot = self.snapshot_view(pane);

        if self
            .nav_history
            .last()
            .is_some_and(|last| last.fingerprint() == snapshot.fingerprint())
        {
            return;
        }

        self.nav_history.push(snapshot);
        if self.nav_history.len() > 50 {
            self.nav_history.remove(0);
        }
    }

    /// Starts checking the links shown in the list (Alt+B).
    pub(super) fn start_link_check(&mut self) -> Option<Command> {
        if self.link_check.is_some() {
            self.user_message = Some(UserMessage::error(
                "already checking links (Esc cancels)",
            ));
            return None;
        }

        let mut seen = HashSet::new();
        let uris: Vec<String> = self
            .bookmark_items
            .items
            .iter()
            .map(|i| i.bookmark.uri.clone())
            .filter(|u| seen.insert(u.clone()))
            .collect();

        if uris.is_empty() {
            self.user_message = Some(UserMessage::error("no links here to check"));
            return None;
        }

        self.link_check_counter += 1;
        let id = self.link_check_counter;
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        self.link_check = Some(LinkCheckState {
            id,
            total: uris.len(),
            done: 0,
            uris: seen,
            cancel: cancel.clone(),
        });
        self.user_message = Some(UserMessage::info(&format!(
            "checking {} links... (Esc cancels)",
            uris.len()
        )));

        Some(Command::CheckLinks { id, uris, cancel })
    }

    /// Stops a running link check, if any. Returns whether one was running.
    pub(super) fn cancel_link_check(&mut self) -> bool {
        match self.link_check.take() {
            Some(state) => {
                state
                    .cancel
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                true
            }
            None => false,
        }
    }

    pub(super) fn apply_link_check_progress(&mut self, id: u64, done: usize) {
        if let Some(state) = self.link_check.as_mut()
            && state.id == id
        {
            state.done = done.min(state.total);
        }
    }

    /// Shows the broken links found by a finished check, as a separate
    /// list - the list it was started from is remembered, so Esc goes back.
    pub(super) fn apply_link_check_result(&mut self, outcome: LinkCheckOutcome) {
        let Some(state) = self.link_check.take_if(|s| s.id == outcome.id) else {
            return; // cancelled, or a leftover from an earlier check
        };

        if outcome.cancelled {
            return;
        }

        if outcome.broken.is_empty() {
            self.user_message = Some(
                UserMessage::info(&format!(
                    "all {} links are working!",
                    outcome.checked
                ))
                .with_frames_left(1),
            );
            return;
        }

        let still_same_list = self.active_pane == ActivePane::List
            && self.bookmark_items.items.len() >= state.uris.len()
            && state
                .uris
                .iter()
                .all(|u| self.bookmark_items.items.iter().any(|i| &i.bookmark.uri == u));

        if !still_same_list {
            self.user_message = Some(UserMessage::error(&format!(
                "{} of {} links are broken - the view changed, press Alt+B again to see them",
                outcome.broken.len(),
                outcome.checked
            )));
            return;
        }

        let reasons: HashMap<String, String> = outcome.broken.into_iter().collect();

        self.push_nav_snapshot(ActivePane::List);

        let mut broken_items: Vec<BookmarkItem> = self
            .bookmark_items
            .items
            .iter()
            .filter(|i| reasons.contains_key(&i.bookmark.uri))
            .cloned()
            .collect();
        for item in broken_items.iter_mut() {
            item.broken_reason = reasons.get(&item.bookmark.uri).cloned();
        }

        let count = broken_items.len();
        self.bookmark_items.items = broken_items;
        self.bookmark_items.state.select(Some(0));

        self.user_message = Some(UserMessage::info(&format!(
            "{count} of {} links are broken (Esc: back)",
            outcome.checked
        )));
    }

    /// Removes a deleted/moved bookmark from every remembered view, so
    /// going back with Esc never shows a bookmark that no longer exists.
    pub(super) fn forget_uri_in_history(&mut self, uri: &str) {
        for snapshot in self.nav_history.iter_mut() {
            snapshot.bookmark_items.items.retain(|i| i.bookmark.uri != uri);

            let len = snapshot.bookmark_items.items.len();
            if len == 0 {
                snapshot.bookmark_items.state.select(None);
            } else if snapshot
                .bookmark_items
                .state
                .selected()
                .is_some_and(|s| s >= len)
            {
                snapshot.bookmark_items.state.select(Some(len - 1));
            }
        }
    }

    /// Edits, tag renames and bulk deletes change bookmarks in ways the
    /// remembered views can't be patched for, so they're forgotten rather
    /// than shown out of date.
    pub(super) fn clear_nav_history(&mut self) {
        self.nav_history.clear();
    }

    /// Goes back to the previous view, skipping any that look exactly
    /// like what's already on screen. Returns `false` if there's nowhere
    /// to go back to.
    pub(super) fn pop_nav_history(&mut self) -> bool {
        // the real pane, not the "main view" it counts as when remembered -
        // eg. a search box opened over a list is not the same view as that
        // list, or Esc would skip the list it should return to
        let current = {
            let mut snapshot = self.snapshot_view(self.active_pane);
            snapshot.pane = self.active_pane;
            snapshot.fingerprint()
        };

        while let Some(snapshot) = self.nav_history.pop() {
            if snapshot.fingerprint() == current {
                continue;
            }

            self.bookmark_items = snapshot.bookmark_items;
            self.tag_items = snapshot.tag_items;
            self.all_tag_items = snapshot.all_tag_items;
            self.showing_starred = snapshot.showing_starred;
            self.initial = snapshot.initial;
            self.global_search_mode = snapshot.global_search_mode;
            self.note_search_mode = snapshot.note_search_mode;
            self.search_scope = snapshot.search_scope;
            self.search_input = snapshot.search_input;
            self.global_tags_mode = snapshot.global_tags_mode;
            self.active_pane = snapshot.pane;
            self.sync_starred_markers();
            self.search_results_from_home = false;
            return true;
        }

        false
    }

    pub(super) fn go_back_or_quit(&mut self) -> Option<Command> {
        if self.terminal_too_small {
            self.running_state = RunningState::Done;
            return None;
        }

        match self.active_pane {
            ActivePane::List => {
                if self.cancel_link_check() {
                    self.user_message =
                        Some(UserMessage::info("link check cancelled").with_frames_left(1));
                } else if self.pop_nav_history() {
                    // went back to the previous view
                } else if self.search_results_from_home {
                    // search results with nothing before them: back to
                    // the full list of bookmarks first
                    self.search_results_from_home = false;
                    self.global_search_mode = false;
                    self.note_search_mode = false;
                    self.search_scope = SearchScope::All;
                    self.search_input.reset();
                    return Some(Command::FetchAllBookmarks);
                } else {
                    self.running_state = RunningState::Done;
                }
            }
            ActivePane::Help => {
                if !self.pop_nav_history() {
                    self.active_pane = ActivePane::List;
                }
            }
            ActivePane::SearchInput => {
                // opened from another view: just go back to it
                if !self.initial && self.pop_nav_history() {
                    self.dismiss_search_tag_suggestions();
                    return None;
                }

                self.search_input.reset();
                self.initial = false;
                self.active_pane = ActivePane::List;
                self.dismiss_search_tag_suggestions();

                // Live search may have left the list showing filtered
                // results from a query that was never confirmed with
                // Enter - restore the unfiltered view on cancel, same
                // as what an empty confirmed search would show.
                let was_global = self.global_search_mode;
                self.global_search_mode = false;
                self.note_search_mode = false;
                self.search_scope = SearchScope::All;

                return Some(if was_global {
                    Command::GlobalSearch(None, SearchScope::All)
                } else {
                    Command::FetchAllBookmarks
                });
            }
            ActivePane::TagSearchInput => {
                self.cancel_tag_search();
            }
            ActivePane::RenameTag => {
                self.cancel_rename_tag();
            }
            ActivePane::EditBookmark => {
                self.cancel_edit();
            }
            ActivePane::Notes => {
                self.cancel_note_edit();
            }
            ActivePane::DatabaseList => {
                let was_switching = self.db_list_purpose == DbListPurpose::Switch;
                self.move_pending.clear();
                self.db_list_purpose = DbListPurpose::Switch;
                self.db_arrange_origin = None;
                self.alt_number_buffer.clear();
                        self.alt_number_deadline = None;
                if !(was_switching && self.pop_nav_history()) {
                    self.active_pane = ActivePane::List;
                }
            }
            ActivePane::DatabaseSearchInput => {
                // only the search is cancelled - back to the database list
                self.db_search_input.reset();
                self.filtered_dbs = self.available_dbs.clone();
                self.db_arrange_origin = None;
                self.db_list_state.select(Some(0));
                self.active_pane = ActivePane::DatabaseList;
            }
            ActivePane::NewDatabaseName => {
                self.new_db_name_input.reset();
                // cancelling a rename goes back to the database list it
                // was started from; cancelling a create goes back to the
                // bookmarks list, same as before
                self.active_pane = match self.db_name_purpose {
                    DbNamePurpose::Rename(_) => ActivePane::DatabaseList,
                    DbNamePurpose::Create => ActivePane::DatabaseList,
                    DbNamePurpose::CreateFolder | DbNamePurpose::RenameFolder(_) => {
                        ActivePane::FolderList
                    }
                };
                self.db_name_purpose = DbNamePurpose::Create;
            }
            ActivePane::Confirm => {
                self.cancel_confirm();
            }
            ActivePane::TagsList => {
                if !self.pop_nav_history() {
                    self.active_pane = ActivePane::List;
                }
            }
            ActivePane::FolderContents => {
                self.marked_dbs.clear();
                self.active_pane = ActivePane::FolderList;
                self.select_open_folder_row();
            }
            ActivePane::FolderList => {
                if self.folder_list_purpose == FolderListPurpose::MoveDatabases {
                    // cancel picking a destination: back to the databases
                    self.folder_list_purpose = FolderListPurpose::Browse;
                    self.folder_move_names.clear();
                    self.active_pane = ActivePane::FolderContents;
                } else if !self.pop_nav_history() {
                    self.active_pane = ActivePane::List;
                }
            }
            ActivePane::ModeSwitcher => {
                self.active_pane = self.pane_before_mode_switch;
            }
            ActivePane::SearchScopePicker => {
                self.active_pane = self.pane_before_scope_picker;
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
                Command::GlobalSearch(None, self.search_scope)
            } else if self.note_search_mode {
                Command::SearchNotes(None)
            } else {
                Command::FetchAllBookmarks
            });
        }

        match SearchTerms::try_from(query) {
            Ok(terms) => Some(if self.global_search_mode {
                Command::GlobalSearch(Some(terms), self.search_scope)
            } else if self.note_search_mode {
                Command::SearchNotes(Some(terms))
            } else {
                Command::SearchBookmarks(terms, self.search_scope)
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
                .filter(|t| {
                    let name_lower = t.name.to_lowercase();
                    // anywhere in the name, or the letters in order from
                    // the first one (eg. "leanet" -> "learn_networking")
                    name_lower.contains(&query) || tag_match_rank(&name_lower, &query).is_some()
                })
                .cloned()
                .collect()
        };

        self.tag_items = TagItems::from(filtered);
    }

    pub(super) fn cancel_tag_search(&mut self) {
        self.tag_search_input.reset();
        self.tag_items = TagItems::from(self.all_tag_items.clone());
        // back to the tags list the search was started from
        self.active_pane = ActivePane::TagsList;
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

    /// Asks to delete every bookmark currently listed in the List pane
    /// (`D`) - whatever's currently on screen, be that all bookmarks, a
    /// search/tag/starred result, or a global search/tag result. Does
    /// nothing if the list is empty.
    pub(super) fn request_delete_all_visible(&mut self) {
        if self.active_pane != ActivePane::List {
            return;
        }

        let items: Vec<(String, Option<String>)> = self
            .bookmark_items
            .items
            .iter()
            .map(|item| {
                (
                    item.bookmark.uri.clone(),
                    item.source_db.as_ref().map(|(_, path)| path.clone()),
                )
            })
            .collect();

        if items.is_empty() {
            return;
        }

        let count = items.len();
        self.pending_confirmation = Some(PendingConfirmation::DeleteAllVisible { items, count });
        self.pane_before_confirm = ActivePane::List;
        self.active_pane = ActivePane::Confirm;
    }

    pub(super) fn remove_bookmark_by_uri(&mut self, uri: &str) {
        self.forget_uri_in_history(uri);
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

        if self.tag_suggest_all {
            // every tag not already in the field - the ones matching what's
            // typed first, then all the rest alphabetically
            let fragment_lower = fragment.to_lowercase();
            let completed_lower: HashSet<String> =
                completed.iter().map(|t| t.to_lowercase()).collect();

            let mut all: Vec<String> = self
                .all_tag_items
                .iter()
                .map(|t| t.name.clone())
                .filter(|name| !completed_lower.contains(&name.to_lowercase()))
                .collect();

            all.sort_by_key(|name| {
                let name_lower = name.to_lowercase();
                let rank = if fragment_lower.is_empty() {
                    None
                } else {
                    tag_match_rank(&name_lower, &fragment_lower)
                };
                // `None` (no match) must sort after `Some(..)`
                (rank.is_none(), rank, name_lower)
            });

            self.tag_suggestion_selected = if all.is_empty() { None } else { Some(0) };
            self.tag_suggestions = all;
            return;
        }

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
                tag_match_rank(&name_lower, &fragment_lower).is_some()
                    && name_lower != fragment_lower
                    && !completed_lower.contains(&name_lower)
            })
            .collect();

        // names starting with what was typed first, then the looser matches
        // (shorter names first, as they're the closest fit)
        matches.sort_by_key(|name| {
            (
                tag_match_rank(&name.to_lowercase(), &fragment_lower),
                name.chars().count(),
                name.clone(),
            )
        });
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
        self.tag_suggest_all = false;
        self.tag_suggestions.clear();
        self.tag_suggestion_selected = None;
    }

    //-------------------------------------//
    //  search scope picker (Alt+s)         //
    //-------------------------------------//

    /// Opens the search-scope picker (Alt+s) - a small popup letting the
    /// user restrict the search that's about to run (or is already
    /// running, in the "z" all-databases case) to just URLs, just
    /// descriptions (titles), or just tags. Works the same whether it's
    /// opened from the List view (before typing anything) or from the
    /// search box itself - `pane_before_scope_picker` remembers which,
    /// so Esc can return to the right place without disturbing whatever
    /// was already typed.
    pub(super) fn open_search_scope_picker(&mut self) {
        self.pane_before_scope_picker = self.active_pane;

        let current = SearchScopeOption::from_scope(self.search_scope);
        let index = SearchScopeOption::ALL
            .iter()
            .position(|o| *o == current)
            .unwrap_or(0);
        self.scope_picker_state.select(Some(index));

        self.active_pane = ActivePane::SearchScopePicker;
    }

    /// Applies whichever scope is highlighted in the picker (Enter, or a
    /// number key), then drops straight back into the search box -
    /// re-running whatever's already typed against the new scope right
    /// away, live, same as every other keystroke in the search box.
    /// Picking "tags" also (re)fetches the current tag list, so the
    /// tag-name suggestions popup has something to match against - the
    /// active database's tags, or every database's tags at once if
    /// this is a "z" (all-databases) search.
    pub(super) fn confirm_search_scope_selection(&mut self) -> Vec<Command> {
        let mut cmds = Vec::new();

        let selected = self
            .scope_picker_state
            .selected()
            .and_then(|i| SearchScopeOption::ALL.get(i).copied());

        if let Some(option) = selected {
            self.search_scope = option.into_scope();
        }

        self.active_pane = ActivePane::SearchInput;
        self.dismiss_search_tag_suggestions();

        if self.search_scope == SearchScope::Tag {
            cmds.push(if self.global_search_mode {
                Command::FetchGlobalTags
            } else {
                Command::FetchTags
            });
        }

        if let Some(c) = self.live_search_command() {
            cmds.push(c);
        }

        cmds
    }

    //-------------------------------------//
    //  tag suggestions (search box, when   //
    //  search_scope is Tag)                //
    //-------------------------------------//

    /// Recomputes `search_tag_suggestions` from whatever's currently
    /// being typed after the last space in the search box, while
    /// `search_scope` is `Tag` - same idea as `recompute_tag_suggestions`
    /// for the Edit Bookmark screen's Tags field, just space-delimited
    /// (search terms are already space-separated) instead of comma-
    /// delimited.
    pub(super) fn recompute_search_tag_suggestions(&mut self) {
        if self.search_scope != SearchScope::Tag {
            self.dismiss_search_tag_suggestions();
            return;
        }

        // Owned from the start (rather than borrowing straight from
        // `self.search_input`) so nothing here ties up a borrow of
        // `self` past this line - keeps every early return below free
        // to call the (whole-`self`) `dismiss_search_tag_suggestions`.
        let value = self.search_input.value().to_string();
        let mut parts: Vec<String> = value.split(' ').map(|s| s.to_string()).collect();
        let fragment = parts.pop().unwrap_or_default().trim().to_string();

        if fragment.is_empty() {
            self.dismiss_search_tag_suggestions();
            return;
        }

        let completed_lower: HashSet<String> = parts
            .into_iter()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        let fragment_lower = fragment.to_lowercase();

        let mut matches: Vec<String> = self
            .all_tag_items
            .iter()
            .map(|t| t.name.clone())
            .filter(|name| {
                let name_lower = name.to_lowercase();
                tag_match_rank(&name_lower, &fragment_lower).is_some()
                    && name_lower != fragment_lower
                    && !completed_lower.contains(&name_lower)
            })
            .collect();

        // names starting with what was typed first, then the looser matches
        // (shorter names first, as they're the closest fit)
        matches.sort_by_key(|name| {
            (
                tag_match_rank(&name.to_lowercase(), &fragment_lower),
                name.chars().count(),
                name.clone(),
            )
        });
        matches.truncate(MAX_TAG_SUGGESTIONS);

        self.search_tag_suggestion_selected = if matches.is_empty() { None } else { Some(0) };
        self.search_tag_suggestions = matches;
    }

    pub(super) fn search_tag_suggestion_next(&mut self) {
        if self.search_tag_suggestions.is_empty() {
            return;
        }

        let len = self.search_tag_suggestions.len();
        self.search_tag_suggestion_selected = Some(match self.search_tag_suggestion_selected {
            Some(i) => (i + 1) % len,
            None => 0,
        });
    }

    pub(super) fn search_tag_suggestion_previous(&mut self) {
        if self.search_tag_suggestions.is_empty() {
            return;
        }

        let len = self.search_tag_suggestions.len();
        self.search_tag_suggestion_selected = Some(match self.search_tag_suggestion_selected {
            Some(0) | None => len - 1,
            Some(i) => i - 1,
        });
    }

    /// Splices the highlighted suggestion into the search box in place
    /// of the fragment being typed (with a trailing space, ready for
    /// the next tag), then clears the suggestion list - mirrors
    /// `accept_selected_tag_suggestion`, just space- rather than comma-
    /// joined.
    pub(super) fn accept_selected_search_tag_suggestion(&mut self) {
        let Some(i) = self.search_tag_suggestion_selected else {
            return;
        };
        let Some(chosen) = self.search_tag_suggestions.get(i).cloned() else {
            return;
        };

        let value = self.search_input.value().to_string();
        let mut parts: Vec<String> = value
            .split(' ')
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty())
            .collect();
        parts.pop();
        parts.push(chosen);

        let new_value = format!("{} ", parts.join(" "));
        self.search_input = Input::new(new_value);
        self.dismiss_search_tag_suggestions();
    }

    pub(super) fn dismiss_search_tag_suggestions(&mut self) {
        self.search_tag_suggestions.clear();
        self.search_tag_suggestion_selected = None;
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

        if self.tag_suggest_all {
            // keep the full list open (minus the tag just added), to pick
            // several tags one after another
            self.recompute_tag_suggestions();
        } else {
            self.dismiss_tag_suggestions();
        }
    }

    /// Alt+z in the tags field: show every existing tag as a suggestion,
    /// or (pressed again) go back to only the matching ones.
    pub(super) fn toggle_all_tag_suggestions(&mut self) {
        if self.tag_suggest_all {
            self.dismiss_tag_suggestions();
            self.recompute_tag_suggestions();
        } else {
            self.tag_suggest_all = true;
            self.recompute_tag_suggestions();
        }
    }

    //----------------------------------//
    //  tag rename (Alt+e in Tags List)  //
    //----------------------------------//

    /// Opens the rename-tag screen for whichever tag is currently
    /// highlighted in the Tags List view (works from `TagsList` and
    /// `TagSearchInput`, so a tag can be renamed right after narrowing
    /// down to it with "/"). Pre-fills the input with the tag's current
    /// name, so fixing a typo is just editing it in place rather than
    /// retyping the whole tag.
    pub(super) fn start_rename_selected_tag(&mut self) {
        if self.active_pane != ActivePane::TagsList && self.active_pane != ActivePane::TagSearchInput {
            return;
        }

        let Some(current_tag_index) = self.tag_items.state.selected() else {
            return;
        };
        let Some(selected_tag) = self.tag_items.items.get(current_tag_index) else {
            return;
        };

        self.rename_tag_original = selected_tag.name.clone();
        self.rename_tag_input = Input::new(self.rename_tag_original.clone());
        self.dismiss_rename_tag_suggestions();
        self.active_pane = ActivePane::RenameTag;
    }

    /// Recomputes `rename_tag_suggestions` from whatever's currently typed
    /// in `rename_tag_input` - called after every keystroke, so existing
    /// tags pop up live as the user types, letting them pick one to merge
    /// into instead of creating a near-duplicate tag by accident.
    pub(super) fn recompute_rename_tag_suggestions(&mut self) {
        let fragment = self.rename_tag_input.value().trim();

        if fragment.is_empty() {
            self.dismiss_rename_tag_suggestions();
            return;
        }

        let fragment_lower = fragment.to_lowercase();
        let original_lower = self.rename_tag_original.to_lowercase();

        let mut matches: Vec<String> = self
            .all_tag_items
            .iter()
            .map(|t| t.name.clone())
            .filter(|name| {
                let name_lower = name.to_lowercase();
                tag_match_rank(&name_lower, &fragment_lower).is_some()
                    && name_lower != fragment_lower
                    && name_lower != original_lower
            })
            .collect();

        matches.sort_by_key(|name| {
            (
                tag_match_rank(&name.to_lowercase(), &fragment_lower),
                name.chars().count(),
                name.clone(),
            )
        });
        matches.truncate(MAX_TAG_SUGGESTIONS);

        self.rename_tag_suggestion_selected = if matches.is_empty() { None } else { Some(0) };
        self.rename_tag_suggestions = matches;
    }

    pub(super) fn rename_tag_suggestion_next(&mut self) {
        if self.rename_tag_suggestions.is_empty() {
            return;
        }

        let len = self.rename_tag_suggestions.len();
        self.rename_tag_suggestion_selected = Some(match self.rename_tag_suggestion_selected {
            Some(i) => (i + 1) % len,
            None => 0,
        });
    }

    pub(super) fn rename_tag_suggestion_previous(&mut self) {
        if self.rename_tag_suggestions.is_empty() {
            return;
        }

        let len = self.rename_tag_suggestions.len();
        self.rename_tag_suggestion_selected = Some(match self.rename_tag_suggestion_selected {
            Some(0) | None => len - 1,
            Some(i) => i - 1,
        });
    }

    pub(super) fn dismiss_rename_tag_suggestions(&mut self) {
        self.rename_tag_suggestions.clear();
        self.rename_tag_suggestion_selected = None;
    }

    /// Replaces the whole rename input with the currently-highlighted
    /// suggestion (unlike `accept_selected_tag_suggestion`, there's no
    /// comma-separated fragment to splice into - this field is always a
    /// single tag name), then clears the suggestion list.
    pub(super) fn accept_selected_rename_tag_suggestion(&mut self) {
        let Some(i) = self.rename_tag_suggestion_selected else {
            return;
        };
        let Some(chosen) = self.rename_tag_suggestions.get(i).cloned() else {
            return;
        };

        self.rename_tag_input = Input::new(chosen);
        self.dismiss_rename_tag_suggestions();
    }

    pub(super) fn cancel_rename_tag(&mut self) {
        self.rename_tag_input.reset();
        self.rename_tag_original.clear();
        self.dismiss_rename_tag_suggestions();
        self.active_pane = ActivePane::TagsList;
    }

    /// Validates and kicks off the actual rename (Ctrl+S). Returns `None`
    /// (without a command) for an empty name, or when the name typed is
    /// the same tag it started as - in the latter case this just quietly
    /// closes the screen, same as pressing Esc, since there's nothing to
    /// do. Deeper validation (tag name format) happens once the command
    /// actually runs, so it can be reported the same way for both the
    /// single- and all-databases cases.
    pub(super) fn request_save_rename_tag(&mut self) -> Option<Command> {
        if self.active_pane != ActivePane::RenameTag {
            return None;
        }

        let new_name = self.rename_tag_input.value().trim().to_string();

        if new_name.is_empty() {
            self.user_message = Some(UserMessage::error("tag name can't be empty"));
            return None;
        }

        if new_name.eq_ignore_ascii_case(&self.rename_tag_original) {
            self.cancel_rename_tag();
            return None;
        }

        let old_name = std::mem::take(&mut self.rename_tag_original);
        let global = self.global_tags_mode;
        self.cancel_rename_tag();

        Some(Command::RenameTag {
            old_name,
            new_name,
            global,
        })
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
            self.user_message = Some(UserMessage::error("url can't be empty"));
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

        // the order the user arranged them in (`a`), then any databases
        // not arranged yet, alphabetically
        self.custom_dbs = db_order::apply_order(dbs, &db_order::load_order());
        self.available_dbs = self.dbs_in_view_order();
        self.filtered_dbs = self.available_dbs.clone();
    }

    /// `custom_dbs` put in the order of the current view (Alt+1..Alt+4).
    /// Databases whose link count isn't known (yet) go last, and ties
    /// keep the user's own arrangement.
    fn dbs_in_view_order(&self) -> Vec<String> {
        let mut dbs = self.custom_dbs.clone();
        let count_of = |name: &String| self.db_counts.get(name).copied().flatten();

        match self.db_view {
            DbView::Custom => {}
            DbView::NameAsc => dbs.sort_by_key(|n| n.to_lowercase()),
            DbView::NameDesc => dbs.sort_by(|a, b| b.to_lowercase().cmp(&a.to_lowercase())),
            DbView::CountAsc => dbs.sort_by_key(|n| match count_of(n) {
                Some(c) => (0, c),
                None => (1, 0),
            }),
            DbView::CountDesc => dbs.sort_by_key(|n| match count_of(n) {
                Some(c) => (0, -c),
                None => (1, 0),
            }),
        }

        dbs
    }

    /// Re-sorts the list for the current view, keeping the same database
    /// highlighted and any search filter applied.
    fn reorder_db_list(&mut self) {
        let selected_name = self
            .selected_db_index()
            .and_then(|i| self.filtered_dbs.get(i))
            .cloned();

        self.available_dbs = self.dbs_in_view_order();
        self.filter_databases();

        if let Some(name) = selected_name
            && let Some(i) = self.filtered_dbs.iter().position(|d| d == &name)
        {
            self.db_list_state.select(Some(i));
        }
    }

    pub(super) fn cycle_db_view(&mut self) {
        self.set_db_view(self.db_view.next());
    }

    pub(super) fn set_db_view(&mut self, view: DbView) {
        self.db_view = view;
        self.reorder_db_list();
        self.user_message = Some(UserMessage::info(&format!("view: {}", view.label())).with_frames_left(1));
    }

    fn show_database_list_for(&mut self, purpose: DbListPurpose) {
        // the list always opens in view 2 (names A-Z, same as Alt+Super+2)
        self.db_view = DbView::NameAsc;
        self.scan_available_databases();
        self.db_search_input.reset();
        self.db_list_purpose = purpose;
        self.db_arrange_origin = None;
        self.alt_number_buffer.clear();
        self.alt_number_deadline = None;

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
                .filter(|d| {
                    let name_lower = d.to_lowercase();
                    // anywhere in the name, or the letters in order from
                    // the first one (eg. "ledata" -> "learn_database"),
                    // same as searching tags
                    name_lower.contains(&query) || tag_match_rank(&name_lower, &query).is_some()
                })
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
        self.db_arrange_origin = None;
        self.db_list_state.select(Some(0));
        // only the search is cancelled: back to the database list it was
        // started from (a pending move, if any, carries on)
        self.active_pane = ActivePane::DatabaseList;
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
        self.db_name_purpose = DbNamePurpose::Create;
        self.active_pane = ActivePane::NewDatabaseName;
    }

    //-------------------------------//
    //  folders of databases ('F')    //
    //-------------------------------//

    /// Opens the folder list.
    pub(super) fn show_folders(&mut self) {
        self.scan_available_databases();
        self.folders = Folders::load();
        self.folder_list_purpose = FolderListPurpose::Browse;
        self.folder_move_names.clear();
        self.marked_dbs.clear();
        self.folder_list_state.select(Some(0));
        self.active_pane = ActivePane::FolderList;
    }

    /// The databases (that exist) in the given folder, in the user's own
    /// database order. `None` = the databases that are in no folder.
    pub(super) fn dbs_in_folder(&self, folder: Option<&str>) -> Vec<String> {
        match folder {
            Some(name) => self
                .custom_dbs
                .iter()
                .filter(|d| self.folders.folder_of(d) == Some(name))
                .cloned()
                .collect(),
            None => self.folders.unfiled(&self.custom_dbs),
        }
    }

    /// The rows of the folder list: every folder, then "(no folder)" - which
    /// is only listed when some database is in no folder, or while picking
    /// a folder to move databases into (so they can be taken out of one).
    pub(super) fn folder_rows(&self) -> Vec<FolderRow> {
        let mut rows: Vec<FolderRow> = self
            .folders
            .folders
            .iter()
            .map(|f| FolderRow::Folder(f.name.clone()))
            .collect();

        if self.folder_list_purpose == FolderListPurpose::MoveDatabases
            || !self.dbs_in_folder(None).is_empty()
        {
            rows.push(FolderRow::Unfiled);
        }

        rows
    }

    fn selected_folder_index(&self) -> Option<usize> {
        let len = self.folder_rows().len();
        if len == 0 {
            return None;
        }
        Some(self.folder_list_state.selected().unwrap_or(0).min(len - 1))
    }

    pub(super) fn selected_folder_row(&self) -> Option<FolderRow> {
        let index = self.selected_folder_index()?;
        self.folder_rows().into_iter().nth(index)
    }

    fn row_folder_name(row: &FolderRow) -> Option<String> {
        match row {
            FolderRow::Folder(name) => Some(name.clone()),
            FolderRow::Unfiled => None,
        }
    }

    /// Puts the folder list's cursor on the folder that was open.
    fn select_open_folder_row(&mut self) {
        let rows = self.folder_rows();
        let index = match &self.folder_open {
            Some(name) => rows
                .iter()
                .position(|r| matches!(r, FolderRow::Folder(n) if n == name)),
            None => rows.iter().position(|r| *r == FolderRow::Unfiled),
        };
        self.folder_list_state.select(Some(index.unwrap_or(0)));
    }

    fn refresh_folder_dbs(&mut self) {
        self.folder_dbs = self.dbs_in_folder(self.folder_open.as_deref());
        let len = self.folder_dbs.len();
        if len == 0 {
            self.folder_dbs_state.select(Some(0));
        } else if self.folder_dbs_state.selected().unwrap_or(0) >= len {
            self.folder_dbs_state.select(Some(len - 1));
        }
    }

    pub(super) fn selected_folder_db_index(&self) -> Option<usize> {
        if self.folder_dbs.is_empty() {
            return None;
        }
        Some(
            self.folder_dbs_state
                .selected()
                .unwrap_or(0)
                .min(self.folder_dbs.len() - 1),
        )
    }

    /// Enter in the folder list: open the folder - or, while picking a
    /// destination, move the waiting databases into it.
    pub(super) fn enter_selected_folder(&mut self) {
        let Some(row) = self.selected_folder_row() else {
            return;
        };

        match self.folder_list_purpose {
            FolderListPurpose::Browse => {
                self.folder_open = Self::row_folder_name(&row);
                self.marked_dbs.clear();
                self.folder_dbs_state.select(Some(0));
                self.refresh_folder_dbs();
                self.active_pane = ActivePane::FolderContents;
            }
            FolderListPurpose::MoveDatabases => {
                let target = Self::row_folder_name(&row);
                let names = std::mem::take(&mut self.folder_move_names);
                self.folders.move_databases(&names, target.as_deref());

                let destination = target.as_deref().unwrap_or(UNFILED_LABEL).to_string();
                self.user_message = Some(match self.folders.save() {
                    Ok(()) => UserMessage::info(&format!(
                        "moved {} {} to \"{destination}\"",
                        names.len(),
                        if names.len() == 1 { "database" } else { "databases" }
                    ))
                    .with_frames_left(1),
                    Err(e) => UserMessage::error(&format!("couldn't save folders: {e}")),
                });

                self.marked_dbs.clear();
                self.folder_list_purpose = FolderListPurpose::Browse;
                self.refresh_folder_dbs();
                self.active_pane = ActivePane::FolderContents;
            }
        }
    }

    /// Space in a folder: marks/unmarks the database under cursor.
    pub(super) fn toggle_mark_selected_db(&mut self) {
        let Some(name) = self
            .selected_folder_db_index()
            .and_then(|i| self.folder_dbs.get(i))
            .cloned()
        else {
            return;
        };

        if !self.marked_dbs.remove(&name) {
            self.marked_dbs.insert(name);
        }
        self.folder_dbs_state.select_next();
    }

    /// `m` in a folder: pick the folder to move the marked databases into
    /// (or the one under cursor, if none is marked).
    pub(super) fn start_move_databases(&mut self) {
        let mut names: Vec<String> = self
            .folder_dbs
            .iter()
            .filter(|d| self.marked_dbs.contains(*d))
            .cloned()
            .collect();

        if names.is_empty()
            && let Some(name) = self
                .selected_folder_db_index()
                .and_then(|i| self.folder_dbs.get(i))
        {
            names.push(name.clone());
        }

        if names.is_empty() {
            return;
        }

        self.folder_move_names = names;
        self.folder_list_purpose = FolderListPurpose::MoveDatabases;

        // start on the first folder other than the one being moved out of
        let current = match &self.folder_open {
            Some(name) => FolderRow::Folder(name.clone()),
            None => FolderRow::Unfiled,
        };
        let first_other = self.folder_rows().iter().position(|r| *r != current);
        match first_other {
            Some(i) => self.folder_list_state.select(Some(i)),
            None => self.select_open_folder_row(),
        }
        self.active_pane = ActivePane::FolderList;
    }

    /// Enter in a folder: switch to the database under cursor.
    pub(super) fn request_switch_to_folder_db(&mut self) -> Option<Command> {
        let name = self
            .selected_folder_db_index()
            .and_then(|i| self.folder_dbs.get(i))?
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

    /// `C` in the folder list: asks for the new folder's name.
    pub(super) fn start_create_folder(&mut self) {
        self.new_db_name_input.reset();
        self.db_name_purpose = DbNamePurpose::CreateFolder;
        self.active_pane = ActivePane::NewDatabaseName;
    }

    /// `R` in the folder list: asks for the folder's new name.
    pub(super) fn start_rename_folder(&mut self) {
        let Some(FolderRow::Folder(name)) = self.selected_folder_row() else {
            self.user_message = Some(UserMessage::error(&format!(
                "\"{UNFILED_LABEL}\" isn't a real folder, it can't be renamed"
            )));
            return;
        };

        self.new_db_name_input = Input::new(name.clone());
        self.db_name_purpose = DbNamePurpose::RenameFolder(name);
        self.active_pane = ActivePane::NewDatabaseName;
    }

    /// Enter in the folder-name input, when creating or renaming a folder.
    pub(super) fn submit_folder_name(&mut self) {
        let renaming = match &self.db_name_purpose {
            DbNamePurpose::RenameFolder(old) => Some(old.clone()),
            _ => None,
        };

        let name = match self
            .folders
            .validate_name(self.new_db_name_input.value(), renaming.as_deref())
        {
            Ok(n) => n,
            Err(e) => {
                self.user_message = Some(UserMessage::error(&e));
                return;
            }
        };

        match &renaming {
            Some(old) => {
                self.folders.rename(old, name.clone());
                if self.folder_open.as_deref() == Some(old.as_str()) {
                    self.folder_open = Some(name.clone());
                }
            }
            None => self.folders.add(name.clone()),
        }

        self.user_message = Some(match self.folders.save() {
            Ok(()) => UserMessage::info(&match renaming {
                Some(_) => format!("folder renamed to \"{name}\""),
                None => format!("folder \"{name}\" created"),
            })
            .with_frames_left(1),
            Err(e) => UserMessage::error(&format!("couldn't save folders: {e}")),
        });

        self.new_db_name_input.reset();
        self.db_name_purpose = DbNamePurpose::Create;
        self.active_pane = ActivePane::FolderList;

        let index = self
            .folder_rows()
            .iter()
            .position(|r| matches!(r, FolderRow::Folder(n) if *n == name))
            .unwrap_or(0);
        self.folder_list_state.select(Some(index));
    }

    /// `D` in the folder list: deletes the folder (its databases are kept,
    /// they just end up in no folder).
    pub(super) fn delete_selected_folder(&mut self) {
        let Some(FolderRow::Folder(name)) = self.selected_folder_row() else {
            self.user_message = Some(UserMessage::error(&format!(
                "\"{UNFILED_LABEL}\" isn't a real folder, it can't be deleted"
            )));
            return;
        };

        let moved = self.folders.delete(&name);
        self.user_message = Some(match self.folders.save() {
            Ok(()) => UserMessage::info(&format!(
                "folder \"{name}\" deleted ({moved} {} now in no folder)",
                if moved == 1 { "database is" } else { "databases are" }
            ))
            .with_frames_left(1),
            Err(e) => UserMessage::error(&format!("couldn't save folders: {e}")),
        });

        let len = self.folder_rows().len();
        let selected = self.folder_list_state.selected().unwrap_or(0);
        self.folder_list_state
            .select(Some(selected.min(len.saturating_sub(1))));
    }

    //-------------------------------//
    //  database list extras:         //
    //  counts, Alt+number, arrange,  //
    //  rename                        //
    //-------------------------------//

    /// Which list row (0-based) is at this screen position, for the
    /// database / folder lists - `None` if the position isn't on a row.
    pub(super) fn clicked_row(&self, column: u16, row: u16) -> Option<usize> {
        let area = self.click_area?;

        if column < area.x
            || column >= area.x + area.width
            || row < area.y
            || row >= area.y + area.height
        {
            return None;
        }

        let (offset, len) = match self.active_pane {
            ActivePane::DatabaseList | ActivePane::DatabaseSearchInput => {
                (self.db_list_state.offset(), self.displayed_dbs().len())
            }
            ActivePane::FolderList => (self.folder_list_state.offset(), self.folder_rows().len()),
            ActivePane::FolderContents => (self.folder_dbs_state.offset(), self.folder_dbs.len()),
            _ => return None,
        };

        let index = offset + (row - area.y) as usize;
        (index < len).then_some(index)
    }

    /// Index (into `filtered_dbs`) of the highlighted database, kept
    /// within the list's bounds - the list state can briefly point one
    /// past the end after pressing Down on the last item.
    pub(super) fn selected_db_index(&self) -> Option<usize> {
        if self.filtered_dbs.is_empty() {
            return None;
        }

        Some(
            self.db_list_state
                .selected()
                .unwrap_or(0)
                .min(self.filtered_dbs.len() - 1),
        )
    }

    /// The databases as they should be drawn right now. While arranging
    /// (`a`), this previews the result: the picked database sits at the
    /// cursor's number, and the one that had that number is at the picked
    /// database's old number.
    pub(super) fn displayed_dbs(&self) -> Vec<String> {
        match (self.db_arrange_origin, self.selected_db_index()) {
            (Some(origin), Some(target)) => db_order::swapped(&self.filtered_dbs, origin, target),
            _ => self.filtered_dbs.clone(),
        }
    }

    pub(super) fn apply_db_counts(&mut self, counts: Vec<(String, Option<i64>)>) {
        self.db_counts = counts.into_iter().collect();
        self.db_counts_loaded = true;

        // count-based views can only be put in order once counts are known
        if matches!(self.db_view, DbView::CountAsc | DbView::CountDesc)
            && self.db_arrange_origin.is_none()
        {
            self.reorder_db_list();
        }
    }

    /// Sum of the bookmarks across all databases, and whether every
    /// database could be counted. `None` until counts have been fetched.
    pub(super) fn total_db_count(&self) -> Option<(i64, bool)> {
        if !self.db_counts_loaded {
            return None;
        }

        let mut total = 0i64;
        let mut complete = true;

        for name in &self.available_dbs {
            match self.db_counts.get(name) {
                Some(Some(n)) => total += n,
                _ => complete = false,
            }
        }

        Some((total, complete))
    }

    /// Records one more digit typed with Alt held down. Returns `None` if
    /// the digit was ignored (a leading zero).
    pub(super) fn push_alt_digit(&mut self, c: char) -> Option<AltNumber> {
        if self.alt_number_buffer.is_empty() && c == '0' {
            // database numbers don't start with a zero
            return None;
        }


        self.alt_number_buffer.push(c);

        let outcome =
            db_order::resolve_alt_number(&self.alt_number_buffer, self.filtered_dbs.len());

        match outcome {
            AltNumber::Wait => {
                self.alt_number_deadline = Some(Instant::now() + ALT_NUMBER_TIMEOUT);
            }
            AltNumber::Jump(_) | AltNumber::Invalid => {
                self.alt_number_buffer.clear();
                self.alt_number_deadline = None;
            }
        }

        Some(outcome)
    }

    pub(super) fn alt_number_expired(&self) -> bool {
        self.alt_number_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
    }

    /// If the pause after the last Alt+digit has passed, takes the typed
    /// number and returns the 0-based database index it refers to (if it
    /// still refers to one). Clears the pending number either way.
    pub(super) fn take_expired_alt_number(&mut self) -> Option<usize> {
        if !self.alt_number_expired() {
            return None;
        }

        self.alt_number_deadline = None;
        let typed = std::mem::take(&mut self.alt_number_buffer);

        if self.active_pane != ActivePane::DatabaseList || self.db_arrange_origin.is_some() {
            return None;
        }

        let number = typed.parse::<usize>().ok()?;
        if number == 0 || number > self.filtered_dbs.len() {
            return None;
        }

        Some(number - 1)
    }

    /// Starts arranging (`a`): the highlighted database is picked up, and
    /// Up/Down now choose the number it'll be moved to.
    pub(super) fn start_arrange_databases(&mut self) {
        if self.db_view != DbView::Custom {
            self.user_message = Some(UserMessage::error(
                "switch to view 1 (my order: Alt+Super+1, or press v) to arrange databases",
            ));
            return;
        }

        if self.filtered_dbs.len() != self.available_dbs.len() {
            self.user_message = Some(UserMessage::error(
                "clear the database search (/) before arranging",
            ));
            return;
        }

        if self.available_dbs.len() < 2 {
            self.user_message = Some(UserMessage::error(
                "you need at least 2 databases to arrange them",
            ));
            return;
        }

        let Some(index) = self.selected_db_index() else {
            return;
        };

        self.db_list_state.select(Some(index));
        self.db_arrange_origin = Some(index);
    }

    /// Saves the arrangement: the picked database takes the number under
    /// the cursor, and the database that had that number takes the picked
    /// database's old number.
    pub(super) fn confirm_arrange_databases(&mut self) {
        let Some(origin) = self.db_arrange_origin.take() else {
            return;
        };
        let target = self.selected_db_index().unwrap_or(origin);
        self.db_list_state.select(Some(target));

        if origin == target {
            return;
        }

        self.available_dbs = db_order::swapped(&self.available_dbs, origin, target);
        self.custom_dbs = self.available_dbs.clone();
        self.filtered_dbs = self.available_dbs.clone();

        self.user_message = Some(match db_order::save_order(&self.available_dbs) {
            Ok(()) => UserMessage::info("arranged!").with_frames_left(1),
            Err(e) => UserMessage::error(&format!("couldn't save the new order: {e}")),
        });
    }

    pub(super) fn cancel_arrange_databases(&mut self) {
        if let Some(origin) = self.db_arrange_origin.take() {
            self.db_list_state.select(Some(origin));
        }
    }

    /// Opens the rename screen (`R`) for the highlighted database.
    pub(super) fn start_rename_database(&mut self) {
        let Some(name) = self
            .selected_db_index()
            .and_then(|i| self.filtered_dbs.get(i))
            .cloned()
        else {
            return;
        };

        // bmm opens this one at every start, so renaming it would make
        // bmm come up with a new, empty database next time
        if name == db_order::DEFAULT_DB_NAME {
            self.user_message = Some(UserMessage::error(&format!(
                "{name} is the default database (bmm opens it on every start), so it can't be renamed"
            )));
            return;
        }

        let stem = name.strip_suffix(".db").unwrap_or(&name).to_string();
        self.new_db_name_input = Input::new(stem);
        self.db_name_purpose = DbNamePurpose::Rename(name);
        self.active_pane = ActivePane::NewDatabaseName;
    }

    pub(super) fn request_rename_database(&mut self) -> Option<Command> {
        if self.db_rename_in_flight {
            return None;
        }

        let DbNamePurpose::Rename(old_name) = self.db_name_purpose.clone() else {
            return None;
        };

        let new_name = match db_order::normalize_db_name(self.new_db_name_input.value()) {
            Ok(n) => n,
            Err(e) => {
                self.user_message = Some(UserMessage::error(&e));
                return None;
            }
        };

        if new_name == old_name {
            self.user_message = Some(UserMessage::info("name unchanged").with_frames_left(1));
            self.new_db_name_input.reset();
            self.db_name_purpose = DbNamePurpose::Create;
            self.active_pane = ActivePane::DatabaseList;
            return None;
        }

        let data_dir = crate::utils::get_data_dir().ok()?;
        let bmm_dir = data_dir.join("bmm");
        let old_path = bmm_dir.join(&old_name);
        let new_path = bmm_dir.join(&new_name);

        if !old_path.exists() {
            self.user_message = Some(UserMessage::error(
                "couldn't find this database's file in bmm's data directory",
            ));
            return None;
        }

        // A case-only change ("work.db" -> "Work.db") points at the very
        // same file on case-insensitive file systems, so `exists()` alone
        // can't be trusted there - but another listed database with
        // exactly the new name is always a clash.
        let same_name_ignoring_case = new_name.to_lowercase() == old_name.to_lowercase();
        let clashes = self.available_dbs.iter().any(|d| d == &new_name)
            || (!same_name_ignoring_case && new_path.exists());
        if clashes {
            self.user_message = Some(UserMessage::error(
                "a database with this name already exists",
            ));
            return None;
        }

        let old_path = old_path.to_str()?.to_string();
        let new_path = new_path.to_str()?.to_string();

        self.db_rename_in_flight = true;

        Some(Command::RenameDatabase {
            is_active: old_name == self.active_db_name,
            old_name,
            old_path,
            new_name,
            new_path,
        })
    }

    /// Applies a finished (successful) rename to what's on screen.
    pub(super) fn apply_database_renamed(&mut self, outcome: &DatabaseRenameOutcome) {
        // keep the renamed database at the same number it had before
        let new_order =
            db_order::rename_in_order(&self.custom_dbs, &outcome.old_name, &outcome.new_name);
        let _ = db_order::save_order(&new_order);

        // the renamed database stays in its folder
        let mut folders = Folders::load();
        folders.rename_database(&outcome.old_name, &outcome.new_name);
        let _ = folders.save();

        if self.active_db_name == outcome.old_name {
            self.active_db_name = outcome.new_name.clone();
        }

        if let Some(count) = self.db_counts.remove(&outcome.old_name) {
            self.db_counts.insert(outcome.new_name.clone(), count);
        }

        // results listed from a database that was just renamed (global
        // search/tag results) must point at its new file - the old path
        // is gone, and opening it again would create an empty database
        for item in self.bookmark_items.items.iter_mut() {
            if let Some((name, path)) = item.source_db.as_mut()
                && *path == outcome.old_path
            {
                *name = outcome.new_name.clone();
                *path = outcome.new_path.clone();
            }
        }

        self.new_db_name_input.reset();
        self.db_name_purpose = DbNamePurpose::Create;

        if matches!(
            self.active_pane,
            ActivePane::NewDatabaseName | ActivePane::DatabaseList
        ) {
            self.scan_available_databases();
            let index = self
                .filtered_dbs
                .iter()
                .position(|d| d == &outcome.new_name)
                .unwrap_or(0);
            self.db_list_state.select(Some(index));
            self.active_pane = ActivePane::DatabaseList;
        }
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
        self.nav_history.clear();
        self.cancel_link_check();
        self.bookmark_items = BookmarkItems::default();
        self.tag_items = TagItems::default();
        self.all_tag_items = vec![];
        self.starred_uris = HashSet::new();
        self.showing_starred = false;
        self.global_tags_mode = false;
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
            Some(PendingConfirmation::DeleteAllVisible { count, .. }) => {
                let word = if *count == 1 { "link" } else { "links" };
                format!(
                    "Warning: this will permanently delete ALL {count} currently listed {word} at once.\n\nAre you sure you want to delete these {count} {word}?"
                )
            }
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

#[cfg(test)]
mod tag_match_tests {
    use super::tag_match_rank;

    #[test]
    fn prefix_matches_rank_first() {
        assert_eq!(tag_match_rank("learn_networking", "lea"), Some(0));
        assert_eq!(tag_match_rank("learn_networking", "learn_net"), Some(0));
    }

    #[test]
    fn letters_further_along_in_order_also_match() {
        assert_eq!(tag_match_rank("learn_networking", "leanet"), Some(1));
        assert_eq!(tag_match_rank("learn_networking", "lework"), Some(1));
        assert_eq!(tag_match_rank("learn_networking", "learking"), Some(1));
        assert_eq!(tag_match_rank("learn_networking", "leaworking"), Some(1));
    }

    #[test]
    fn must_start_with_the_same_letter_and_keep_order() {
        assert_eq!(tag_match_rank("learn_networking", "net"), None);
        assert_eq!(tag_match_rank("learn_networking", "lwn"), Some(1));
        assert_eq!(tag_match_rank("learn_networking", "lnaer"), None);
        assert_eq!(tag_match_rank("learn_networking", "leaz"), None);
        assert_eq!(tag_match_rank("learn_networking", "x"), None);
    }

    #[test]
    fn database_names_match_the_same_way() {
        // "le" + "data" typed together finds learn_database.db
        assert_eq!(tag_match_rank("learn_database.db", "ledata"), Some(1));
        assert_eq!(tag_match_rank("learn_database.db", "data"), None);
    }

    #[test]
    fn works_with_emoji_and_other_scripts() {
        assert_eq!(tag_match_rank("🔥cool_stuff", "🔥st"), Some(1));
        assert_eq!(tag_match_rank("सुरक्षा", "सक"), Some(1));
    }
}
