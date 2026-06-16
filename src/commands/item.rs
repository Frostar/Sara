use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::config::Config;
use crate::db;
use crate::model::Item;
use crate::vault;

pub fn run(conn: &Connection, cfg: &Config, handle: &str) -> Result<()> {
    let item = db::get_item_by_handle(conn, handle)?;
    let store = vault::store_root(cfg)?;
    let path = item.path.as_deref().context("Item has no path")?;
    let full = store.join(path);
    let content = std::fs::read_to_string(&full)
        .with_context(|| format!("Failed to read {}", full.display()))?;

    println!("{} {} — {}", item.kind.to_uppercase(), item.handle(), item.title);
    if let Some(ref url) = item.url {
        println!("URL: {url}");
    }
    if !item.tags.is_empty() {
        println!("Tags: {}", item.tags.join(", "));
    }
    if let Some(ref s) = item.summary {
        println!("Summary: {s}");
    }
    println!();
    if content.contains("---") {
        if let Ok((_, body)) = vault::parse_item_md(&content) {
            if !body.is_empty() {
                println!("{body}");
            }
        }
    } else {
        println!("{content}");
    }
    Ok(())
}

pub fn delete_item(conn: &Connection, cfg: &Config, handle: &str) -> Result<()> {
    let item = db::get_item_by_handle(conn, handle)?;
    let store = vault::store_root(cfg)?;
    if let Some(ref path) = item.path {
        vault::archive_item_md(&store, path)?;
    }
    db::archive_item(conn, &item.uuid)?;
    db::record_event(conn, "delete", Some(&item.uuid), Some(&item.kind), &[], item.project.as_deref())?;
    println!("Archived {}", item.handle());
    Ok(())
}

pub fn edit_item_body(conn: &Connection, cfg: &Config, handle: &str, new_body: &str) -> Result<()> {
    let mut item = db::get_item_by_handle(conn, handle)?;
    item.body = new_body.to_string();
    item.modified = chrono::Utc::now();
    let store = vault::store_root(cfg)?;
    vault::write_item_md(&store, &item, new_body)?;
    db::update_item(conn, &item)?;
    db::record_event(conn, "edit", Some(&item.uuid), Some(&item.kind), &item.tags, item.project.as_deref())?;
    println!("Updated {}", item.handle());
    Ok(())
}
