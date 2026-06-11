use anyhow::Result;
use rusqlite::Connection;

use crate::db;

pub fn annotate(conn: &Connection, id_or_uuid: &str, words: &[String]) -> Result<()> {
    let text = words
        .iter()
        .filter(|w| !w.starts_with("--"))
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    if text.trim().is_empty() {
        anyhow::bail!("Annotation text cannot be empty");
    }
    let task = db::resolve_task(conn, id_or_uuid)?;
    db::add_annotation(conn, &task.uuid, text.trim())?;
    println!("Annotated task {}: {}", task.id.unwrap_or(0), text.trim());
    Ok(())
}

pub fn denotate(conn: &Connection, annotation_id: i64) -> Result<()> {
    if db::delete_annotation(conn, annotation_id)? {
        println!("Removed annotation {annotation_id}.");
    } else {
        anyhow::bail!("No annotation with id {annotation_id}");
    }
    Ok(())
}

pub fn attach(conn: &Connection, id_or_uuid: &str, path: &str) -> Result<()> {
    let task = db::resolve_task(conn, id_or_uuid)?;
    // URLs become navigable/openable links; everything else is a file.
    if db::is_url(path) {
        return link(conn, id_or_uuid, path, None);
    }
    let mut files = db::get_task_files(conn, &task.uuid)?;
    if !files.contains(&path.to_string()) {
        files.push(path.to_string());
    }
    db::set_task_files(conn, &task.uuid, &files)?;
    println!("Attached to task {}: {}", task.id.unwrap_or(0), path);
    Ok(())
}

pub fn link(conn: &Connection, id_or_uuid: &str, url: &str, label: Option<&str>) -> Result<()> {
    let task = db::resolve_task(conn, id_or_uuid)?;
    db::add_link(conn, &task.uuid, url, label)?;
    println!("Linked task {}: {}", task.id.unwrap_or(0), url);
    Ok(())
}

pub fn unlink(conn: &Connection, link_id: i64) -> Result<()> {
    if db::delete_link(conn, link_id)? {
        println!("Removed link {link_id}.");
    } else {
        anyhow::bail!("No link with id {link_id}");
    }
    Ok(())
}
