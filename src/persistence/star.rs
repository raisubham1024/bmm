use super::DBError;
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(thiserror::Error, Debug)]
pub enum StarError {
    #[error("couldn't execute query: {0}")]
    CouldntExecuteQuery(#[source] DBError),
    #[error("couldn't determine current time: {0}")]
    CouldntDetermineTime(String),
}

/// Returns the set of uris that are currently starred. Used by the TUI to
/// decide which bookmarks to show a star marker next to.
pub async fn get_starred_uris(pool: &Pool<Sqlite>) -> Result<HashSet<String>, DBError> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT uri FROM bookmarks WHERE starred = 1")
        .fetch_all(pool)
        .await
        .map_err(|e| DBError::CouldntExecuteQuery("get starred uris".into(), e))?;

    Ok(rows.into_iter().map(|(uri,)| uri).collect())
}

/// Flips a bookmark's starred status, and returns the new value (true if
/// it's now starred, false if it's now unstarred).
pub async fn toggle_starred(pool: &Pool<Sqlite>, uri: &str) -> Result<bool, StarError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| StarError::CouldntDetermineTime(e.to_string()))?
        .as_secs() as i64;

    sqlx::query(
        "UPDATE bookmarks SET starred = CASE WHEN starred = 0 THEN 1 ELSE 0 END, updated_at = ? WHERE uri = ?",
    )
    .bind(now)
    .bind(uri)
    .execute(pool)
    .await
    .map_err(|e| StarError::CouldntExecuteQuery(DBError::CouldntExecuteQuery("toggle starred".into(), e)))?;

    let row: Option<(i64,)> = sqlx::query_as("SELECT starred FROM bookmarks WHERE uri = ?")
        .bind(uri)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            StarError::CouldntExecuteQuery(DBError::CouldntExecuteQuery(
                "read starred status".into(),
                e,
            ))
        })?;

    Ok(row.map(|(v,)| v != 0).unwrap_or(false))
}

/// Sets a bookmark's starred status explicitly (used by `bmm save --star`).
pub async fn set_starred(pool: &Pool<Sqlite>, uri: &str, starred: bool) -> Result<(), StarError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| StarError::CouldntDetermineTime(e.to_string()))?
        .as_secs() as i64;

    sqlx::query("UPDATE bookmarks SET starred = ?, updated_at = ? WHERE uri = ?")
        .bind(starred)
        .bind(now)
        .bind(uri)
        .execute(pool)
        .await
        .map_err(|e| {
            StarError::CouldntExecuteQuery(DBError::CouldntExecuteQuery("set starred".into(), e))
        })?;

    Ok(())
}

/// Fetches bookmarks that have been starred, along with their tags, in the
/// same shape [`super::get::get_bookmarks`] returns them in.
pub async fn get_starred_bookmarks(
    pool: &Pool<Sqlite>,
) -> Result<Vec<crate::domain::SavedBookmark>, DBError> {
    let bookmarks = sqlx::query_as::<_, crate::domain::SavedBookmark>(
        r#"
SELECT
    uri,
    title,
    (
        SELECT
            GROUP_CONCAT(t.name, ',' ORDER BY t.name ASC)
        FROM
            tags t
            JOIN bookmark_tags bt ON t.id = bt.tag_id
        WHERE
            bt.bookmark_id = b.id
    ) AS tags
FROM
    bookmarks b
WHERE
    starred = 1
ORDER BY
    updated_at DESC
"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| DBError::CouldntExecuteQuery("fetch starred bookmarks".into(), e))?;

    Ok(bookmarks)
}
