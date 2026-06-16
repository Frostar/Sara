use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::config::Config;
use crate::db;

pub fn rebuild_profile(conn: &Connection, cfg: &Config) -> Result<()> {
    let store = crate::vault::store_root(cfg)?;
    let profile_path = store.join(".sara/profile.md");
    let events = db::recent_events(conn, 500)?;

    let mut kind_counts: HashMap<String, u32> = HashMap::new();
    let mut action_counts: HashMap<String, u32> = HashMap::new();
    for (action, kind, _at) in &events {
        *action_counts.entry(action.clone()).or_insert(0) += 1;
        if let Some(k) = kind {
            *kind_counts.entry(k.clone()).or_insert(0) += 1;
        }
    }

    let notes = db::list_items(conn, Some("note")).unwrap_or_default();
    let links = db::list_items(conn, Some("link")).unwrap_or_default();

    let mut body = String::from("# Sara's profile\n\n");
    body.push_str("_Auto-generated from your behavior. Sara uses this to personalize enrichment and surfacing._\n\n");
    body.push_str("## Activity\n\n");
    for (action, count) in action_counts {
        body.push_str(&format!("- {action}: {count}\n"));
    }
    body.push_str("\n## Capture mix\n\n");
    for (kind, count) in kind_counts {
        body.push_str(&format!("- {kind}: {count}\n"));
    }
    body.push_str(&format!("\n## Store size\n\n- {} notes\n- {} links\n", notes.len(), links.len()));

    write_profile(&profile_path, &body)?;
    println!("Profile updated at {}", profile_path.display());
    Ok(())
}

pub fn read_profile_context(cfg: &Config) -> Option<String> {
    let store = crate::config::vault_path(cfg).ok()?;
    let profile_path = store.join(".sara/profile.md");
    let content = fs::read_to_string(&profile_path).ok()?;
    Some(content.chars().take(2000).collect())
}

fn write_profile(path: &Path, body: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = format!("---\ntype: profile\n---\n\n{body}");
    fs::write(path, content)?;
    Ok(())
}
