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

    /// Opens `url` in a way that lands in its own new tab even when
    /// called several times in quick succession - used for bulk opens
    /// ("O"). On desktop, plain `open_url` already reopens the default
    /// browser and reliably gets a new tab per call, so this just reuses
    /// it (unlike Android, which needs an explicit "new tab" intent extra
    /// to behave the same way - see the termux module below).
    pub fn open_url_new_tab(url: &str) -> Result<(), String> {
        open_url(url)
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

    /// Opens `url` with whatever the system's actual default handler is
    /// for it - used for plain single-url opens ("o"), since it doesn't
    /// need any of the browser-specific control the functions below do.
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

    // Incognito mode and forced-new-tab opens both need to target a
    // specific browser's activity directly rather than "whatever handles
    // this URL" (which `termux-open-url` gives us, but doesn't let us
    // attach the extras needed below). Brave and Chrome (and most other
    // Chromium-based Android browsers) all ship this exact activity class
    // name, since they're built from the same underlying Chromium browser
    // code - only the package name differs between them.
    const ACTIVITY: &str = "com.google.android.apps.chrome.Main";

    // Tried in this order unless BMM_ANDROID_BROWSER_PACKAGE is set.
    // Brave first, since that's the common case; Chrome as a fallback.
    const BROWSER_PACKAGE_CANDIDATES: &[&str] = &[
        "com.brave.browser",
        "com.android.chrome",
        "com.brave.browser_beta",
        "com.brave.browser_nightly",
    ];

    fn is_package_installed(pkg: &str) -> bool {
        Command::new("pm")
            .args(["list", "packages", pkg])
            .output()
            .map(|o| {
                o.status.success()
                    && String::from_utf8_lossy(&o.stdout)
                        .lines()
                        .any(|line| line.trim() == format!("package:{pkg}"))
            })
            .unwrap_or(false)
    }

    fn browser_component() -> String {
        if let Ok(pkg) = std::env::var("BMM_ANDROID_BROWSER_PACKAGE") {
            let activity = std::env::var("BMM_ANDROID_BROWSER_ACTIVITY")
                .unwrap_or_else(|_| ACTIVITY.to_string());
            return format!("{pkg}/{activity}");
        }

        for pkg in BROWSER_PACKAGE_CANDIDATES {
            if is_package_installed(pkg) {
                return format!("{pkg}/{ACTIVITY}");
            }
        }

        // Couldn't confirm any candidate is installed (`pm list packages`
        // can be flaky/restricted under some Termux setups) - fall back
        // to the first candidate anyway; `am start` below will surface a
        // clear error if it genuinely isn't there.
        format!("{}/{ACTIVITY}", BROWSER_PACKAGE_CANDIDATES[0])
    }

    fn no_browser_hint(detail: &str) -> String {
        format!(
            "couldn't open the browser directly (needed for incognito mode / opening several \
links at once). By default bmm looks for Brave or Chrome; if you use a different Chromium-\
based browser, set BMM_ANDROID_BROWSER_PACKAGE to its package name (and \
BMM_ANDROID_BROWSER_ACTIVITY too, if it isn't \"{ACTIVITY}\"). ({detail})"
        )
    }

    /// Fires a VIEW intent at `url`, targeting the resolved browser
    /// component directly, with the given boolean extras (each set to
    /// "true") attached.
    fn am_start(url: &str, extra_bool_flags: &[&str]) -> Result<(), String> {
        let component = browser_component();

        let mut args: Vec<String> = vec![
            "start".to_string(),
            "-a".to_string(),
            "android.intent.action.VIEW".to_string(),
            "-d".to_string(),
            url.to_string(),
            "-n".to_string(),
            component,
        ];
        for flag in extra_bool_flags {
            args.push("--ez".to_string());
            args.push((*flag).to_string());
            args.push("true".to_string());
        }

        let output = Command::new("am")
            .args(&args)
            .output()
            .map_err(|e| no_browser_hint(&format!("couldn't run 'am start': {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        // `am start` almost always exits 0 even when it fails to find the
        // target component - the actual failure shows up as "Error: ..."
        // in stdout instead, so that has to be checked too, not just the
        // exit status.
        if output.status.success() && !stdout.contains("Error:") {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(no_browser_hint(&format!("am start said: {stdout}{stderr}")))
        }
    }

    /// Opens `url` in a new, regular (non-incognito) tab - used when
    /// opening several urls back-to-back ("O"), since firing plain
    /// `termux-open-url` intents in a loop can get coalesced into the
    /// same tab instead of landing one per url.
    pub fn open_url_new_tab(url: &str) -> Result<(), String> {
        am_start(url, &["create_new_tab"])
    }

    /// Opens `url` directly in a new incognito/private tab.
    ///
    /// This works by targeting the browser's activity directly with the
    /// undocumented (but long-stable, used by Chromium's own
    /// instrumentation tests) "IS_INCOGNITO" boolean extra, rather than
    /// firing the public "OPEN_PRIVATE_TAB" action. The latter is
    /// deliberately restricted to Google-signed callers (Chromium's
    /// `ChromeTabbedActivity` checks `isTrustedIntent()`), so it silently
    /// no-ops for third-party apps like Termux, and even when it does
    /// work it can only ever open a *blank* tab (no way to attach a url).
    /// Passing this extra alongside a normal VIEW intent sidesteps both
    /// problems: it isn't gated the same way, and the url travels with
    /// the intent instead of needing a clipboard hand-off.
    pub fn open_url_incognito(url: &str) -> Result<(), String> {
        am_start(
            url,
            &[
                "org.chromium.chrome.browser.document.IS_INCOGNITO",
                "create_new_tab",
            ],
        )
    }
}

#[cfg(not(target_os = "android"))]
pub use desktop::{copy_to_clipboard, open_url, open_url_incognito, open_url_new_tab};

#[cfg(target_os = "android")]
pub use termux::{copy_to_clipboard, open_url, open_url_incognito, open_url_new_tab};
