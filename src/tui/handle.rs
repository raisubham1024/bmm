use super::commands::Command;
use super::message::{Message, UrlsOpenedResult};
use crate::common::DEFAULT_LIMIT;
use crate::domain::{DraftBookmark, PotentialBookmark, Tag};
use crate::persistence::{
    DBError, SaveBookmarkOptions, create_or_update_bookmark, delete_bookmarks_with_uris,
    get_all_bookmarks, get_bookmark_with_exact_uri, get_bookmarks, get_bookmarks_by_query,
    get_bookmarks_with_notes, get_db_pool, get_note, get_starred_bookmarks, get_starred_uris,
    get_tags_with_stats, rename_bookmark_uri, rename_tag_name, set_note, set_starred,
    toggle_starred,
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
            tokio::spawn(open_urls_and_report(
                urls,
                crate::platform::open_url_new_tab,
                event_tx,
            ));
        }
        Command::OpenInBrowserIncognito(url) => {
            tokio::spawn(async move {
                let message = match crate::platform::open_url_incognito(&url) {
                    Ok(_) => Message::UrlsOpenedInBrowser(UrlsOpenedResult::Success),
                    Err(e) => Message::UrlsOpenedInBrowser(UrlsOpenedResult::Failure(
                        std::io::Error::other(e),
                    )),
                };

                let _ = event_tx.try_send(message);
            });
        }
        Command::OpenMultipleInBrowserIncognito(urls) => {
            tokio::spawn(open_urls_and_report(
                urls,
                crate::platform::open_url_incognito,
                event_tx,
            ));
        }
        Command::SearchBookmarks(search_query, scope) => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result =
                    get_bookmarks_by_query(&pool, &search_query, DEFAULT_LIMIT, scope).await;
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
        Command::FetchGlobalTags => {
            tokio::spawn(async move {
                let mut totals: std::collections::BTreeMap<String, i64> =
                    std::collections::BTreeMap::new();
                let mut errors: Vec<String> = Vec::new();

                for (name, path) in local_db_files() {
                    match get_db_pool(&path).await {
                        Ok(db_pool) => match get_tags_with_stats(&db_pool).await {
                            Ok(stats) => {
                                for stat in stats {
                                    *totals.entry(stat.name).or_insert(0) += stat.num_bookmarks;
                                }
                            }
                            Err(e) => errors.push(format!("{name}: {e}")),
                        },
                        Err(e) => errors.push(format!("{name}: {e}")),
                    }
                }

                let tags: Vec<crate::domain::TagStats> = totals
                    .into_iter()
                    .map(|(name, num_bookmarks)| crate::domain::TagStats {
                        name,
                        num_bookmarks,
                    })
                    .collect();

                let _ = event_tx.try_send(Message::GlobalTagsFetched(tags, errors));
            });
        }
        Command::FetchBookmarksForTagAcrossDatabases(tag) => {
            tokio::spawn(async move {
                let mut all_results: Vec<(String, String, crate::domain::SavedBookmark)> =
                    Vec::new();
                let mut errors: Vec<String> = Vec::new();

                for (name, path) in local_db_files() {
                    match get_db_pool(&path).await {
                        Ok(db_pool) => {
                            let result =
                                get_bookmarks(&db_pool, None, None, vec![tag.clone()], DEFAULT_LIMIT)
                                    .await;
                            match result {
                                Ok(bookmarks) => {
                                    for b in bookmarks {
                                        all_results.push((name.clone(), path.clone(), b));
                                    }
                                }
                                Err(e) => errors.push(format!("{name}: {e}")),
                            }
                        }
                        Err(e) => errors.push(format!("{name}: {e}")),
                    }
                }

                let _ = event_tx.try_send(Message::BookmarksForTagAcrossDatabasesFetched(
                    all_results,
                    errors,
                ));
            });
        }
        Command::FetchStarredBookmarks => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result = get_starred_bookmarks(&pool).await;
                let _ = event_tx.try_send(Message::StarredBookmarksFetched(result));
            });
        }
        Command::FetchStarredUris => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result = get_starred_uris(&pool).await;
                let _ = event_tx.try_send(Message::StarredUrisFetched(result));
            });
        }
        Command::ToggleStar(uri) => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result = toggle_starred(&pool, &uri)
                    .await
                    .map_err(|e| format!("{e}"));
                let _ = event_tx.try_send(Message::StarToggled(uri, result));
            });
        }
        Command::SwitchDatabase { path, display_name } => {
            tokio::spawn(async move {
                let result = get_db_pool(&path)
                    .await
                    .map(|new_pool| (new_pool, display_name))
                    .map_err(|e| format!("{e}"));
                let _ = event_tx.try_send(Message::DatabaseSwitched(result));
            });
        }
        Command::DeleteBookmark(uri, target_db_path) => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let uri_for_message = uri.clone();
                let result: Result<u64, DBError> = async {
                    let target_pool = match &target_db_path {
                        Some(path) => get_db_pool(path).await?,
                        None => pool,
                    };
                    delete_bookmarks_with_uris(&target_pool, &vec![uri]).await
                }
                .await;

                let _ = event_tx.try_send(Message::BookmarkDeleted(uri_for_message, result));
            });
        }
        Command::DeleteBookmarks(items) => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result = delete_bookmarks(&pool, items).await;
                let _ = event_tx.try_send(Message::BookmarksDeleted(result));
            });
        }
        Command::RenameTag {
            old_name,
            new_name,
            global,
        } => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result = rename_tag(&pool, &old_name, &new_name, global).await;
                let _ = event_tx.try_send(Message::TagRenamed(old_name, new_name, result));
            });
        }
        Command::FetchNote(uri) => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result = get_note(&pool, &uri).await;
                let _ = event_tx.try_send(Message::NoteFetched(uri, result));
            });
        }
        Command::FetchNoteExists(uri) => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result = get_note(&pool, &uri).await.map(|note| note.is_some());
                let _ = event_tx.try_send(Message::NoteExistenceFetched(uri, result));
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
            is_new,
            target_db_path,
        } => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result: Result<(), String> = async {
                    let target_pool = match &target_db_path {
                        Some(path) => get_db_pool(path).await.map_err(|e| format!("{e}"))?,
                        None => pool,
                    };

                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_err(|e| format!("system time error: {e}"))?
                        .as_secs() as i64;

                    let effective_uri = if is_new {
                        uri
                    } else {
                        match &new_uri {
                            Some(target) if target != &uri => {
                                rename_bookmark_uri(&target_pool, &uri, target, now)
                                    .await
                                    .map_err(|e| format!("{e}"))?;
                                target.clone()
                            }
                            _ => uri,
                        }
                    };

                    let potential_bookmark =
                        PotentialBookmark::from((effective_uri, title, &tags));

                    let draft_bookmark = DraftBookmark::try_from(potential_bookmark)
                        .map_err(|e| format!("{e}"))?;

                    let options = SaveBookmarkOptions {
                        reset_missing_attributes: true,
                        reset_tags: true,
                    };

                    create_or_update_bookmark(&target_pool, &draft_bookmark, now, options)
                        .await
                        .map_err(|e| format!("{e}"))
                }
                .await;

                let _ = event_tx.try_send(Message::BookmarkUpdated(result));
            });
        }
        Command::GlobalSearch(search_terms, scope) => {
            tokio::spawn(async move {
                let mut all_results: Vec<(String, String, crate::domain::SavedBookmark)> =
                    Vec::new();
                let mut errors: Vec<String> = Vec::new();

                for (name, path) in local_db_files() {
                    match get_db_pool(&path).await {
                        Ok(db_pool) => {
                            let result = match &search_terms {
                                Some(terms) => {
                                    get_bookmarks_by_query(&db_pool, terms, DEFAULT_LIMIT, scope)
                                        .await
                                }
                                None => get_all_bookmarks(&db_pool).await,
                            };

                            match result {
                                Ok(bookmarks) => {
                                    for b in bookmarks {
                                        all_results.push((name.clone(), path.clone(), b));
                                    }
                                }
                                Err(e) => errors.push(format!("{name}: {e}")),
                            }
                        }
                        Err(e) => errors.push(format!("{name}: {e}")),
                    }
                }

                let _ = event_tx.try_send(Message::GlobalSearchFinished(all_results, errors));
            });
        }
        Command::SearchNotes(search_terms) => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result =
                    get_bookmarks_with_notes(&pool, search_terms.as_ref(), DEFAULT_LIMIT).await;
                let message = Message::SearchFinished(result);
                let _ = event_tx.try_send(message);
            });
        }
        Command::MoveBookmarks {
            items,
            target_db_path,
            target_display_name,
        } => {
            let pool = pool.clone();
            tokio::spawn(async move {
                let result = move_bookmarks(&pool, items, &target_db_path)
                    .await
                    .map(|count| (count, target_display_name));
                let _ = event_tx.try_send(Message::BookmarksMoved(result));
            });
        }
        Command::CopyContentToClipboard(content) => {
            tokio::task::spawn_blocking(move || {
                let result = copy_content_to_clipboard(&content);
                let _ = event_tx.try_send(Message::ContentCopiedToClipboard(result));
            });
        }
        Command::BackupDatabases => {
            tokio::task::spawn_blocking(move || {
                let result = super::backup::backup_databases();
                let _ = event_tx.try_send(Message::DatabasesBackedUp(result));
            });
        }
        Command::RestoreDatabases => {
            tokio::task::spawn_blocking(move || {
                let result = super::backup::restore_databases();
                let _ = event_tx.try_send(Message::DatabasesRestored(result));
            });
        }
        Command::CheckForUpdate => {
            tokio::spawn(async move {
                let result = crate::self_update::update_bmm()
                    .await
                    .map_err(|e| e.to_string());
                let _ = event_tx.try_send(Message::UpdateCheckFinished(result));
            });
        }
        Command::FetchDescription(uri) => {
            tokio::spawn(async move {
                let uri = crate::domain::normalize_uri_scheme(uri);
                let result = crate::cli::fetch_page_metadata(
                    &uri,
                    crate::cli::DEFAULT_FETCH_TIMEOUT_SECS,
                )
                .await
                .map(|metadata| metadata.description)
                .map_err(|e| e.to_string());

                let _ = event_tx.try_send(Message::DescriptionFetched(result));
            });
        }
    }
}

/// Renames a tag - updating every bookmark that used it - either in just
/// `pool`'s database, or (`global: true`) across every local database.
/// Merges into `new_name` if it already exists as a tag (`rename_tag_name`
/// takes care of that), rather than ending up with a duplicate tag.
///
/// Returns the total number of bookmarks whose tag was actually renamed.
/// As with `delete_bookmarks`, a global rename that partially fails still
/// keeps whatever succeeded and reports both the successes and the
/// failures rather than losing the successful part.
async fn rename_tag(
    pool: &Pool<Sqlite>,
    old_name: &str,
    new_name: &str,
    global: bool,
) -> Result<u64, String> {
    // Validated once up front so an invalid new name fails fast, before
    // touching any database - `Tag` isn't `Clone`, so a fresh one still
    // has to be built from `new_name` per-database below.
    if Tag::try_from(new_name).is_err() {
        return Err(format!(
            "\"{new_name}\" isn't a valid tag name (letters, numbers, \"_\"/\"-\", and \"/\" for nesting, 1-30 chars per segment)"
        ));
    }

    if !global {
        let Ok(new_tag) = Tag::try_from(new_name) else {
            unreachable!("validated above");
        };
        return rename_tag_name(pool, old_name.to_string(), new_tag)
            .await
            .map_err(|e| format!("{e}"));
    }

    let mut total = 0u64;
    let mut errors: Vec<String> = Vec::new();

    for (name, path) in local_db_files() {
        let Ok(new_tag) = Tag::try_from(new_name) else {
            unreachable!("validated above");
        };

        match get_db_pool(&path).await {
            Ok(db_pool) => match rename_tag_name(&db_pool, old_name.to_string(), new_tag).await {
                Ok(n) => total += n,
                Err(e) => errors.push(format!("{name}: {e}")),
            },
            Err(e) => errors.push(format!("{name}: {e}")),
        }
    }

    if errors.is_empty() {
        Ok(total)
    } else if total == 0 {
        Err(errors.join("; "))
    } else {
        Err(format!(
            "renamed for {total} bookmark(s), but failed for: {}",
            errors.join("; ")
        ))
    }
}

fn copy_content_to_clipboard(content: &str) -> Result<(), String> {
    crate::platform::copy_to_clipboard(content)
}

/// Lists every local bmm database file (name, full path) found in bmm's
/// data directory - the same set of databases `Command::GlobalSearch`
/// searches across. Shared by every "across all databases" command
/// (`GlobalSearch`, `FetchGlobalTags`, `FetchBookmarksForTagAcrossDatabases`)
/// so they all agree on exactly which databases count as "all of them".
/// Returns an empty list (rather than erroring) if the data directory can't
/// be read - callers just end up with no results, same as before this was
/// factored out.
fn local_db_files() -> Vec<(String, String)> {
    let mut db_files: Vec<(String, String)> = Vec::new();

    if let Ok(data_dir) = crate::utils::get_data_dir() {
        let bmm_dir = data_dir.join("bmm");

        if let Ok(entries) = std::fs::read_dir(&bmm_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("db") {
                    continue;
                }
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                let Some(path_str) = path.to_str() else {
                    continue;
                };
                db_files.push((name.to_string(), path_str.to_string()));
            }
        }
    }

    db_files
}

/// Deletes every bookmark in `items` - (uri, source database path, `None`
/// meaning the currently active database) - grouping by source database so
/// each database is only opened once. Used for "delete every currently
/// listed bookmark" (`D`), whose items may span several databases when
/// they came from a global search/tag result.
///
/// Returns the total number of bookmarks actually deleted. If any
/// bookmark(s) fail to delete, the rest are still attempted; partial/total
/// failure is reported as `Err` with a combined message (bookmarks that did
/// delete successfully stay deleted either way).
async fn delete_bookmarks(
    pool: &Pool<Sqlite>,
    items: Vec<(String, Option<String>)>,
) -> Result<u64, String> {
    use std::collections::HashMap;

    let mut by_db: HashMap<Option<String>, Vec<String>> = HashMap::new();
    for (uri, source_db_path) in items {
        by_db.entry(source_db_path).or_default().push(uri);
    }

    let mut deleted = 0u64;
    let mut errors: Vec<String> = Vec::new();

    for (source_db_path, uris) in by_db {
        let result: Result<u64, String> = async {
            let source_pool = match &source_db_path {
                Some(path) => get_db_pool(path).await.map_err(|e| format!("{e}"))?,
                None => pool.clone(),
            };

            delete_bookmarks_with_uris(&source_pool, &uris)
                .await
                .map_err(|e| format!("{e}"))
        }
        .await;

        match result {
            Ok(n) => deleted += n,
            Err(e) => errors.push(format!(
                "{}: {e}",
                source_db_path.as_deref().unwrap_or("active database")
            )),
        }
    }

    if errors.is_empty() {
        Ok(deleted)
    } else if deleted == 0 {
        Err(errors.join("; "))
    } else {
        Err(format!(
            "deleted {deleted}, but failed for: {}",
            errors.join("; ")
        ))
    }
}

/// Moves each bookmark (with its title, tags, note, and starred status) out
/// of its source database and into `target_db_path`. `items` pairs each
/// bookmark's uri with the path of the database it currently lives in -
/// `None` means "the currently active database" (`pool`).
///
/// Returns the number of bookmarks successfully moved. If any bookmark
/// fails to move, the rest are still attempted; partial/total failure is
/// reported as `Err` with a combined message (bookmarks that did move
/// successfully stay moved either way - this only affects what's reported
/// back to the user).
async fn move_bookmarks(
    pool: &Pool<Sqlite>,
    items: Vec<(String, Option<String>)>,
    target_db_path: &str,
) -> Result<usize, String> {
    let target_pool = get_db_pool(target_db_path)
        .await
        .map_err(|e| format!("couldn't open destination database: {e}"))?;

    let mut moved = 0usize;
    let mut errors: Vec<String> = Vec::new();

    for (uri, source_db_path) in items {
        let result: Result<(), String> = async {
            let source_pool = match &source_db_path {
                Some(path) => get_db_pool(path).await.map_err(|e| format!("{e}"))?,
                None => pool.clone(),
            };

            let bookmark = get_bookmark_with_exact_uri(&source_pool, &uri)
                .await
                .map_err(|e| format!("{e}"))?
                .ok_or_else(|| "bookmark no longer exists".to_string())?;

            let note = get_note(&source_pool, &uri).await.map_err(|e| format!("{e}"))?;

            let starred_uris = get_starred_uris(&source_pool)
                .await
                .map_err(|e| format!("{e}"))?;
            let is_starred = starred_uris.contains(&uri);

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| format!("system time error: {e}"))?
                .as_secs() as i64;

            let tags: Vec<String> = bookmark
                .tags
                .as_deref()
                .unwrap_or("")
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();

            let potential_bookmark =
                PotentialBookmark::from((bookmark.uri.clone(), bookmark.title.clone(), &tags));
            let draft_bookmark =
                DraftBookmark::try_from(potential_bookmark).map_err(|e| format!("{e}"))?;

            let options = SaveBookmarkOptions {
                reset_missing_attributes: true,
                reset_tags: true,
            };

            create_or_update_bookmark(&target_pool, &draft_bookmark, now, options)
                .await
                .map_err(|e| format!("{e}"))?;

            if is_starred {
                set_starred(&target_pool, &uri, true)
                    .await
                    .map_err(|e| format!("{e}"))?;
            }

            if note.is_some() {
                set_note(&target_pool, &uri, note)
                    .await
                    .map_err(|e| format!("{e}"))?;
            }

            delete_bookmarks_with_uris(&source_pool, &vec![uri.clone()])
                .await
                .map_err(|e| format!("{e}"))?;

            Ok(())
        }
        .await;

        match result {
            Ok(()) => moved += 1,
            Err(e) => errors.push(format!("{uri}: {e}")),
        }
    }

    if errors.is_empty() {
        Ok(moved)
    } else if moved == 0 {
        Err(errors.join("; "))
    } else {
        Err(format!(
            "moved {moved}, but failed for: {}",
            errors.join("; ")
        ))
    }
}

/// Opens each of `urls` one at a time via `open_one`, reporting the
/// combined result as a single `Message::UrlsOpenedInBrowser` - shared by
/// `Command::OpenMultipleInBrowser` and `Command::OpenMultipleInBrowserIncognito`,
/// which differ only in which platform function they open each url with.
async fn open_urls_and_report(
    urls: Vec<String>,
    open_one: fn(&str) -> Result<(), String>,
    event_tx: Sender<Message>,
) {
    let mut failures: Vec<String> = Vec::new();

    // On Android, firing intents back-to-back with no gap can cause the
    // browser/OS hand-off to drop all but the first one - the hand-off
    // needs a brief moment to actually complete before the next is
    // fired. Desktop browsers don't have this problem, so that path is
    // left untouched (zero delay).
    let delay_between_opens = if cfg!(target_os = "android") {
        std::time::Duration::from_millis(400)
    } else {
        std::time::Duration::ZERO
    };

    for (i, url) in urls.iter().enumerate() {
        if i > 0 && !delay_between_opens.is_zero() {
            tokio::time::sleep(delay_between_opens).await;
        }

        if let Err(e) = open_one(url) {
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
}
