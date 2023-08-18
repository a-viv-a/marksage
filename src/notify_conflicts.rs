use std::path::PathBuf;

use ntfy::{Dispatcher, Payload};
use url::Url;
use walkdir::WalkDir;

use crate::util::is_sync_conflict;

pub fn notify_conflicts(vault_path: &PathBuf, ntfy_url: Url, topic: String) {
    let sync_conflicts = WalkDir::new(vault_path.clone())
        .into_iter()
        .map(Result::unwrap)
        .filter(is_sync_conflict)
        .map(|e| {
            e.path()
                .clone()
                .strip_prefix(&vault_path)
                .expect("should always be a prefix as the walkdir starts at the vault path")
                .to_str()
                .expect("should always be a valid string")
                .to_string()
        })
        .collect::<Vec<String>>();

    if sync_conflicts.is_empty() {
        println!("No sync conflicts found");
        return;
    }

    match Dispatcher::builder(ntfy_url).build().unwrap().send(
        &Payload::new(topic)
            .title(format!("{} sync conflicts found", sync_conflicts.len()))
            .message(sync_conflicts.join("\n"))
            .priority(ntfy::Priority::High),
    ) {
        Ok(_) => println!("Successfully sent notification"),
        Err(e) => println!("Failed to send notification: {e}"),
    }
}
