use super::commands::Command;
use super::message::{Message, UrlsOpenedResult};
use crate::common::DEFAULT_LIMIT;
use crate::domain::{DraftBookmark, PotentialBookmark};
use crate::persistence::{
    SaveBookmarkOptions, create_or_update_bookmark, delete_bookmarks_with_uris, get_all_bookmarks,
    get_bookmarks, get_bookmarks_by_query, get_duplicate_bookmarks, get_tags_with_stats,
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

                for url in &urls {
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
        Command::UpdateBookmark { uri, title, tags } => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let potential_bookmark = PotentialBookmark::from((uri, title, &tags));

                let result: Result<(), String> = async {
                    let draft_bookmark = DraftBookmark::try_from(potential_bookmark)
                        .map_err(|e| format!("{e}"))?;

                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_err(|e| format!("system time error: {e}"))?
                        .as_secs() as i64;

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
