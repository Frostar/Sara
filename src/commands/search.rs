use anyhow::Result;
use rusqlite::Connection;

use crate::config::Config;
use crate::db;
use crate::embed;

pub fn run(conn: &Connection, cfg: &Config, query: &str) -> Result<()> {
    db::record_event(conn, "search", None, None, &[], None)?;

    match embed::embed_text(cfg, query) {
        Ok(qvec) => {
            let all = db::all_embeddings(conn)?;
            if all.is_empty() {
                println!("No embeddings indexed yet. Capture notes/links first.");
                return keyword_fallback(conn, query);
            }
            let mut scored: Vec<(f32, String)> = all
                .iter()
                .map(|(uuid, vec)| (embed::cosine_similarity(&qvec, vec), uuid.clone()))
                .collect();
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            println!("Results for \"{query}\":\n");
            for (score, uuid) in scored.into_iter().take(10) {
                if score < 0.1 {
                    continue;
                }
                if let Ok(item) = find_item_by_uuid(conn, &uuid) {
                    println!("  {:.2}  {} {} — {}", score, item.kind, item.handle(), item.title);
                }
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("Semantic search unavailable ({e}); falling back to keyword search.");
            keyword_fallback(conn, query)
        }
    }
}

fn find_item_by_uuid(conn: &Connection, uuid: &str) -> Result<crate::model::Item> {
    conn.query_row(
        "SELECT uuid, kind, display_id, title, url, project, tags_json, path, summary, body, created, modified, status
         FROM items WHERE uuid = ?1",
        [uuid],
        |row| {
            let tags_json: String = row.get(6)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            Ok(crate::model::Item {
                uuid: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_else(|_| uuid::Uuid::new_v4()),
                display_id: row.get(2)?,
                kind: row.get(1)?,
                title: row.get(3)?,
                url: row.get(4)?,
                project: row.get(5)?,
                tags,
                path: Some(row.get(7)?),
                summary: row.get(8)?,
                body: row.get(9)?,
                created: chrono::Utc::now(),
                modified: chrono::Utc::now(),
                status: row.get(12)?,
            })
        },
    )
    .map_err(Into::into)
}

fn keyword_fallback(conn: &Connection, query: &str) -> Result<()> {
    let q = query.to_lowercase();
    let items = db::list_items(conn, None)?;
    let mut hits = 0;
    for item in items {
        let hay = format!("{} {} {}", item.title, item.body, item.summary.as_deref().unwrap_or("")).to_lowercase();
        if hay.contains(&q) {
            println!("  {} {} — {}", item.kind, item.handle(), item.title);
            hits += 1;
        }
    }
    if hits == 0 {
        println!("No matches.");
    }
    Ok(())
}
