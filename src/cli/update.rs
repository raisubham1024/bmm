use crate::self_update::{UpdateError, UpdateOutcome, update_bmm};

/// Handles `bmm update` - checks whether a newer `bmm` binary is
/// available and, if so, installs it everywhere `bmm` is currently found
/// on `$PATH`.
pub async fn handle_update_command() -> Result<(), UpdateError> {
    match update_bmm().await? {
        UpdateOutcome::NothingToUpdate => {
            println!(
                "couldn't find a \"bmm\" binary on $PATH (checked everywhere \"which -a bmm\" \
would); nothing to update"
            );
        }
        UpdateOutcome::UpToDate => {
            println!("Already up to date");
        }
        UpdateOutcome::Updated { locations } => {
            println!("Update available");
            let noun = if locations.len() == 1 {
                "location"
            } else {
                "locations"
            };
            println!("updated bmm at {} {noun}:", locations.len());
            for location in &locations {
                println!("  {}", location.display());
            }
        }
    }

    Ok(())
}
