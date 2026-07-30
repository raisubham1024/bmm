//! Self-updating for bmm's own binary.
//!
//! Both `bmm update` (CLI) and `Alt+u` (TUI, works from anywhere) do the
//! same thing:
//!
//! 1. Find every place `bmm` is actually installed, the same way
//!    `which -a bmm` would (i.e. every match on `$PATH`, not just the
//!    first one) - this matters because it's common to have more than one
//!    (e.g. a Homebrew-installed copy *and* a `~/.cargo/bin` copy).
//! 2. Check whether the binary available for download is actually
//!    different from what's already installed, *without* downloading it
//!    first - a cheap `HEAD` request is enough to compare sizes. If they
//!    match, nothing further happens and bmm reports "Already up to
//!    date". There's no point downloading and overwriting anything if
//!    nothing changed.
//! 3. If the sizes differ (or can't be compared), the new binary is
//!    downloaded once, then written to every location found in step 1,
//!    overwriting what's there. Each location's executable permission is
//!    checked, and only touched if it isn't already executable (so this
//!    behaves like `chmod +x` would, run just once per file).
//!
//! Only Linux (x86_64) and Android (aarch64) are supported right now,
//! matching the platforms bmm actually ships prebuilt binaries for.

use std::io;
use std::path::{Path, PathBuf};

#[cfg(target_os = "android")]
const DOWNLOAD_URL: &str =
    "https://github.com/raisubham1024/bmm/releases/download/final/bmm-android-aarch64";

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const DOWNLOAD_URL: &str =
    "https://github.com/raisubham1024/bmm/releases/download/final/bmm-linux-x86_64";

const BINARY_NAME: &str = "bmm";
const USER_AGENT: &str = "bmm-self-updater";

#[derive(thiserror::Error, Debug)]
pub enum UpdateError {
    // Only exists on platforms we don't ship a binary for - on Linux
    // x86_64 and Android aarch64, `platform_download_url()` always
    // succeeds, so this variant would otherwise never be constructed
    // there, which trips the `dead_code` lint on every "normal" build.
    #[cfg(not(any(target_os = "android", all(target_os = "linux", target_arch = "x86_64"))))]
    #[error(
        "self-updating isn't supported on this platform yet (only Linux x86_64 and Android \
aarch64 are); please update bmm manually"
    )]
    UnsupportedPlatform,
    #[error("couldn't set up http client: {0}")]
    CouldntBuildHttpClient(reqwest::Error),
    #[error("couldn't check for a new version: {0}")]
    CouldntCheckForUpdate(reqwest::Error),
    #[error("couldn't download the new version: {0}")]
    CouldntDownloadUpdate(reqwest::Error),
    #[error("couldn't read the downloaded binary: {0}")]
    CouldntReadDownloadedBinary(reqwest::Error),
    #[error("couldn't write to {path}: {source}")]
    CouldntWriteBinary { path: PathBuf, source: io::Error },
    #[error("couldn't set executable permissions on {path}: {source}")]
    CouldntSetPermissions { path: PathBuf, source: io::Error },
}

#[derive(Debug)]
pub enum UpdateOutcome {
    /// No `bmm` binary was found on `$PATH` (via the equivalent of
    /// `which -a bmm`), so there was nothing to check or update.
    NothingToUpdate,
    /// A `bmm` binary was found, but the one available for download looks
    /// identical to what's already installed - nothing was downloaded or
    /// overwritten.
    UpToDate,
    /// A new binary was downloaded and installed at every location
    /// listed here (overwriting what was there, and made executable).
    Updated { locations: Vec<PathBuf> },
}

/// Checks whether a newer `bmm` binary is available and, if so, downloads
/// it and installs it at every location found via the equivalent of
/// `which -a bmm`.
pub async fn update_bmm() -> Result<UpdateOutcome, UpdateError> {
    let locations: Vec<PathBuf> = which::which_all(BINARY_NAME)
        .map(|iter| iter.collect())
        .unwrap_or_default();

    if locations.is_empty() {
        return Ok(UpdateOutcome::NothingToUpdate);
    }

    let download_url = platform_download_url()?;

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(UpdateError::CouldntBuildHttpClient)?;

    // Cheap check first: a HEAD request's Content-Length tells us the
    // remote binary's size without downloading its body. If that matches
    // the size of what's already installed, there's nothing new, and we
    // skip the download entirely.
    let remote_len = client
        .head(download_url)
        .send()
        .await
        .map_err(UpdateError::CouldntCheckForUpdate)?
        .content_length();

    let installed_len = installed_binary_len(&locations);
    if let (Some(remote_len), Some(installed_len)) = (remote_len, installed_len) {
        if remote_len == installed_len {
            return Ok(UpdateOutcome::UpToDate);
        }
    }

    let response = client
        .get(download_url)
        .send()
        .await
        .map_err(UpdateError::CouldntDownloadUpdate)?;
    let bytes = response
        .bytes()
        .await
        .map_err(UpdateError::CouldntReadDownloadedBinary)?;

    let mut updated_locations = Vec::with_capacity(locations.len());
    for location in &locations {
        install_binary(location, &bytes)?;
        updated_locations.push(location.clone());
    }

    Ok(UpdateOutcome::Updated {
        locations: updated_locations,
    })
}

#[cfg(any(target_os = "android", all(target_os = "linux", target_arch = "x86_64")))]
fn platform_download_url() -> Result<&'static str, UpdateError> {
    Ok(DOWNLOAD_URL)
}

#[cfg(not(any(target_os = "android", all(target_os = "linux", target_arch = "x86_64"))))]
fn platform_download_url() -> Result<&'static str, UpdateError> {
    Err(UpdateError::UnsupportedPlatform)
}

/// Picks a location to read a size from for comparison against the
/// remote binary: the currently-running executable if it resolves and is
/// one of the locations `which -a bmm` found, otherwise just the first
/// location found. Returns `None` if that file's size can't be read,
/// which just means the size-based shortcut is skipped (the update
/// proceeds as if something may have changed).
fn installed_binary_len(locations: &[PathBuf]) -> Option<u64> {
    let candidate = std::env::current_exe()
        .ok()
        .filter(|p| locations.contains(p))
        .or_else(|| locations.first().cloned())?;

    std::fs::metadata(&candidate).ok().map(|m| m.len())
}

/// Overwrites `path` with `content`, then makes sure it's executable -
/// checking first, and only changing permissions if it isn't already.
fn install_binary(path: &Path, content: &[u8]) -> Result<(), UpdateError> {
    std::fs::write(path, content).map_err(|e| UpdateError::CouldntWriteBinary {
        path: path.to_path_buf(),
        source: e,
    })?;

    ensure_executable(path)?;

    Ok(())
}

#[cfg(unix)]
fn ensure_executable(path: &Path) -> Result<(), UpdateError> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path).map_err(|e| UpdateError::CouldntSetPermissions {
        path: path.to_path_buf(),
        source: e,
    })?;

    let mut perms = metadata.permissions();

    // Only the equivalent of `chmod +x` - add the execute bit for owner/
    // group/other on top of whatever's already there, and only if at
    // least one of those bits is missing. Files `which -a` finds are
    // already executable in virtually all cases, so this is mostly a
    // defensive check rather than something expected to trigger.
    if perms.mode() & 0o111 != 0o111 {
        perms.set_mode(perms.mode() | 0o111);
        std::fs::set_permissions(path, perms).map_err(|e| UpdateError::CouldntSetPermissions {
            path: path.to_path_buf(),
            source: e,
        })?;
    }

    Ok(())
}

#[cfg(not(unix))]
fn ensure_executable(_path: &Path) -> Result<(), UpdateError> {
    Ok(())
}
