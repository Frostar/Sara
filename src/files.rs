use ignore::WalkBuilder;
use nucleo_matcher::{
    pattern::{Atom, AtomKind, CaseMatching, Normalization},
    Config, Matcher,
};
use std::path::Path;

const MAX_FILES: usize = 2000;
const MAX_FILE_SIZE: u64 = 1_000_000; // 1 MB
const SCORE_THRESHOLD: u16 = 30;

/// Walk a project root (gitignore-aware) and collect relative file paths.
pub fn collect_project_files(root: &Path) -> Vec<String> {
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .follow_links(false)
        .add_custom_ignore_filename(".tkignore")
        .build();

    let mut files = Vec::new();
    for entry in walker.flatten() {
        if files.len() >= MAX_FILES {
            break;
        }
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        // Skip large files
        if let Ok(meta) = path.metadata() {
            if meta.len() > MAX_FILE_SIZE {
                continue;
            }
        }
        // Skip binary-ish extensions
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(
                ext.to_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "svg" | "woff"
                    | "woff2" | "ttf" | "eot" | "mp4" | "mp3" | "wav" | "zip"
                    | "tar" | "gz" | "pdf" | "lock"
            ) {
                continue;
            }
        }
        if let Ok(rel) = path.strip_prefix(root) {
            files.push(rel.to_string_lossy().to_string());
        }
    }
    files
}

/// Given task keywords, rank project files by fuzzy relevance.
/// Returns (path, score) pairs above the threshold, sorted by score desc.
pub fn find_relevant_files(
    keywords: &[String],
    project_files: &[String],
) -> Vec<(String, u16)> {
    if keywords.is_empty() || project_files.is_empty() {
        return vec![];
    }

    let query = keywords.join(" ");
    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    let pattern = Atom::new(
        &query,
        CaseMatching::Ignore,
        Normalization::Smart,
        AtomKind::Fuzzy,
        false,
    );

    let mut scored: Vec<(String, u16)> = project_files
        .iter()
        .filter_map(|path| {
            let score = pattern.score(
                nucleo_matcher::Utf32Str::new(path, &mut Vec::new()),
                &mut matcher,
            );
            score.map(|s| (path.clone(), s))
        })
        .filter(|&(_, s)| s >= SCORE_THRESHOLD)
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored.truncate(10);
    scored
}

/// Quick keyword extraction from a task description (naive word split + stop-word filter).
pub fn extract_keywords(description: &str) -> Vec<String> {
    let stop: &[&str] = &[
        "a", "an", "the", "to", "for", "in", "on", "of", "and", "or", "is", "it",
        "with", "that", "this", "at", "by", "be", "as", "we", "do",
    ];
    description
        .split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|w| w.len() > 2 && !stop.contains(&w.as_str()))
        .collect()
}

/// Build a concise file-tree summary string to embed in LLM prompts (max ~100 lines).
pub fn build_tree_summary(root: &Path, files: &[String]) -> String {
    let root_name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");
    let mut lines = vec![format!("{root_name}/")];
    for f in files.iter().take(80) {
        lines.push(format!("  {f}"));
    }
    if files.len() > 80 {
        lines.push(format!("  ... ({} more files)", files.len() - 80));
    }
    lines.join("\n")
}

// ── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keywords() {
        let kw = extract_keywords("Fix the broken login form for users");
        assert!(kw.contains(&"fix".to_string()));
        assert!(kw.contains(&"broken".to_string()));
        assert!(kw.contains(&"login".to_string()));
        assert!(!kw.contains(&"the".to_string()));
    }
}
