use anyhow::Result;
use rusqlite::Connection;

use crate::db;

pub fn run(conn: &Connection) -> Result<()> {
    match db::undo(conn)? {
        Some(command) => println!("Undid: {command}"),
        None => println!("Nothing to undo."),
    }
    Ok(())
}
