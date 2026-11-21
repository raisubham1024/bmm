//! Folders for databases (the `F` view in the TUI).
//!
//! A folder is only a label bmm keeps to group databases ("learning" could
//! hold "learning_linux.db" and "learning_networking.db"): nothing is
//! created on disk besides one small text file, `folders.txt`, next to the
//! databases - the database files themselves never move.
//!
//! File format, one entry per line:
//!
//! ```text
//! F<TAB>learning          <- a folder
//! D<TAB>learning_linux.db <- a database inside the folder above
//! ```
//!
//! A database is in at most one folder; databases in none are "unfiled".

use std::path::PathBuf;

const FOLDERS_FILE_NAME: &str = "folders.txt";

/// Shown for the databases that aren't in any folder.
pub(super) const UNFILED_LABEL: &str = "(no folder)";

const MAX_FOLDER_NAME_CHARS: usize = 60;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct Folder {
    pub(super) name: String,
    pub(super) databases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct Folders {
    pub(super) folders: Vec<Folder>,
}

fn folders_file_path() -> Option<PathBuf> {
    let data_dir = crate::utils::get_data_dir().ok()?;
    Some(data_dir.join("bmm").join(FOLDERS_FILE_NAME))
}

impl Folders {
    pub(super) fn load() -> Self {
        let Some(path) = folders_file_path() else {
            return Self::default();
        };
        let Ok(contents) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        Self::parse(&contents)
    }

    pub(super) fn save(&self) -> Result<(), String> {
        let path =
            folders_file_path().ok_or_else(|| "couldn't find bmm's data directory".to_string())?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        std::fs::write(path, self.serialize()).map_err(|e| e.to_string())
    }

    pub(super) fn parse(contents: &str) -> Self {
        let mut folders: Vec<Folder> = Vec::new();

        for line in contents.lines() {
            let Some((kind, value)) = line.split_once('\t') else {
                continue;
            };
            let value = value.trim();
            if value.is_empty() {
                continue;
            }

            match kind {
                "F" => {
                    if !folders.iter().any(|f| f.name == value) {
                        folders.push(Folder {
                            name: value.to_string(),
                            databases: vec![],
                        });
                    }
                }
                "D" => {
                    // a database belongs to one folder only: the first wins
                    let already_filed = folders.iter().any(|f| f.databases.iter().any(|d| d == value));
                    if let Some(folder) = folders.last_mut()
                        && !already_filed
                    {
                        folder.databases.push(value.to_string());
                    }
                }
                _ => {}
            }
        }

        Self { folders }
    }

    pub(super) fn serialize(&self) -> String {
        let mut out = String::new();
        for folder in &self.folders {
            out.push_str(&format!("F\t{}\n", folder.name));
            for db in &folder.databases {
                out.push_str(&format!("D\t{db}\n"));
            }
        }
        out
    }

    pub(super) fn index_of(&self, name: &str) -> Option<usize> {
        self.folders.iter().position(|f| f.name == name)
    }

    /// Name of the folder holding this database, if any.
    pub(super) fn folder_of(&self, database: &str) -> Option<&str> {
        self.folders
            .iter()
            .find(|f| f.databases.iter().any(|d| d == database))
            .map(|f| f.name.as_str())
    }

    /// Databases (out of `all`) that are in no folder, keeping `all`'s order.
    pub(super) fn unfiled(&self, all: &[String]) -> Vec<String> {
        all.iter()
            .filter(|d| self.folder_of(d).is_none())
            .cloned()
            .collect()
    }

    /// Checks a folder name typed by the user and returns it cleaned up.
    /// `renaming` is the folder's current name when renaming (so that
    /// keeping the name, or only changing its case, isn't a clash).
    pub(super) fn validate_name(&self, raw: &str, renaming: Option<&str>) -> Result<String, String> {
        let name = raw.trim();

        if name.is_empty() {
            return Err("folder name can't be empty".to_string());
        }
        if name.chars().count() > MAX_FOLDER_NAME_CHARS {
            return Err(format!(
                "folder name can't be longer than {MAX_FOLDER_NAME_CHARS} characters"
            ));
        }
        if name.chars().any(|c| c.is_control()) {
            return Err("folder name can't contain control characters".to_string());
        }
        if name.to_lowercase() == UNFILED_LABEL.to_lowercase() {
            return Err(format!("\"{UNFILED_LABEL}\" is reserved, pick another name"));
        }

        let clash = self
            .folders
            .iter()
            .any(|f| f.name.to_lowercase() == name.to_lowercase() && Some(f.name.as_str()) != renaming);
        if clash {
            return Err("a folder with this name already exists".to_string());
        }

        Ok(name.to_string())
    }

    /// Adds an empty folder at the end. The name must already be validated.
    pub(super) fn add(&mut self, name: String) {
        self.folders.push(Folder {
            name,
            databases: vec![],
        });
    }

    /// Renames a folder, keeping its position and databases.
    pub(super) fn rename(&mut self, old: &str, new: String) {
        if let Some(folder) = self.folders.iter_mut().find(|f| f.name == old) {
            folder.name = new;
        }
    }

    /// Removes a folder; the databases in it become unfiled. Returns how
    /// many databases that was.
    pub(super) fn delete(&mut self, name: &str) -> usize {
        match self.index_of(name) {
            Some(i) => self.folders.remove(i).databases.len(),
            None => 0,
        }
    }

    /// Puts the databases in `target` (or takes them out of every folder
    /// when `target` is `None`), removing them from wherever they were.
    pub(super) fn move_databases(&mut self, databases: &[String], target: Option<&str>) {
        for folder in self.folders.iter_mut() {
            folder.databases.retain(|d| !databases.contains(d));
        }

        if let Some(target) = target
            && let Some(folder) = self.folders.iter_mut().find(|f| f.name == target)
        {
            for db in databases {
                if !folder.databases.contains(db) {
                    folder.databases.push(db.clone());
                }
            }
        }
    }

    /// A database was renamed: it stays in its folder.
    pub(super) fn rename_database(&mut self, old: &str, new: &str) {
        for folder in self.folders.iter_mut() {
            for db in folder.databases.iter_mut() {
                if db == old {
                    *db = new.to_string();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn serialize_and_parse_round_trip() {
        let mut f = Folders::default();
        f.add("learning 📚".to_string());
        f.add("work".to_string());
        f.move_databases(&v(&["linux.db", "net.db"]), Some("learning 📚"));

        let parsed = Folders::parse(&f.serialize());
        assert_eq!(parsed, f);
        assert_eq!(parsed.folder_of("net.db"), Some("learning 📚"));
        assert_eq!(parsed.folder_of("other.db"), None);
    }

    #[test]
    fn a_database_lives_in_one_folder_only() {
        let mut f = Folders::default();
        f.add("a".to_string());
        f.add("b".to_string());
        f.move_databases(&v(&["x.db"]), Some("a"));
        f.move_databases(&v(&["x.db"]), Some("b"));

        assert!(f.folders[0].databases.is_empty());
        assert_eq!(f.folders[1].databases, v(&["x.db"]));

        f.move_databases(&v(&["x.db"]), None);
        assert_eq!(f.folder_of("x.db"), None);
    }

    #[test]
    fn parse_ignores_junk_and_duplicates() {
        let f = Folders::parse("junk\nF\ta\nD\tx.db\nF\ta\nD\tx.db\nF\tb\nD\tx.db\nD\t\n");
        assert_eq!(f.folders.len(), 2);
        assert_eq!(f.folders[0].databases, v(&["x.db"]));
        assert!(f.folders[1].databases.is_empty());
    }

    #[test]
    fn unfiled_keeps_order() {
        let mut f = Folders::default();
        f.add("a".to_string());
        f.move_databases(&v(&["b.db"]), Some("a"));
        assert_eq!(f.unfiled(&v(&["c.db", "b.db", "a.db"])), v(&["c.db", "a.db"]));
    }

    #[test]
    fn name_validation() {
        let mut f = Folders::default();
        f.add("Learning".to_string());

        assert_eq!(f.validate_name("  work  ", None), Ok("work".to_string()));
        assert!(f.validate_name("", None).is_err());
        assert!(f.validate_name("learning", None).is_err());
        assert!(f.validate_name("(No Folder)", None).is_err());
        // renaming a folder to a different case of its own name is fine
        assert_eq!(
            f.validate_name("learning", Some("Learning")),
            Ok("learning".to_string())
        );
        assert_eq!(f.validate_name("🚀 rust", None), Ok("🚀 rust".to_string()));
    }

    #[test]
    fn delete_unfiles_databases() {
        let mut f = Folders::default();
        f.add("a".to_string());
        f.move_databases(&v(&["x.db", "y.db"]), Some("a"));
        assert_eq!(f.delete("a"), 2);
        assert_eq!(f.folder_of("x.db"), None);
        assert_eq!(f.delete("a"), 0);
    }

    #[test]
    fn rename_database_keeps_folder() {
        let mut f = Folders::default();
        f.add("a".to_string());
        f.move_databases(&v(&["x.db"]), Some("a"));
        f.rename_database("x.db", "z.db");
        assert_eq!(f.folder_of("z.db"), Some("a"));
        assert_eq!(f.folder_of("x.db"), None);
    }
}
