use crate::args::{Args, BmmCommand, TagsCommand};
use crate::cli::*;
use crate::domain::PotentialBookmark;
use crate::errors::AppError;
use crate::persistence::{get_db_pool, set_starred};
use crate::tui::{TuiContext, run_tui};
use crate::utils::get_data_dir;
use std::fs;
use std::path::PathBuf;

const DATA_DIR: &str = "bmm";
const DATA_FILE: &str = "bmm.db";

/// If a parent tag (`--ptag`) is provided, prefix every given tag with it
/// using "/" as the separator, e.g. ptag "linux/linuxmint" + tag "blog"
/// becomes "linux/linuxmint/blog". Tags are validated later downstream as
/// usual, so an invalid combination will still surface a normal error.
fn apply_parent_tag(tags: Vec<String>, ptag: Option<&str>) -> Vec<String> {
    match ptag {
        None => tags,
        Some(p) => {
            let p = p.trim().trim_matches('/');
            if p.is_empty() {
                return tags;
            }
            tags.into_iter()
                .map(|t| format!("{}/{}", p, t.trim()))
                .collect()
        }
    }
}

/// Names of the databases in bmm's data directory, sorted.
fn local_database_names(bmm_dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(bmm_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("db"))
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
        .collect();
    names.sort();
    names
}

/// Other databases (besides `current_db_name`) that have a tag with this
/// name - used to explain a "no such tag" error: the tag is probably in a
/// database other than the one the command ran on.
async fn databases_with_tag(tag: &str, current_db_name: &str) -> Vec<String> {
    let mut found = Vec::new();

    let Ok(data_dir) = get_data_dir() else {
        return found;
    };

    for name in local_database_names(&data_dir.join(DATA_DIR)) {
        if name == current_db_name {
            continue;
        }
        let Some(path) = data_dir.join(DATA_DIR).join(&name).to_str().map(String::from) else {
            continue;
        };
        if let Ok(pool) = get_db_pool(&path).await {
            if let Ok(tags) = crate::persistence::get_tags_with_stats(&pool).await
                && tags.iter().any(|t| t.name == tag)
            {
                found.push(name);
            }
            pool.close().await;
        }
    }

    found
}

fn print_tag_hint(tag: &str, current_db_name: &str, others: &[String]) {
    if others.is_empty() {
        return;
    }
    eprintln!(
        "hint: tag \"{tag}\" isn't in {current_db_name}, but it is in: {}\n      run the command again with --db <NAME> (eg. --db {})",
        others.join(", "),
        others[0].trim_end_matches(".db"),
    );
}

pub async fn handle(args: Args) -> Result<(), AppError> {
    let db_path = match (&args.db_path, &args.db) {
        (Some(p), _) => PathBuf::from(p),
        (None, Some(name)) => {
            let user_data_dir = get_data_dir().map_err(AppError::CouldntGetDataDirectory)?;
            let data_dir = user_data_dir.join(PathBuf::from(DATA_DIR));

            let wanted = name.trim();
            let file_name = if wanted.to_lowercase().ends_with(".db") {
                wanted.to_string()
            } else {
                format!("{wanted}.db")
            };

            let available = local_database_names(&data_dir);
            if !available.contains(&file_name) {
                return Err(AppError::DatabaseNotFound(
                    wanted.to_string(),
                    if available.is_empty() {
                        "none yet".to_string()
                    } else {
                        available.join(", ")
                    },
                ));
            }

            data_dir.join(file_name)
        }
        (None, None) => {
            let user_data_dir = get_data_dir().map_err(AppError::CouldntGetDataDirectory)?;
            let data_dir = user_data_dir.join(PathBuf::from(DATA_DIR));

            if !data_dir.exists() {
                fs::create_dir_all(&data_dir).map_err(AppError::CouldntCreateDataDirectory)?;
            }

            data_dir.join(PathBuf::from(DATA_FILE))
        }
    };

    let db_path = db_path.to_str().ok_or(AppError::DBPathNotValidStr)?;

    let db_name = std::path::Path::new(db_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(DATA_FILE)
        .to_string();

    if args.debug {
        display_debug_info(&args, db_path);
        return Ok(());
    }

    let pool = get_db_pool(db_path).await?;

    match args.command {
        None => run_tui(&pool, TuiContext::Initial, db_name.clone()).await?,
        Some(cmd) => match cmd {
        BmmCommand::Delete {
            uris,
            skip_confirmation,
        } => {
            delete_bookmarks(&pool, uris, skip_confirmation).await?;
        }

        BmmCommand::Import {
            file,
            reset_missing,
            dry_run,
            ignore_attribute_errors,
        } => {
            let result = import_bookmarks(
                &pool,
                &file,
                reset_missing,
                dry_run,
                ignore_attribute_errors,
            )
            .await?;
            if let Some(stats) = result {
                println!("imported {} bookmarks", stats.num_bookmarks_imported);
            }
        }

        BmmCommand::List {
            uri,
            title,
            tags,
            format,
            limit,
        } => list_bookmarks(&pool, uri, title, tags, format, limit).await?,

        BmmCommand::Search {
            query_terms,
            format,
            limit,
            tui,
        } => search_bookmarks(&pool, &query_terms, format, limit, tui, db_name.clone()).await?,

        BmmCommand::Save {
            uri,
            title,
            tags,
            ptag,
            use_editor,
            fail_if_uri_already_saved,
            reset_missing,
            ignore_attribute_errors,
            star,
        } => {
            let tags = apply_parent_tag(tags, ptag.as_deref());
            let uri = crate::domain::normalize_uri_scheme(uri);
            let potential_bookmark = PotentialBookmark::from((uri.clone(), title, &tags));

            save_bookmark(
                &pool,
                potential_bookmark,
                use_editor,
                fail_if_uri_already_saved,
                reset_missing,
                ignore_attribute_errors,
            )
            .await?;

            if star {
                set_starred(&pool, &uri, true).await?;
            }
        }

        BmmCommand::SaveAll {
            uris,
            tags,
            ptag,
            use_stdin,
            reset_missing,
            ignore_attribute_errors,
        } => {
            let tags = apply_parent_tag(tags, ptag.as_deref());
            let result = save_all_bookmarks(
                &pool,
                uris,
                tags,
                use_stdin,
                reset_missing,
                ignore_attribute_errors,
            )
            .await?;
            if let Some(stats) = result {
                if stats.num_bookmarks == 1 {
                    println!("saved 1 bookmark");
                } else {
                    println!("saved {} bookmarks", stats.num_bookmarks);
                }
            }
        }

        BmmCommand::Show { uri } => show_bookmark(&pool, uri).await?,

        BmmCommand::Tags { tags_command } => match tags_command {
            TagsCommand::List {
                format,
                show_stats,
                tui,
            } => list_tags(&pool, format, show_stats, tui, db_name.clone()).await?,
            TagsCommand::Rename {
                source_tag,
                target_tag,
            } => {
                let result = rename_tag(&pool, source_tag.clone(), target_tag).await;
                if let Err(crate::cli::RenameTagError::NoSuchTag) = &result {
                    let others = databases_with_tag(source_tag.trim(), &db_name).await;
                    print_tag_hint(source_tag.trim(), &db_name, &others);
                }
                result?
            }
            TagsCommand::Delete {
                tags,
                skip_confirmation,
            } => {
                let result = delete_tags(&pool, tags.clone(), skip_confirmation).await;
                if let Err(crate::cli::DeleteTagsError::TagsDoNotExist(missing)) = &result {
                    for tag in missing {
                        let others = databases_with_tag(tag.trim(), &db_name).await;
                        print_tag_hint(tag.trim(), &db_name, &others);
                    }
                }
                result?
            }
        },
        BmmCommand::Check {
            uri,
            tags,
            limit,
            concurrency,
            timeout,
            show_all,
        } => check_bookmarks(&pool, uri, tags, limit, concurrency, timeout, show_all).await?,

        BmmCommand::Notes { uri, print } => handle_notes_command(&pool, uri, print).await?,

        BmmCommand::Fetch { uri, timeout } => fetch_description(&uri, timeout).await?,

        BmmCommand::Update => handle_update_command().await?,

        BmmCommand::Tui => run_tui(&pool, TuiContext::Initial, db_name.clone()).await?,
        },
    }

    Ok(())
}
