use ratatui::style::Color;

pub const FG_COLOR: Color = Color::from_u32(0x282828);
pub const PRIMARY_COLOR: Color = Color::from_u32(0xd3869b);
pub const HELP_COLOR: Color = Color::from_u32(0xfabd2f);
pub const COLOR_TWO: Color = Color::from_u32(0x83a598);
pub const COLOR_THREE: Color = Color::from_u32(0xfabd2f);
pub const TAGS_COLOR: Color = Color::from_u32(0xb8bb26);
pub const INFO_MESSAGE_COLOR: Color = Color::from_u32(0x83a598);
pub const ERROR_MESSAGE_COLOR: Color = Color::from_u32(0xfb4934);
pub const TITLE: &str = " bmm ";
pub const MIN_TERMINAL_WIDTH: u16 = 40;
pub const MIN_TERMINAL_HEIGHT: u16 = 16;
pub const MAX_BULK_OPEN_LINKS: usize = 30;
pub const MAX_TAG_SUGGESTIONS: usize = 6;

#[derive(PartialEq, Debug, Clone, Copy)]
pub(crate) enum ActivePane {
    List,
    TagsList,
    SearchInput,
    TagSearchInput,
    RenameTag,
    EditBookmark,
    Notes,
    DatabaseList,
    DatabaseSearchInput,
    NewDatabaseName,
    /// The list of folders (databases grouped by label) - `F`.
    FolderList,
    /// The databases inside one folder (or the ones in no folder).
    FolderContents,
    Confirm,
    Help,
    ModeSwitcher,
    /// The small popup opened with Alt+s, letting the user restrict a
    /// search (plain "s" or cross-database "z") to just URLs, just
    /// descriptions (titles), or just tags.
    SearchScopePicker,
}

/// Which task the [`ActivePane::DatabaseList`] / [`ActivePane::DatabaseSearchInput`]
/// panes are currently being used for - they're shared between the "switch
/// active database" flow (`A`) and the "move bookmark(s) to another
/// database" flow (`m` / `M`), since both are just "pick a database from
/// the list" at their core.
#[derive(PartialEq, Debug, Clone, Copy)]
pub(crate) enum DbListPurpose {
    Switch,
    Move,
}

/// How the Database List view orders its databases (Alt+Super+1 ..
/// Alt+Super+5, or `v` to go through them one by one). `Custom` is the
/// order the user arranged with `a` and is the default.
#[derive(PartialEq, Debug, Clone, Copy)]
pub(crate) enum DbView {
    /// 1: the order the user arranged by hand
    Custom,
    /// 2: names A-Z
    NameAsc,
    /// 3: names Z-A
    NameDesc,
    /// 4: fewest links first
    CountAsc,
    /// 5: most links first
    CountDesc,
}

impl DbView {
    pub(crate) fn from_digit(c: char) -> Option<DbView> {
        match c {
            '1' => Some(DbView::Custom),
            '2' => Some(DbView::NameAsc),
            '3' => Some(DbView::NameDesc),
            '4' => Some(DbView::CountAsc),
            '5' => Some(DbView::CountDesc),
            _ => None,
        }
    }

    /// The next view, going around: 1 -> 2 -> 3 -> 4 -> 5 -> 1.
    pub(crate) fn next(&self) -> DbView {
        match self {
            DbView::Custom => DbView::NameAsc,
            DbView::NameAsc => DbView::NameDesc,
            DbView::NameDesc => DbView::CountAsc,
            DbView::CountAsc => DbView::CountDesc,
            DbView::CountDesc => DbView::Custom,
        }
    }

    pub(crate) fn label(&self) -> &'static str {
        match self {
            DbView::Custom => "1: my order",
            DbView::NameAsc => "2: name A-Z",
            DbView::NameDesc => "3: name Z-A",
            DbView::CountAsc => "4: links low-high",
            DbView::CountDesc => "5: links high-low",
        }
    }
}

/// What the [`ActivePane::NewDatabaseName`] pane is currently being used
/// for - creating a brand new database, or renaming an existing one (`R`
/// in the Database List view; holds the database's current file name).
#[derive(PartialEq, Debug, Clone)]
pub(crate) enum DbNamePurpose {
    Create,
    Rename(String),
    /// The same input is used to name a new folder / rename a folder.
    CreateFolder,
    RenameFolder(String),
}

/// Whether the folder list is being browsed, or used to pick the folder
/// that database(s) are moved into (`m` in a folder).
#[derive(PartialEq, Debug, Clone, Copy)]
pub(crate) enum FolderListPurpose {
    Browse,
    MoveDatabases,
}

/// One row of the folder list.
#[derive(PartialEq, Debug, Clone)]
pub(crate) enum FolderRow {
    Folder(String),
    /// The databases that are in no folder.
    Unfiled,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub(crate) enum EditField {
    Uri,
    Title,
    Tags,
}

pub(super) struct TerminalDimensions {
    pub(super) width: u16,
    pub(super) height: u16,
}

/// Feeds a terminal event to a text input. Same as the input's own
/// `handle_event`, except for Ctrl+u: the library's version wipes the whole
/// line, but in a terminal Ctrl+u deletes only what is to the *left* of the
/// cursor (like Ctrl+k deletes what is to the right).
pub(crate) fn handle_input_event(input: &mut tui_input::Input, event: &ratatui::crossterm::event::Event) {
    use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
    use tui_input::backend::crossterm::EventHandler;

    if let Event::Key(key) = event
        && key.kind != KeyEventKind::Release
        && key.modifiers == KeyModifiers::CONTROL
        && key.code == KeyCode::Char('u')
    {
        let rest: String = input.value().chars().skip(input.cursor()).collect();
        *input = tui_input::Input::new(rest).with_cursor(0);
        return;
    }

    input.handle_event(event);
}

#[cfg(test)]
mod db_view_tests {
    use super::DbView;

    #[test]
    fn digits_map_to_the_five_views() {
        assert_eq!(DbView::from_digit('1'), Some(DbView::Custom));
        assert_eq!(DbView::from_digit('2'), Some(DbView::NameAsc));
        assert_eq!(DbView::from_digit('3'), Some(DbView::NameDesc));
        assert_eq!(DbView::from_digit('4'), Some(DbView::CountAsc));
        assert_eq!(DbView::from_digit('5'), Some(DbView::CountDesc));
        assert_eq!(DbView::from_digit('6'), None);
        assert_eq!(DbView::from_digit('0'), None);
    }

    #[test]
    fn next_goes_round_all_five() {
        let mut view = DbView::Custom;
        let mut seen = vec![];
        for _ in 0..5 {
            view = view.next();
            seen.push(view);
        }
        assert_eq!(
            seen,
            vec![
                DbView::NameAsc,
                DbView::NameDesc,
                DbView::CountAsc,
                DbView::CountDesc,
                DbView::Custom
            ]
        );
    }
}
