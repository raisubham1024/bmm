use super::commands::Command;
use super::message::{Message, UrlsOpenedResult};
use crate::common::DEFAULT_LIMIT;
use crate::domain::{DraftBookmark, PotentialBookmark};
use crate::persistence::{
    SaveBookmarkOptions, create_or_update_bookmark, delete_bookmarks_with_uris, get_all_bookmarks,
    get_bookmarks, get_bookmarks_by_query, get_duplicate_bookmarks, get_note, get_tags_with_stats,
    rename_bookmark_uri, set_note,
};
use sqlx::{Pool, Sqlite};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::Sender;

pub(super) async fn handle_command(
    pool: &Pool<Sqlite>,
    command: Command,
    event_tx: Sender<Message>,
) {
    match command {
        // TODO: handle errors here
        Command::OpenInBrowser(url) => {
            tokio::spawn(async move {
                let message = match crate::platform::open_url(&url) {
                    Ok(_) => Message::UrlsOpenedInBrowser(UrlsOpenedResult::Success),
                    Err(e) => Message::UrlsOpenedInBrowser(UrlsOpenedResult::Failure(
                        std::io::Error::other(e),
                    )),
                };

                let _ = event_tx.try_send(message);
            });
        }
        Command::OpenMultipleInBrowser(urls) => {
            tokio::spawn(async move {
                let mut failures: Vec<String> = Vec::new();

                // On Android (Termux), firing "termux-open-url" back-to-back
                // with no gap can cause the underlying Android intent to be
                // dropped for all but the first url — the OS/Termux:API
                // hand-off needs a brief moment to actually complete before
                // the next one is fired. Desktop browsers don't have this
                // problem, so we leave that path untouched (zero delay).
                let delay_between_opens = if cfg!(target_os = "android") {
                    std::time::Duration::from_millis(400)
                } else {
                    std::time::Duration::ZERO
                };

                for (i, url) in urls.iter().enumerate() {
                    if i > 0 && !delay_between_opens.is_zero() {
                        tokio::time::sleep(delay_between_opens).await;
                    }

                    if let Err(e) = crate::platform::open_url(url) {
                        failures.push(format!("{url}: {e}"));
                    }
                }

                let message = if failures.is_empty() {
                    Message::UrlsOpenedInBrowser(UrlsOpenedResult::Success)
                } else {
                    Message::UrlsOpenedInBrowser(UrlsOpenedResult::Failure(std::io::Error::other(
                        failures.join("; "),
                    )))
                };

                let _ = event_tx.try_send(message);
            });
        }
        Command::SearchBookmarks(search_query) => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result = get_bookmarks_by_query(&pool, &search_query, DEFAULT_LIMIT).await;
                let message = Message::SearchFinished(result);
                let _ = event_tx.try_send(message);
            });
        }
        Command::FetchAllBookmarks => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result = get_all_bookmarks(&pool).await;
                let _ = event_tx.try_send(Message::AllBookmarksFetched(result));
            });
        }
        Command::FetchTags => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result = get_tags_with_stats(&pool).await;
                let message = Message::TagsFetched(result);
                let _ = event_tx.try_send(message);
            });
        }
        Command::FetchBookmarksForTag(tag) => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result = get_bookmarks(&pool, None, None, vec![tag], DEFAULT_LIMIT).await;
                let message = Message::BookmarksForTagFetched(result);
                let _ = event_tx.try_send(message);
            });
        }
        Command::FetchDuplicateBookmarks => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result = get_duplicate_bookmarks(&pool).await;
                let _ = event_tx.try_send(Message::DuplicateBookmarksFetched(result));
            });
        }
        Command::DeleteBookmark(uri) => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result = delete_bookmarks_with_uris(&pool, &vec![uri]).await;
                let _ = event_tx.try_send(Message::BookmarkDeleted(result));
            });
        }
        Command::FetchNote(uri) => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result = get_note(&pool, &uri).await;
                let _ = event_tx.try_send(Message::NoteFetched(uri, result));
            });
        }
        Command::SaveNote { uri, note } => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result = set_note(&pool, &uri, note)
                    .await
                    .map_err(|e| format!("{e}"));
                let _ = event_tx.try_send(Message::NoteSaved(result));
            });
        }
        Command::UpdateBookmark {
            uri,
            new_uri,
            title,
            tags,
        } => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result: Result<(), String> = async {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_err(|e| format!("system time error: {e}"))?
                        .as_secs() as i64;

                    let effective_uri = match &new_uri {
                        Some(target) if target != &uri => {
                            rename_bookmark_uri(&pool, &uri, target, now)
                                .await
                                .map_err(|e| format!("{e}"))?;
                            target.clone()
                        }
                        _ => uri,
                    };

                    let potential_bookmark =
                        PotentialBookmark::from((effective_uri, title, &tags));

                    let draft_bookmark = DraftBookmark::try_from(potential_bookmark)
                        .map_err(|e| format!("{e}"))?;

                    let options = SaveBookmarkOptions {
                        reset_missing_attributes: true,
                        reset_tags: true,
                    };

                    create_or_update_bookmark(&pool, &draft_bookmark, now, options)
                        .await
                        .map_err(|e| format!("{e}"))
                }
                .await;

                let _ = event_tx.try_send(Message::BookmarkUpdated(result));
            });
        }
        Command::CopyContentToClipboard(content) => {
            tokio::task::spawn_blocking(move || {
                let result = copy_content_to_clipboard(&content);
                let _ = event_tx.try_send(Message::ContentCopiedToClipboard(result));
            });
        }
    }
}

fn copy_content_to_clipboard(content: &str) -> Result<(), String> {
    crate::platform::copy_to_clipboard(content)
}
