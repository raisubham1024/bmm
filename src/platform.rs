//! Platform-specific handling for clipboard access and "open URL in browser".
//!
//! `arboard` (clipboard) and `open` (launch default app for a URL) both assume
//! a desktop windowing system (X11/Wayland/Win32/Cocoa). On Android/Termux
//! there is no such system, so:
//!   - `arboard` fails to even *compile* (it pulls in X11 dev headers that
//!     don't exist / don't make sense on Termux), and
//!   - `open::that` has nothing sensible to shell out to at runtime.
//!
//! On Android we instead shell out to the `termux-clipboard-set` and
//! `termux-open-url` commands, which come from the Termux:API app + the
//! `termux-api` package (`pkg install termux-api`).
//!
//! Everywhere else, behavior is unchanged (arboard + open, same as upstream).

#[cfg(not(target_os = "android"))]
mod desktop {
    use arboard::Clipboard;

    pub fn copy_to_clipboard(content: &str) -> Result<(), String> {
        let mut clipboard =
            Clipboard::new().map_err(|e| format!("couldn't get system clipboard: {e}"))?;
        clipboard.set_text(content).map_err(|e| e.to_string())
    }

    pub fn open_url(url: &str) -> Result<(), String> {
        open::that(url).map_err(|e| e.to_string())
    }
}

#[cfg(target_os = "android")]
mod termux {
    use std::io::Write;
    use std::process::{Command, Stdio};

    const INSTALL_HINT: &str =
        "termux-api command not found. Install the 'Termux:API' app from F-Droid/Play Store, \
then run: pkg install termux-api";

    pub fn copy_to_clipboard(content: &str) -> Result<(), String> {
        let mut child = Command::new("termux-clipboard-set")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("couldn't run termux-clipboard-set ({INSTALL_HINT}): {e}"))?;

        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(content.as_bytes())
                .map_err(|e| format!("couldn't write to termux-clipboard-set: {e}"))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("couldn't wait on termux-clipboard-set: {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("termux-clipboard-set failed: {stderr}"))
        }
    }

    pub fn open_url(url: &str) -> Result<(), String> {
        let output = Command::new("termux-open-url")
            .arg(url)
            .output()
            .map_err(|e| format!("couldn't run termux-open-url ({INSTALL_HINT}): {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("termux-open-url failed: {stderr}"))
        }
    }
}

#[cfg(not(target_os = "android"))]
pub use desktop::{copy_to_clipboard, open_url};

#[cfg(target_os = "android")]
pub use termux::{copy_to_clipboard, open_url};
