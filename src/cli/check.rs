use crate::persistence::DBError;
use crate::persistence::get_bookmarks;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

const USER_AGENT: &str = "bmm-link-checker";

#[derive(thiserror::Error, Debug)]
pub enum CheckBookmarksError {
    #[error("couldn't get bookmarks from db: {0}")]
    CouldntGetBookmarksFromDB(DBError),
    #[error("couldn't set up http client: {0}")]
    CouldntBuildHttpClient(reqwest::Error),
}

#[derive(Debug)]
enum LinkStatus {
    Ok(reqwest::StatusCode),
    Broken(String),
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

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout.max(1) as u64))
        .user_agent(USER_AGENT)
        .build()
        .map_err(CheckBookmarksError::CouldntBuildHttpClient)?;

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
                Ok(_permit) => check_single_link(&client, &uri).await,
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

async fn check_single_link(client: &reqwest::Client, uri: &str) -> LinkStatus {
    // HEAD is cheaper, but some servers don't support it properly, so fall
    // back to a GET request if it doesn't succeed.
    match client.head(uri).send().await {
        Ok(resp) if resp.status().is_success() || resp.status().is_redirection() => {
            LinkStatus::Ok(resp.status())
        }
        _ => match client.get(uri).send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() || status.is_redirection() {
                    LinkStatus::Ok(status)
                } else {
                    LinkStatus::Broken(format!("HTTP {status}"))
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    LinkStatus::Broken("timed out".to_string())
                } else if e.is_connect() {
                    LinkStatus::Broken("couldn't connect".to_string())
                } else if e.is_builder() {
                    LinkStatus::Broken("invalid URI".to_string())
                } else {
                    LinkStatus::Broken("request failed".to_string())
                }
            }
        },
    }
}
