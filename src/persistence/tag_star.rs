use super::DBError;
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;

/// Returns the names of the tags that are starred in this database. Used by
/// the TUI to put a star marker next to them in the Tags List view, and to
/// show only starred tags.
pub async fn get_starred_tag_names(pool: &Pool<Sqlite>) -> Result<HashSet<String>, DBError> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT name FROM tags WHERE starred = 1")
        .fetch_all(pool)
        .await
        .map_err(|e| DBError::CouldntExecuteQuery("get starred tag names".into(), e))?;

    Ok(rows.into_iter().map(|(name,)| name).collect())
}

/// Sets whether a tag is starred, and returns how many tags were changed
/// (0 if this database doesn't have a tag with that name).
pub async fn set_tag_starred(
    pool: &Pool<Sqlite>,
    name: &str,
    starred: bool,
) -> Result<u64, DBError> {
    let result = sqlx::query("UPDATE tags SET starred = ? WHERE name = ?")
        .bind(starred)
        .bind(name)
        .execute(pool)
        .await
        .map_err(|e| DBError::CouldntExecuteQuery("set tag starred".into(), e))?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DraftBookmark, PotentialBookmark};
    use crate::persistence::test_fixtures::DBPoolFixture;
    use crate::persistence::{SaveBookmarkOptions, create_or_update_bookmark};
    use std::time::{SystemTime, UNIX_EPOCH};

    async fn save_with_tags(pool: &Pool<Sqlite>, uri: &str, tags: &[&str]) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let draft_bookmark =
            DraftBookmark::try_from(PotentialBookmark::from((uri, None, &tags.to_vec())))
                .expect("draft bookmark should be initialized");

        create_or_update_bookmark(pool, &draft_bookmark, now, SaveBookmarkOptions::default())
            .await
            .expect("bookmark should be saved in db");
    }

    #[tokio::test]
    async fn starring_and_unstarring_a_tag_works() {
        // GIVEN
        let fixture = DBPoolFixture::new().await;
        let pool = &fixture.pool;
        save_with_tags(pool, "https://example.com", &["rust", "learn"]).await;

        // WHEN / THEN
        assert!(get_starred_tag_names(pool).await.unwrap().is_empty());

        assert_eq!(set_tag_starred(pool, "rust", true).await.unwrap(), 1);
        let starred = get_starred_tag_names(pool).await.unwrap();
        assert_eq!(starred, HashSet::from(["rust".to_string()]));

        assert_eq!(set_tag_starred(pool, "rust", false).await.unwrap(), 1);
        assert!(get_starred_tag_names(pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn starring_a_tag_that_doesnt_exist_changes_nothing() {
        // GIVEN
        let fixture = DBPoolFixture::new().await;

        // WHEN
        let changed = set_tag_starred(&fixture.pool, "nope", true).await.unwrap();

        // THEN
        assert_eq!(changed, 0);
    }

    #[tokio::test]
    async fn merging_a_starred_tag_into_another_keeps_the_star() {
        // GIVEN
        let fixture = DBPoolFixture::new().await;
        let pool = &fixture.pool;
        save_with_tags(pool, "https://example.com/1", &["old"]).await;
        save_with_tags(pool, "https://example.com/2", &["new"]).await;
        set_tag_starred(pool, "old", true).await.unwrap();

        // WHEN
        let target = crate::domain::Tag::try_from("new").unwrap();
        crate::persistence::rename_tag_name(pool, "old".to_string(), target)
            .await
            .unwrap();

        // THEN
        let starred = get_starred_tag_names(pool).await.unwrap();
        assert_eq!(starred, HashSet::from(["new".to_string()]));
    }

    #[tokio::test]
    async fn plainly_renaming_a_starred_tag_keeps_the_star() {
        // GIVEN
        let fixture = DBPoolFixture::new().await;
        let pool = &fixture.pool;
        save_with_tags(pool, "https://example.com/1", &["old"]).await;
        set_tag_starred(pool, "old", true).await.unwrap();

        // WHEN
        let target = crate::domain::Tag::try_from("fresh").unwrap();
        crate::persistence::rename_tag_name(pool, "old".to_string(), target)
            .await
            .unwrap();

        // THEN
        let starred = get_starred_tag_names(pool).await.unwrap();
        assert_eq!(starred, HashSet::from(["fresh".to_string()]));
    }
}
