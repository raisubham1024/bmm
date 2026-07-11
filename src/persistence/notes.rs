use super::DBError;
use sqlx::{Pool, Sqlite};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(thiserror::Error, Debug)]
pub enum NoteError {
    #[error("couldn't execute query: {0}")]
    CouldntExecuteQuery(#[source] DBError),
    #[error("bookmark with this uri doesn't exist; save it first with \"bmm save\"")]
    BookmarkDoesntExist,
    #[error("couldn't determine current time: {0}")]
    CouldntDetermineTime(String),
}

/// Returns the note attached to a bookmark, if any. Returns `Ok(None)` both
/// when the bookmark has no note, and (deliberately, to keep this a cheap
/// read-only lookup) when the uri isn't a saved bookmark at all.
pub async fn get_note(pool: &Pool<Sqlite>, uri: &str) -> Result<Option<String>, DBError> {
    let note: Option<(Option<String>,)> =
        sqlx::query_as("SELECT notes FROM bookmarks WHERE uri = ?")
            .bind(uri)
            .fetch_optional(pool)
            .await
            .map_err(|e| DBError::CouldntExecuteQuery("get note".into(), e))?;

    Ok(note.and_then(|(n,)| n))
}

/// Sets (or, if `note` is `None`/empty, clears) the note for a bookmark.
/// Fails with [`NoteError::BookmarkDoesntExist`] if there's no bookmark
/// saved with this exact uri.
pub async fn set_note(
    pool: &Pool<Sqlite>,
    uri: &str,
    note: Option<String>,
) -> Result<(), NoteError> {
    let note = note.and_then(|n| {
        let trimmed = n.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| NoteError::CouldntDetermineTime(e.to_string()))?
        .as_secs() as i64;

    let result = sqlx::query("UPDATE bookmarks SET notes = ?, updated_at = ? WHERE uri = ?")
        .bind(note)
        .bind(now)
        .bind(uri)
        .execute(pool)
        .await
        .map_err(|e| {
            NoteError::CouldntExecuteQuery(DBError::CouldntExecuteQuery("set note".into(), e))
        })?;

    if result.rows_affected() == 0 {
        return Err(NoteError::BookmarkDoesntExist);
    }

    Ok(())
}
