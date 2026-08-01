use super::DBError;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use sqlx::{Pool, Sqlite, SqlitePool, migrate::MigrateDatabase};
use std::str::FromStr;

pub async fn get_db_pool(uri: &str) -> Result<Pool<Sqlite>, DBError> {
    let db_exists = Sqlite::database_exists(uri)
        .await
        .map_err(DBError::CouldntCheckIfDbExists)?;

    if !db_exists {
        // sqlx creates new SQLite databases in WAL journal mode by
        // default (an internal, undocumented flag - see
        // https://github.com/launchbadge/sqlx/blob/main/sqlx-sqlite/src/lib.rs),
        // which keeps a permanent "<name>.db-wal" and "<name>.db-shm"
        // file sitting next to the database at all times, alongside the
        // ".db" file itself. Turning this off here is only half the
        // fix - it stops *new* databases from starting out in WAL mode,
        // but doesn't change any that already exist; the
        // `.journal_mode(...)` call below (which runs regardless of
        // whether the database is brand new or already existed) is what
        // actually enforces "DELETE" mode either way.
        sqlx::sqlite::CREATE_DB_WAL.store(false, std::sync::atomic::Ordering::Release);

        Sqlite::create_database(uri)
            .await
            .map_err(DBError::CouldntCreateDatabase)?;
    }

    let options = SqliteConnectOptions::from_str(uri)
        .map_err(DBError::CouldntConnectToDB)?
        // SQLite's traditional rollback-journal mode, rather than WAL:
        // WAL mode keeps persistent "-wal"/"-shm" files sitting next to
        // the database at all times, while DELETE mode only ever
        // creates a transient "-journal" file during an active write,
        // removing it immediately after - so besides the ".db" file
        // itself, there's nothing left behind the rest of the time.
        // Setting this explicitly (rather than just relying on whatever
        // mode a given database file already happens to be in) also
        // means bmm converts any database that ended up in WAL mode
        // under an older version of bmm back to DELETE mode - cleaning
        // up its "-wal"/"-shm" files - the next time it's opened.
        .journal_mode(SqliteJournalMode::Delete);

    let db = SqlitePool::connect_with(options)
        .await
        .map_err(DBError::CouldntConnectToDB)?;

    sqlx::migrate!()
        .run(&db)
        .await
        .map_err(DBError::CouldntMigrateDB)?;

    Ok(db)
}

#[cfg(test)]
pub(super) async fn get_in_memory_db_pool() -> Result<Pool<Sqlite>, DBError> {
    let db = SqlitePool::connect("sqlite://:memory:")
        .await
        .map_err(DBError::CouldntConnectToDB)?;

    sqlx::migrate!()
        .run(&db)
        .await
        .map_err(DBError::CouldntMigrateDB)?;

    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;

    #[tokio::test]
    async fn migrating_db_works() {
        // GIVEN
        // WHEN
        let result = get_in_memory_db_pool().await;

        // THEN
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_conn_fails_if_path_doesnt_exist() {
        // GIVEN
        let path = "nonexistent/nonexistent/nonexistent.db";

        // WHEN
        let error = get_db_pool(path)
            .await
            .expect_err("result should've been an error");

        // THEN
        assert_debug_snapshot!(error, @r#"
        CouldntCreateDatabase(
            Database(
                SqliteError {
                    code: 14,
                    message: "unable to open database file",
                },
            ),
        )
        "#);
    }
}
