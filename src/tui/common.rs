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
