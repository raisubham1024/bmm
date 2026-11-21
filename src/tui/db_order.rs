//! Helpers for the Database List view: remembering the order the user
//! arranged their databases in (`a` in the Database List), and the small
//! pure functions behind arranging and the `Alt+<number>` jump.
//!
//! The order is kept in a plain text file (one database file name per
//! line) next to the databases themselves. Its extension is not `.db`, so
//! it's never mistaken for a database by any of the "list every database"
//! code paths.

use std::path::PathBuf;

const ORDER_FILE_NAME: &str = "db_order.txt";

/// The database bmm opens at every start.
pub(super) const DEFAULT_DB_NAME: &str = "bmm.db";

fn order_file_path() -> Option<PathBuf> {
    let data_dir = crate::utils::get_data_dir().ok()?;
    Some(data_dir.join("bmm").join(ORDER_FILE_NAME))
}

/// Reads the saved order. A missing/unreadable file just means "no saved
/// order yet" and yields an empty list.
pub(super) fn load_order() -> Vec<String> {
    let Some(path) = order_file_path() else {
        return vec![];
    };
    let Ok(contents) = std::fs::read_to_string(path) else {
        return vec![];
    };

    contents
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

pub(super) fn save_order(order: &[String]) -> Result<(), String> {
    let path = order_file_path().ok_or_else(|| "couldn't find bmm's data directory".to_string())?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut contents = order.join("\n");
    contents.push('\n');

    std::fs::write(path, contents).map_err(|e| e.to_string())
}

/// Puts `dbs` in the order given by `order`: databases named in `order`
/// come first (in that order), every other database follows,
/// alphabetically. Names in `order` that aren't in `dbs` (eg. a database
/// that was deleted by hand) are ignored, and duplicates are dropped.
pub(super) fn apply_order(dbs: Vec<String>, order: &[String]) -> Vec<String> {
    let mut remaining = dbs;
    remaining.sort();
    remaining.dedup();

    let mut result: Vec<String> = Vec::with_capacity(remaining.len());

    for name in order {
        if let Some(pos) = remaining.iter().position(|d| d == name) {
            result.push(remaining.remove(pos));
        }
    }

    result.extend(remaining);
    result
}

/// Returns a copy of `list` with the entries at positions `a` and `b`
/// swapped - this is how "arranging" works: the database moved to a new
/// number takes that number, and the database that used to have that
/// number takes the moved database's old number. Out-of-range positions
/// leave the list unchanged.
pub(super) fn swapped(list: &[String], a: usize, b: usize) -> Vec<String> {
    let mut result = list.to_vec();
    if a < result.len() && b < result.len() {
        result.swap(a, b);
    }
    result
}

/// Replaces `old` with `new` in `order` (if present), keeping its position.
pub(super) fn rename_in_order(order: &[String], old: &str, new: &str) -> Vec<String> {
    order
        .iter()
        .map(|n| if n == old { new.to_string() } else { n.clone() })
        .collect()
}

/// What to do after one more digit was typed with Alt held down in the
/// Database List view.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum AltNumber {
    /// The typed number can't be extended into another valid database
    /// number, so jump to this 0-based index right away.
    Jump(usize),
    /// More digits could still follow (eg. "1" when there are 12
    /// databases: could be 1, or the start of 10-12), so wait a moment.
    Wait,
    /// No database has this number.
    Invalid,
}

pub(super) fn resolve_alt_number(typed: &str, db_count: usize) -> AltNumber {
    let Ok(number) = typed.parse::<usize>() else {
        return AltNumber::Invalid;
    };

    if number == 0 || number > db_count {
        return AltNumber::Invalid;
    }

    if number.saturating_mul(10) <= db_count {
        AltNumber::Wait
    } else {
        AltNumber::Jump(number - 1)
    }
}

/// Validates a database name typed by the user (used when renaming) and
/// returns the file name to use (with ".db" added if missing).
pub(super) fn normalize_db_name(raw: &str) -> Result<String, String> {
    let raw = raw.trim();

    if raw.is_empty() {
        return Err("database name can't be empty".to_string());
    }

    let name = if raw.to_lowercase().ends_with(".db") {
        raw.to_string()
    } else {
        format!("{raw}.db")
    };

    let stem = &name[..name.len() - 3];
    if stem.trim().is_empty() {
        return Err("database name can't be empty".to_string());
    }

    if name.starts_with('.') {
        return Err("database name can't start with a dot".to_string());
    }

    if name
        .chars()
        .any(|c| c == '/' || c == '\\' || c == '\0' || c.is_control())
    {
        return Err("database name can't contain slashes or control characters".to_string());
    }

    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn apply_order_puts_saved_first_then_rest_alphabetically() {
        let dbs = v(&["a.db", "b.db", "c.db", "d.db"]);
        let order = v(&["c.db", "a.db"]);
        assert_eq!(
            apply_order(dbs, &order),
            v(&["c.db", "a.db", "b.db", "d.db"])
        );
    }

    #[test]
    fn apply_order_ignores_missing_and_duplicate_names() {
        let dbs = v(&["a.db", "b.db"]);
        let order = v(&["gone.db", "b.db", "b.db"]);
        assert_eq!(apply_order(dbs, &order), v(&["b.db", "a.db"]));
    }

    #[test]
    fn swapped_swaps_two_positions() {
        let list = v(&["1", "2", "3", "4", "5", "6", "7"]);
        // moving the 7th database to the 3rd position: the 3rd one goes to 7th
        assert_eq!(
            swapped(&list, 6, 2),
            v(&["1", "2", "7", "4", "5", "6", "3"])
        );
    }

    #[test]
    fn swapped_out_of_range_is_noop() {
        let list = v(&["1", "2"]);
        assert_eq!(swapped(&list, 0, 5), list);
    }

    #[test]
    fn rename_in_order_keeps_position() {
        assert_eq!(
            rename_in_order(&v(&["a.db", "b.db"]), "a.db", "z.db"),
            v(&["z.db", "b.db"])
        );
    }

    #[test]
    fn alt_number_jumps_immediately_when_unambiguous() {
        assert_eq!(resolve_alt_number("3", 5), AltNumber::Jump(2));
        assert_eq!(resolve_alt_number("2", 12), AltNumber::Jump(1));
        assert_eq!(resolve_alt_number("12", 12), AltNumber::Jump(11));
    }

    #[test]
    fn alt_number_waits_when_more_digits_could_follow() {
        assert_eq!(resolve_alt_number("1", 12), AltNumber::Wait);
        assert_eq!(resolve_alt_number("1", 10), AltNumber::Wait);
        assert_eq!(resolve_alt_number("10", 10), AltNumber::Jump(9));
    }

    #[test]
    fn alt_number_rejects_out_of_range() {
        assert_eq!(resolve_alt_number("13", 12), AltNumber::Invalid);
        assert_eq!(resolve_alt_number("0", 12), AltNumber::Invalid);
        assert_eq!(resolve_alt_number("6", 5), AltNumber::Invalid);
        assert_eq!(resolve_alt_number("", 5), AltNumber::Invalid);
    }

    #[test]
    fn normalize_db_name_adds_extension_and_validates() {
        assert_eq!(normalize_db_name(" work "), Ok("work.db".to_string()));
        assert_eq!(normalize_db_name("work.db"), Ok("work.db".to_string()));
        assert!(normalize_db_name("").is_err());
        assert!(normalize_db_name("   ").is_err());
        assert!(normalize_db_name(".db").is_err());
        assert!(normalize_db_name("a/b").is_err());
        assert!(normalize_db_name("a\\b").is_err());
        assert!(normalize_db_name(".hidden").is_err());
    }
}
