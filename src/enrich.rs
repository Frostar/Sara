use indicatif::{ProgressBar, ProgressStyle};
use rusqlite::Connection;
use std::path::Path;
use std::time::Duration;

use crate::config::Config;
use crate::files::{build_tree_summary, collect_project_files, extract_keywords, find_relevant_files};
use crate::llm::{self, EnrichmentRequest, EnrichmentResponse};
use crate::model::Project;

/// Run LLM enrichment for a task description.
/// Returns None if LLM is unavailable/fails (caller treats as no suggestions).
pub fn enrich_task(
    conn: &Connection,
    cfg: &Config,
    description: &str,
    project: &Project,
) -> Option<EnrichmentResponse> {
    let project_root = project.path.as_deref().map(Path::new);

    // Gather file context
    let (files, tree_summary) = if let Some(root) = project_root {
        let files = collect_project_files(root);
        let summary = build_tree_summary(root, &files);
        (files, summary)
    } else {
        (vec![], String::new())
    };

    // Gather existing tasks for dep suggestions
    let existing_tasks: Vec<(String, String)> = crate::db::list_tasks(conn, None)
        .unwrap_or_default()
        .into_iter()
        .filter(|t| t.description != description)
        .map(|t| {
            let short = t.uuid.to_string()[..8].to_string();
            (short, t.description.clone())
        })
        .take(20)
        .collect();

    let req = EnrichmentRequest {
        description: description.to_string(),
        project_name: project.name.clone(),
        project_goal: project.goal.clone(),
        project_stack: project.stack.clone(),
        project_notes: project.notes.clone(),
        file_tree_summary: tree_summary,
        existing_tasks,
    };

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Asking LLM for suggestions…");
    spinner.enable_steady_tick(Duration::from_millis(80));

    let provider = llm::build_provider(cfg);
    let result = provider.enrich(&req);
    spinner.finish_and_clear();

    match result {
        Ok(mut resp) => {
            // Drop hallucinated files — keep only those that fuzzy-match real ones
            let keywords = extract_keywords(description);
            let real_files: Vec<String> = files.clone();
            let scored = find_relevant_files(&keywords, &real_files);
            let scored_paths: std::collections::HashSet<String> =
                scored.into_iter().map(|(p, _)| p).collect();

            // Also accept LLM-suggested files if they exist verbatim
            resp.relevant_files.retain(|f| {
                real_files.contains(f) || scored_paths.contains(f)
            });

            // Add fuzzy-matched files not already in the list
            for p in scored_paths {
                if !resp.relevant_files.contains(&p) {
                    resp.relevant_files.push(p);
                }
            }

            Some(resp)
        }
        Err(e) => {
            eprintln!("LLM enrichment failed (continuing without): {e:#}");
            None
        }
    }
}
