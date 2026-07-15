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

    const NO_BROWSER_HINT: &str = "couldn't find a supported browser for incognito/private \
mode (tried chrome, chromium, brave, edge, firefox); set the BMM_BROWSER env var to your \
browser's executable name (and optionally BMM_BROWSER_INCOGNITO_FLAG if it isn't a \
chromium/firefox-style browser)";

    #[cfg(not(target_os = "macos"))]
    const BROWSER_CANDIDATES: &[(&str, &str)] = &[
        ("google-chrome", "--incognito"),
        ("google-chrome-stable", "--incognito"),
        ("chromium", "--incognito"),
        ("chromium-browser", "--incognito"),
        ("brave-browser", "--incognito"),
        ("microsoft-edge", "--inprivate"),
        ("msedge", "--inprivate"),
        ("firefox", "--private-window"),
        ("firefox-esr", "--private-window"),
        // in case any of the above are only reachable under their Windows exe name
        ("chrome.exe", "--incognito"),
        ("firefox.exe", "--private-window"),
        ("msedge.exe", "--inprivate"),
    ];

    #[cfg(not(target_os = "macos"))]
    pub fn open_url_incognito(url: &str) -> Result<(), String> {
        if let Ok(custom_browser) = std::env::var("BMM_BROWSER") {
            let flag = std::env::var("BMM_BROWSER_INCOGNITO_FLAG")
                .unwrap_or_else(|_| "--incognito".to_string());
            return launch(&custom_browser, &flag, url);
        }

        for (exe, flag) in BROWSER_CANDIDATES {
            if which::which(exe).is_ok() {
                return launch(exe, flag, url);
            }
        }

        Err(NO_BROWSER_HINT.to_string())
    }

    #[cfg(not(target_os = "macos"))]
    fn launch(exe: &str, flag: &str, url: &str) -> Result<(), String> {
        std::process::Command::new(exe)
            .arg(flag)
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("couldn't launch {exe}: {e}"))
    }

    // On macOS, browsers are app bundles rather than PATH executables, so we
    // launch them via `open -na "<App Name>" --args <flag> <url>` instead.
    #[cfg(target_os = "macos")]
    const MACOS_BROWSER_CANDIDATES: &[(&str, &str)] = &[
        ("Google Chrome", "--incognito"),
        ("Brave Browser", "--incognito"),
        ("Chromium", "--incognito"),
        ("Microsoft Edge", "--inprivate"),
        ("Firefox", "--private-window"),
    ];

    #[cfg(target_os = "macos")]
    pub fn open_url_incognito(url: &str) -> Result<(), String> {
        if let Ok(custom_browser) = std::env::var("BMM_BROWSER") {
            let flag = std::env::var("BMM_BROWSER_INCOGNITO_FLAG")
                .unwrap_or_else(|_| "--incognito".to_string());
            return launch_macos(&custom_browser, &flag, url);
        }

        for (app, flag) in MACOS_BROWSER_CANDIDATES {
            let path = format!("/Applications/{app}.app");
            if std::path::Path::new(&path).exists() {
                return launch_macos(app, flag, url);
            }
        }

        Err(NO_BROWSER_HINT.to_string())
    }

    #[cfg(target_os = "macos")]
    fn launch_macos(app: &str, flag: &str, url: &str) -> Result<(), String> {
        std::process::Command::new("open")
            .args(["-na", app, "--args", flag, url])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("couldn't launch {app}: {e}"))
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

    // There's no generic "open incognito" intent on Android; this uses
    // Chrome's specific incognito-tab intent extra, which is the most
    // common default browser. If your default browser isn't Chrome, this
    // will likely fail or silently open a normal tab instead.
    pub fn open_url_incognito(url: &str) -> Result<(), String> {
        let output = Command::new("am")
            .args([
                "start",
                "-a",
                "android.intent.action.VIEW",
                "-d",
                url,
                "--ez",
                "com.google.android.apps.chrome.EXTRA_OPEN_NEW_INCOGNITO_TAB",
                "true",
                "-n",
                "com.android.chrome/com.google.android.apps.chrome.Main",
            ])
            .output()
            .map_err(|e| format!("couldn't run 'am start': {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!(
                "couldn't open incognito tab (this only works if Chrome is installed as \
com.android.chrome): {stderr}"
            ))
        }
    }
}

#[cfg(not(target_os = "android"))]
pub use desktop::{copy_to_clipboard, open_url, open_url_incognito};

#[cfg(target_os = "android")]
pub use termux::{copy_to_clipboard, open_url, open_url_incognito};
