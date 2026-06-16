use anyhow::Result;
use rusqlite::Connection;

use crate::config::Config;
use crate::db;
use crate::model::Item;
use crate::vault;

pub fn is_url(s: &str) -> bool {
    let s = s.trim();
    s.starts_with("http://") || s.starts_with("https://")
}

pub fn fetch_link_title(url: &str) -> Option<String> {
    let resp = reqwest::blocking::get(url).ok()?;
    let html = resp.text().ok()?;
    extract_title(&html).or_else(|| extract_og_title(&html))
}

fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<title>")? + 7;
    let end = lower[start..].find("</title>")? + start;
    let title = html.get(start..end)?.trim().to_string();
    if title.is_empty() {
        None
    } else {
        Some(title)
    }
}

fn extract_og_title(html: &str) -> Option<String> {
    for line in html.lines() {
        let l = line.to_lowercase();
        if l.contains("og:title") {
            if let Some(start) = line.find("content=\"") {
                let rest = &line[start + 9..];
                if let Some(end) = rest.find('"') {
                    let t = rest[..end].trim().to_string();
                    if !t.is_empty() {
                        return Some(t);
                    }
                }
            }
        }
    }
    None
}

pub fn capture_note(conn: &Connection, cfg: &Config, text: &str) -> Result<Item> {
    let store = vault::store_root(cfg)?;
    let title: String = text
        .lines()
        .next()
        .unwrap_or("Untitled")
        .chars()
        .take(80)
        .collect();
    let mut item = Item::new_note(title, text.to_string());
    item.path = Some(vault::item_relative_path(&item));
    vault::write_item_md(&store, &item, &item.body)?;
    db::insert_item(conn, &mut item)?;
    db::record_event(conn, "capture", Some(&item.uuid), Some(&item.kind), &item.tags, item.project.as_deref())?;
    let embed_text = format!("{} {}", item.title, item.body);
    crate::embed::embed_and_store(conn, cfg, &item.uuid, &embed_text)?;
    println!("Captured note {}: {}", item.handle(), item.title);
    Ok(item)
}

pub fn capture_link(conn: &Connection, cfg: &Config, url: &str, note: Option<&str>) -> Result<Item> {
    let store = vault::store_root(cfg)?;
    let title = fetch_link_title(url).unwrap_or_else(|| url.to_string());
    let body = note.unwrap_or("").to_string();
    let mut item = Item::new_link(url.to_string(), title.clone(), body.clone());
    item.path = Some(vault::item_relative_path(&item));
    vault::write_item_md(&store, &item, &body)?;
    db::insert_item(conn, &mut item)?;
    db::record_event(conn, "capture", Some(&item.uuid), Some(&item.kind), &item.tags, item.project.as_deref())?;
    let embed_text = format!("{} {} {}", item.title, url, body);
    crate::embed::embed_and_store(conn, cfg, &item.uuid, &embed_text)?;
    println!("Captured link {}: {}", item.handle(), item.title);
    Ok(item)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_urls() {
        assert!(is_url("https://example.com"));
        assert!(!is_url("hello world"));
    }

    #[test]
    fn extracts_title_from_html() {
        let html = "<html><head><title>Hello</title></head></html>";
        assert_eq!(extract_title(html), Some("Hello".into()));
    }
}
