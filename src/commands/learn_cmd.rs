use anyhow::Result;
use rusqlite::Connection;

use crate::config::Config;
use crate::learn;

pub fn run(conn: &Connection, cfg: &Config) -> Result<()> {
    learn::rebuild_profile(conn, cfg)
}
