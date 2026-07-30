//! Backup/restore for bmm's local databases, to a plain folder *outside*
//! of bmm's own data directory - easy to find, and easy to sync off-device
//! by hand or with something else (an Android file-sync app, Syncthing, a
//! synced cloud folder, ...).
//!
//! - `Alt+b` (works from anywhere) copies every `*.db` file out of bmm's
//!   data directory into that folder, creating the folder if it doesn't
//!   exist yet, and overwriting any file there that shares a database's
//!   name.
//! - `Alt+g` (works from anywhere) copies every `*.db` file the other
//!   way, from that folder back into bmm's data directory, again
//!   overwriting same-named files.
//!
//! Both directions use the exact same folder, so backing up on one device
//! and restoring on another (or the same device after a fresh install)
//! uses the same folder either way. Its exact location depends on the
//! platform, since there's no single "obvious downloads folder" shared by
//! desktop and Android:
//!   - Android:                ~/sdcard/Download(s)/links
//!   - Windows:                C:\links
//!   - everywhere else (Linux/macOS): ~/Download(s)/links
//!
//! On Android and Linux/macOS, the "downloads" folder name itself isn't
//! fixed - some systems use "Download", others "Downloads", and casing can
//! vary too. Rather than hard-coding one spelling, bmm looks for an
//! existing folder matching "download" or "downloads" case-insensitively
//! (preferring "downloads" if both exist), and only creates a new
//! "Downloads" folder if neither is found.
//!
//! Note: this copies the underlying files directly rather than merging
//! records inside the databases. Restoring over a database that's
//! currently open (e.g. the active one, or one open in another bmm
//! session) is safe on the filesystem, but that session won't see the
//! change until it's restarted.

use std::fs;
use std::path::{Path, PathBuf};

/// Looks inside `parent` for an existing subfolder matching "download" or
/// "downloads", case-insensitively (so "download", "Download", "downloads",
/// "Downloads", "DOWNLOADS", etc. are all recognized). If both a
/// "download"-like and a "downloads"-like folder exist, the "downloads"
/// one is preferred. If neither exists (or `parent` can't be read), falls
/// back to "Downloads" so callers can create it fresh.
fn find_downloads_folder_name(parent: &Path) -> String {
    let mut download_match: Option<String> = None;
    let mut downloads_match: Option<String> = None;

    if let Ok(entries) = fs::read_dir(parent) {
        for entry in entries.flatten() {
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if !is_dir {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            match name.to_lowercase().as_str() {
                "downloads" => downloads_match = Some(name),
                "download" => download_match = Some(name),
                _ => {}
            }
        }
    }

    downloads_match
        .or(download_match)
        .unwrap_or_else(|| "Downloads".to_string())
}

/// The one folder both `Alt+b` and `Alt+g` read/write, chosen per platform.
pub(super) fn links_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(PathBuf::from(r"C:\links"))
    }

    #[cfg(target_os = "android")]
    {
        let home = dirs::home_dir()
            .ok_or_else(|| "couldn't determine your home directory".to_string())?;
        let sdcard = home.join("sdcard");
        let downloads_name = find_downloads_folder_name(&sdcard);
        Ok(sdcard.join(downloads_name).join("links"))
    }

    #[cfg(not(any(target_os = "windows", target_os = "android")))]
    {
        let home = dirs::home_dir()
            .ok_or_else(|| "couldn't determine your home directory".to_string())?;
        let downloads_name = find_downloads_folder_name(&home);
        Ok(home.join(downloads_name).join("links"))
    }
}

/// Copies every `*.db` file directly inside `from_dir` into `to_dir`,
/// creating `to_dir` if it doesn't exist. Same-named files already in
/// `to_dir` are overwritten. Returns how many files were copied.
fn copy_db_files(from_dir: &Path, to_dir: &Path) -> Result<usize, String> {
    fs::create_dir_all(to_dir)
        .map_err(|e| format!("couldn't create {}: {e}", to_dir.display()))?;

    let entries = fs::read_dir(from_dir)
        .map_err(|e| format!("couldn't read {}: {e}", from_dir.display()))?;

    let mut count = 0usize;
    let mut errors: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("db") {
            continue;
        }
        let Some(name) = path.file_name() else {
            continue;
        };

        let dest = to_dir.join(name);
        match fs::copy(&path, &dest) {
            Ok(_) => count += 1,
            Err(e) => errors.push(format!("{}: {e}", path.display())),
        }
    }

    if errors.is_empty() {
        Ok(count)
    } else if count == 0 {
        Err(errors.join("; "))
    } else {
        Err(format!("copied {count}, but failed for: {}", errors.join("; ")))
    }
}

/// `Alt+b` - copies every database out of bmm's data directory into the
/// platform's links folder. Returns the number of databases copied and
/// the folder they were copied to.
pub(super) fn backup_databases() -> Result<(usize, PathBuf), String> {
    let data_dir = crate::utils::get_data_dir()
        .map_err(|e| format!("couldn't determine bmm's data directory: {e}"))?;
    let bmm_dir = data_dir.join("bmm");
    let dest = links_dir()?;

    let count = copy_db_files(&bmm_dir, &dest)?;
    Ok((count, dest))
}

/// `Alt+g` - copies every database found in the platform's links folder
/// back into bmm's data directory, overwriting any that share a name.
/// Returns the number of databases copied and the folder they came from.
pub(super) fn restore_databases() -> Result<(usize, PathBuf), String> {
    let data_dir = crate::utils::get_data_dir()
        .map_err(|e| format!("couldn't determine bmm's data directory: {e}"))?;
    let bmm_dir = data_dir.join("bmm");
    let src = links_dir()?;

    fs::create_dir_all(&bmm_dir)
        .map_err(|e| format!("couldn't create {}: {e}", bmm_dir.display()))?;

    let count = copy_db_files(&src, &bmm_dir)?;
    Ok((count, src))
}
