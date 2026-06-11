use anyhow::Result;
use chrono::Local;
use rusqlite::Connection;

use crate::db;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const CYAN: &str = "\x1b[36m";

pub fn run(conn: &Connection, id_or_uuid: &str) -> Result<()> {
    let task = db::resolve_task(conn, id_or_uuid)?;
    let no_color = std::env::var("NO_COLOR").is_ok();

    let label = |k: &str| {
        if no_color {
            format!("{k:<14}")
        } else {
            format!("{DIM}{k:<14}{RESET}")
        }
    };

    let title = if no_color {
        format!("Task {}", task.id.unwrap_or(0))
    } else {
        format!("{BOLD}{CYAN}Task {}{RESET}", task.id.unwrap_or(0))
    };
    println!("{title}");
    println!();

    println!("{}{}", label("Description"), task.description);
    println!("{}{}", label("Project"), task.project);
    println!("{}{}", label("Status"), task.status);
    println!(
        "{}{}",
        label("Priority"),
        task.priority
            .as_ref()
            .map(|p| p.label().to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "{}{}",
        label("Due"),
        task.due
            .map(|d| d.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "{}{}",
        label("Tags"),
        if task.tags.is_empty() {
            "-".to_string()
        } else {
            task.tags.join(", ")
        }
    );
    println!("{}{:.1}", label("Urgency"), task.urgency);
    println!(
        "{}{}",
        label("Entered"),
        task.entry.with_timezone(&Local).format("%Y-%m-%d %H:%M")
    );
    println!(
        "{}{}",
        label("Modified"),
        task.modified.with_timezone(&Local).format("%Y-%m-%d %H:%M")
    );
    println!("{}{}", label("UUID"), task.uuid);

    // Dependencies
    let blockers = db::get_blockers(conn, &task.uuid)?;
    if !blockers.is_empty() {
        let ids: Vec<String> = blockers
            .iter()
            .filter_map(|u| {
                db::get_task_by_uuid_prefix(conn, &u.to_string()[..8])
                    .ok()
                    .flatten()
            })
            .map(|t| format!("{} ({})", t.id.unwrap_or(0), t.description))
            .collect();
        println!("{}{}", label("Blocked by"), ids.join(", "));
    }
    let blocking = db::get_blocking(conn, &task.uuid)?;
    if !blocking.is_empty() {
        let ids: Vec<String> = blocking
            .iter()
            .filter_map(|u| {
                db::get_task_by_uuid_prefix(conn, &u.to_string()[..8])
                    .ok()
                    .flatten()
            })
            .map(|t| format!("{} ({})", t.id.unwrap_or(0), t.description))
            .collect();
        println!("{}{}", label("Blocking"), ids.join(", "));
    }

    // Files
    let files = db::get_task_files(conn, &task.uuid)?;
    if !files.is_empty() {
        println!("{}", label("Files"));
        for f in files {
            println!("  {f}");
        }
    }

    Ok(())
}
