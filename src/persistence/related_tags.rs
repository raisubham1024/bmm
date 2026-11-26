use super::DBError;
use sqlx::{Pool, Sqlite};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

/// A tag connected to another one - directly (they sit together on a
/// bookmark) or through other tags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedTag {
    pub name: String,
    /// 0 = the tag itself, 1 = used together with it on some bookmark,
    /// 2 = used together with one of those, and so on.
    pub hops: usize,
    /// How many bookmarks have this tag.
    pub num_bookmarks: i64,
}

/// For every bookmark in this database that has tags: the names of its tags.
pub async fn get_bookmark_tag_sets(pool: &Pool<Sqlite>) -> Result<Vec<Vec<String>>, DBError> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT bt.bookmark_id, t.name \
         FROM bookmark_tags bt \
         JOIN tags t ON t.id = bt.tag_id \
         ORDER BY bt.bookmark_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| DBError::CouldntExecuteQuery("get bookmark tag sets".into(), e))?;

    let mut by_bookmark: BTreeMap<i64, Vec<String>> = BTreeMap::new();
    for (bookmark_id, name) in rows {
        by_bookmark.entry(bookmark_id).or_default().push(name);
    }

    Ok(by_bookmark.into_values().collect())
}

/// Finds every tag connected to `origin`, given the tag sets of all the
/// bookmarks (from any number of databases - tags are matched by name).
///
/// Two tags are connected when some bookmark has both. The result follows
/// those connections as far as they go: the tag itself, then the tags used
/// together with it, then the tags used together with those, and so on.
/// Sorted nearest first, then by name. Empty if no bookmark has `origin`.
pub fn find_related_tags(origin: &str, bookmark_tag_sets: &[Vec<String>]) -> Vec<RelatedTag> {
    let mut num_bookmarks: HashMap<&str, i64> = HashMap::new();
    let mut neighbours: HashMap<&str, BTreeSet<&str>> = HashMap::new();

    for tags in bookmark_tag_sets {
        let unique: BTreeSet<&str> = tags.iter().map(String::as_str).collect();

        for tag in &unique {
            *num_bookmarks.entry(tag).or_insert(0) += 1;

            let entry = neighbours.entry(tag).or_default();
            entry.extend(unique.iter().filter(|other| *other != tag));
        }
    }

    if !num_bookmarks.contains_key(origin) {
        return vec![];
    }

    let mut hops: HashMap<&str, usize> = HashMap::new();
    let mut queue: VecDeque<&str> = VecDeque::new();

    let origin_key = num_bookmarks
        .get_key_value(origin)
        .map(|(k, _)| *k)
        .unwrap_or(origin);
    hops.insert(origin_key, 0);
    queue.push_back(origin_key);

    while let Some(tag) = queue.pop_front() {
        let next_hops = hops[tag] + 1;

        for other in neighbours.get(tag).into_iter().flatten() {
            if !hops.contains_key(other) {
                hops.insert(other, next_hops);
                queue.push_back(other);
            }
        }
    }

    let mut related: Vec<RelatedTag> = hops
        .into_iter()
        .map(|(name, hops)| RelatedTag {
            name: name.to_string(),
            hops,
            num_bookmarks: num_bookmarks[name],
        })
        .collect();
    related.sort_by(|a, b| a.hops.cmp(&b.hops).then_with(|| a.name.cmp(&b.name)));

    related
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DraftBookmark, PotentialBookmark};
    use crate::persistence::test_fixtures::DBPoolFixture;
    use crate::persistence::{SaveBookmarkOptions, create_or_update_bookmark};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sets(raw: &[&[&str]]) -> Vec<Vec<String>> {
        raw.iter()
            .map(|tags| tags.iter().map(|t| t.to_string()).collect())
            .collect()
    }

    fn summary(related: &[RelatedTag]) -> Vec<(&str, usize, i64)> {
        related
            .iter()
            .map(|r| (r.name.as_str(), r.hops, r.num_bookmarks))
            .collect()
    }

    #[test]
    fn follows_connections_through_other_tags() {
        // cyber - hacking - ctf - writeups, and an unrelated cooking - pasta
        let sets = sets(&[
            &["cyber", "hacking"],
            &["hacking", "ctf"],
            &["ctf", "writeups"],
            &["cooking", "pasta"],
        ]);

        let related = find_related_tags("cyber", &sets);

        assert_eq!(
            summary(&related),
            [
                ("cyber", 0, 1),
                ("hacking", 1, 2),
                ("ctf", 2, 2),
                ("writeups", 3, 1),
            ]
        );
    }

    #[test]
    fn counts_bookmarks_and_sorts_nearest_first_then_by_name() {
        let sets = sets(&[
            &["cyber", "zeta", "alpha"],
            &["cyber", "alpha"],
            &["alpha", "deep"],
        ]);

        let related = find_related_tags("cyber", &sets);

        assert_eq!(
            summary(&related),
            [
                ("cyber", 0, 2),
                ("alpha", 1, 3),
                ("zeta", 1, 1),
                ("deep", 2, 1),
            ]
        );
    }

    #[test]
    fn a_tag_that_is_never_used_with_others_only_lists_itself() {
        let sets = sets(&[&["solo"], &["a", "b"]]);

        assert_eq!(summary(&find_related_tags("solo", &sets)), [("solo", 0, 1)]);
    }

    #[test]
    fn an_unknown_tag_has_nothing_related() {
        let sets = sets(&[&["a", "b"]]);

        assert!(find_related_tags("nope", &sets).is_empty());
    }

    #[test]
    fn duplicate_names_on_one_bookmark_count_once() {
        let sets = sets(&[&["a", "a", "b"]]);

        assert_eq!(
            summary(&find_related_tags("a", &sets)),
            [("a", 0, 1), ("b", 1, 1)]
        );
    }

    #[tokio::test]
    async fn reads_the_tag_sets_of_every_tagged_bookmark() {
        // GIVEN
        let fixture = DBPoolFixture::new().await;
        let pool = &fixture.pool;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        for (uri, tags) in [
            ("https://example.com/1", vec!["cyber", "hacking"]),
            ("https://example.com/2", vec!["hacking", "ctf"]),
            ("https://example.com/3", vec![]),
        ] {
            let draft = DraftBookmark::try_from(PotentialBookmark::from((uri, None, &tags)))
                .expect("draft bookmark should be initialized");
            create_or_update_bookmark(pool, &draft, now, SaveBookmarkOptions::default())
                .await
                .expect("bookmark should be saved");
        }

        // WHEN
        let mut sets = get_bookmark_tag_sets(pool).await.unwrap();
        for tags in &mut sets {
            tags.sort();
        }
        sets.sort();

        // THEN (the bookmark without tags isn't there)
        assert_eq!(
            sets,
            vec![
                vec!["cyber".to_string(), "hacking".to_string()],
                vec!["ctf".to_string(), "hacking".to_string()],
            ]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
        );
    }
}
