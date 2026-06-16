use anyhow::Result;
use rusqlite::Connection;

use crate::config::Config;
use crate::db;

pub fn run(conn: &Connection, cfg: &Config) -> Result<()> {
    let _ = cfg;
    println!("Sara's brief — what matters now:\n");

    let tasks = db::list_tasks(conn, None)?;
    let mut top: Vec<_> = tasks.into_iter().take(5).collect();
    top.sort_by(|a, b| b.urgency.partial_cmp(&a.urgency).unwrap_or(std::cmp::Ordering::Equal));

    if top.is_empty() {
        println!("  No pending tasks.");
    } else {
        println!("Tasks (by urgency):");
        for t in top {
            println!(
                "  {} [{:.1}] {}",
                t.id.unwrap_or(0),
                t.urgency,
                t.description
            );
        }
    }

    let notes = db::list_items(conn, Some("note")).unwrap_or_default();
    let links = db::list_items(conn, Some("link")).unwrap_or_default();
    if !notes.is_empty() || !links.is_empty() {
        println!("\nRecent captures:");
        for item in notes.iter().chain(links.iter()).take(5) {
            println!("  {} {} — {}", item.kind, item.handle(), item.title);
        }
    }

    Ok(())
}
