use super::save::{CouldntGetDetailsViaEditorError, get_text_editor_exe};
use crate::persistence::{DBError, NoteError, get_note, set_note};
use sqlx::{Pool, Sqlite};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::process::Command;
use tempfile::tempdir;
use which::which;

#[derive(thiserror::Error, Debug)]
pub enum NotesCommandError {
    #[error("couldn't get current note: {0}")]
    CouldntGetNote(DBError),
    #[error(transparent)]
    CouldntUseTextEditor(#[from] CouldntGetDetailsViaEditorError),
    #[error("couldn't save note: {0}")]
    CouldntSaveNote(NoteError),
}

pub async fn handle_notes_command(
    pool: &Pool<Sqlite>,
    uri: String,
    print_only: bool,
) -> Result<(), NotesCommandError> {
    let existing_note = get_note(pool, &uri)
        .await
        .map_err(NotesCommandError::CouldntGetNote)?;

    if print_only {
        match existing_note {
            Some(note) => println!("{note}"),
            None => println!("(no note saved for this bookmark)"),
        }
        return Ok(());
    }

    let new_note = get_note_via_editor(&uri, existing_note.as_deref())?;

    set_note(pool, &uri, new_note)
        .await
        .map_err(NotesCommandError::CouldntSaveNote)?;

    println!("note saved!");

    Ok(())
}

fn get_note_via_editor(
    uri: &str,
    existing_note: Option<&str>,
) -> Result<Option<String>, CouldntGetDetailsViaEditorError> {
    let tmp_dir = tempdir().map_err(CouldntGetDetailsViaEditorError::CreateTempFile)?;
    let tmp_file_path = tmp_dir.path().join("bmm-note.txt");

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&tmp_file_path)
        .map_err(CouldntGetDetailsViaEditorError::OpenTempFile)?;

    let header = format!(
        "# Note for: {uri}\n#\n# Write your note below this line. Save and close the editor when\n# you're done. Leave everything below blank to remove the note.\n"
    );

    file.write_all(header.as_bytes())
        .map_err(CouldntGetDetailsViaEditorError::WriteToTempFile)?;

    if let Some(note) = existing_note {
        file.write_all(note.as_bytes())
            .map_err(CouldntGetDetailsViaEditorError::WriteToTempFile)?;
        file.write_all(b"\n")
            .map_err(CouldntGetDetailsViaEditorError::WriteToTempFile)?;
    }

    let (editor_exe, env_var_used) = get_text_editor_exe()?;

    let editor_exe_path = which(&editor_exe).map_err(|e| {
        CouldntGetDetailsViaEditorError::CouldntFindEditorExe(editor_exe, env_var_used, e)
    })?;

    let _ = Command::new(&editor_exe_path)
        .arg(&tmp_file_path)
        .status()
        .map_err(|e| CouldntGetDetailsViaEditorError::OpenTextEditor(editor_exe_path, e))?;

    let mut modified_file =
        File::open(&tmp_file_path).map_err(CouldntGetDetailsViaEditorError::OpenTempFile)?;
    let mut modified_contents = String::new();
    modified_file
        .read_to_string(&mut modified_contents)
        .map_err(CouldntGetDetailsViaEditorError::ReadTempFileContents)?;

    let note_body: String = modified_contents
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    let trimmed = note_body.trim();

    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}
