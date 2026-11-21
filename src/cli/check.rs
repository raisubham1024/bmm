use crate::persistence::DBError;
use crate::persistence::get_bookmarks;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

// A browser-like user agent: plenty of sites turn away unknown bots with an
// error even though the page opens fine in a browser.
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36 bmm-link-checker";

#[derive(thiserror::Error, Debug)]
pub enum CheckBookmarksError {
    #[error("couldn't get bookmarks from db: {0}")]
    CouldntGetBookmarksFromDB(DBError),
    #[error("couldn't set up http client: {0}")]
    CouldntBuildHttpClient(reqwest::Error),
}

#[derive(Debug)]
pub(crate) enum LinkStatus {
    /// The link opens (possibly after redirects), or the server answered
    /// in a way that shows the page exists but won't talk to a bot
    /// (401/403/429...).
    Ok(reqwest::StatusCode),
    /// Not an http(s) link (mailto:, ftp:...), so there's nothing to test.
    Skipped(String),
    /// The link can't be opened: the reason is a short description.
    Broken(String),
}

pub(crate) fn build_client(timeout_secs: u64) -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs.max(1)))
        .user_agent(USER_AGENT)
        .build()
}

pub async fn check_bookmarks(
    pool: &Pool<Sqlite>,
    uri: Option<String>,
    tags: Vec<String>,
    limit: u16,
    concurrency: u16,
    timeout: u16,
    show_all: bool,
) -> Result<(), CheckBookmarksError> {
    let bookmarks = get_bookmarks(pool, uri, None, tags, limit)
        .await
        .map_err(CheckBookmarksError::CouldntGetBookmarksFromDB)?;

    if bookmarks.is_empty() {
        println!("no bookmarks found to check");
        return Ok(());
    }

    let client = build_client(timeout as u64).map_err(CheckBookmarksError::CouldntBuildHttpClient)?;

    let total = bookmarks.len();
    let noun = if total == 1 { "bookmark" } else { "bookmarks" };
    println!("checking {total} {noun} ({concurrency} at a time)...\n");

    let semaphore = Arc::new(Semaphore::new(concurrency.max(1) as usize));
    let mut handles = Vec::with_capacity(total);

    for bookmark in bookmarks {
        let client = client.clone();
        let semaphore = Arc::clone(&semaphore);
        let uri = bookmark.uri.clone();

        let handle = tokio::spawn(async move {
            let permit = semaphore.acquire_owned().await;
            let status = match permit {
                Ok(_permit) => check_link(&client, &uri).await,
                Err(_) => LinkStatus::Broken("internal error: couldn't schedule check".into()),
            };

            (uri, status)
        });

        handles.push(handle);
    }

    let mut ok_count: usize = 0;
    let mut broken_count: usize = 0;
    let mut broken_uris: Vec<String> = Vec::new();

    for handle in handles {
        let (uri, status) = match handle.await {
            Ok(r) => r,
            Err(_) => continue,
        };

        match status {
            LinkStatus::Ok(code) => {
                ok_count += 1;
                if show_all {
                    println!("[OK]     {code} {uri}");
                }
            }
            LinkStatus::Skipped(reason) => {
                ok_count += 1;
                if show_all {
                    println!("[SKIP]   {reason} {uri}");
                }
            }
            LinkStatus::Broken(reason) => {
                broken_count += 1;
                println!("[BROKEN] {reason} {uri}");
                broken_uris.push(uri);
            }
        }
    }

    println!("\nchecked {total} {noun}: {ok_count} ok, {broken_count} broken");

    if !show_all && broken_uris.is_empty() {
        println!("(no broken links found; pass --show-all to see the full results)");
    }

    Ok(())
}

/// Checks whether one link can be opened.
///
/// Counted as broken: error statuses like 404/410/5xx, domains that don't
/// exist, refused connections, timeouts, bad certificates and redirect
/// loops. Statuses that only mean "this server won't talk to a bot" (401,
/// 403, 429, ...) count as working, since the page exists - flagging
/// those would bury the genuinely dead links under false alarms.
/// Timeouts and connection failures are retried once, as they're often
/// momentary.
pub(crate) async fn check_link(client: &reqwest::Client, uri: &str) -> LinkStatus {
    let parsed = match reqwest::Url::parse(uri) {
        Ok(u) => u,
        Err(_) => return LinkStatus::Broken("invalid URL".to_string()),
    };

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return LinkStatus::Skipped(format!("not a web link ({}:)", parsed.scheme()));
    }

    let mut status = check_link_once(client, uri).await;

    if let LinkStatus::Broken(reason) = &status
        && is_transient(reason)
    {
        status = check_link_once(client, uri).await;
    }

    status
}

fn is_transient(reason: &str) -> bool {
    // a timeout isn't retried: it already waited the full time once
    reason == "couldn't connect" || reason == "request failed"
}

fn is_alive_status(status: reqwest::StatusCode) -> bool {
    status.is_success()
        || status.is_redirection()
        || matches!(status.as_u16(), 401 | 403 | 407 | 429 | 451 | 999)
}

async fn check_link_once(client: &reqwest::Client, uri: &str) -> LinkStatus {
    // HEAD is cheaper, but some servers don't support it properly, so fall
    // back to a GET request if it doesn't succeed.
    match client.head(uri).send().await {
        Ok(resp) if resp.status().is_success() || resp.status().is_redirection() => {
            return LinkStatus::Ok(resp.status());
        }
        // the server can't be reached at all (or redirects forever), so
        // a GET to the same place would just wait and fail again
        Err(e) if e.is_timeout() || e.is_connect() || e.is_redirect() => {
            return LinkStatus::Broken(describe_error(&e));
        }
        _ => {}
    }

    match client.get(uri).send().await {
        Ok(resp) => {
            let status = resp.status();
            if is_alive_status(status) {
                LinkStatus::Ok(status)
            } else {
                LinkStatus::Broken(format!("HTTP {status}"))
            }
        }
        Err(e) => LinkStatus::Broken(describe_error(&e)),
    }
}

fn describe_error(e: &reqwest::Error) -> String {
    use std::error::Error;

    if e.is_timeout() {
        return "timed out".to_string();
    }
    if e.is_redirect() {
        return "too many redirects".to_string();
    }
    if e.is_builder() {
        return "invalid URL".to_string();
    }

    // the interesting detail (DNS, certificate...) is deeper in the chain
    let mut chain = e.to_string().to_lowercase();
    let mut source = e.source();
    while let Some(err) = source {
        chain.push(' ');
        chain.push_str(&err.to_string().to_lowercase());
        source = err.source();
    }

    if chain.contains("dns")
        || chain.contains("failed to lookup")
        || chain.contains("name or service not known")
        || chain.contains("no address associated")
        || chain.contains("nodename nor servname")
    {
        "domain not found (DNS)".to_string()
    } else if chain.contains("certificate") || chain.contains("tls") || chain.contains("ssl") {
        "SSL/TLS certificate error".to_string()
    } else if e.is_connect() {
        "couldn't connect".to_string()
    } else {
        "request failed".to_string()
    }
}
