use std::path::{Path, PathBuf};
use std::time::Instant;

use ignore::WalkBuilder;

const SKIP_DIRS: &[&str] = &[".git", "node_modules", "target", "dist", ".venv", "vendor"];

/// Flat list of workspace-relative file paths for quick-open.
#[derive(Default)]
pub struct FileIndex {
    /// Paths relative to the workspace root, using `/` separators.
    pub paths: Vec<String>,
    built_at: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub rel: String,
    pub abs: PathBuf,
    pub score: i64,
}

impl FileIndex {
    pub fn is_stale(&self, max_age_secs: u64) -> bool {
        match self.built_at {
            None => true,
            Some(t) => t.elapsed().as_secs() >= max_age_secs,
        }
    }

    pub fn built_at_invalidate(&mut self) {
        self.built_at = None;
    }

    pub fn rebuild(&mut self, root: &Path) {
        let mut paths = Vec::new();
        let walker = WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !SKIP_DIRS.iter().any(|s| *s == name)
            })
            .build();

        for entry in walker.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Ok(rel) = path.strip_prefix(root) else {
                continue;
            };
            let rel = rel.to_string_lossy().replace('\\', "/");
            if rel.is_empty() {
                continue;
            }
            paths.push(rel);
        }

        paths.sort_unstable_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        self.paths = paths;
        self.built_at = Some(Instant::now());
    }

    pub fn search(&self, root: &Path, query: &str, limit: usize) -> Vec<SearchHit> {
        let q = query.trim();
        if q.is_empty() {
            return self
                .paths
                .iter()
                .take(limit)
                .map(|rel| SearchHit {
                    abs: root.join(rel),
                    rel: rel.clone(),
                    score: 0,
                })
                .collect();
        }

        let mut hits: Vec<SearchHit> = self
            .paths
            .iter()
            .filter_map(|rel| {
                let score = fuzzy_score(q, rel)?;
                Some(SearchHit {
                    abs: root.join(rel),
                    rel: rel.clone(),
                    score,
                })
            })
            .collect();

        hits.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.rel.len().cmp(&b.rel.len()))
                .then_with(|| a.rel.cmp(&b.rel))
        });
        hits.truncate(limit);
        hits
    }
}

/// Case-insensitive subsequence match with bonuses for consecutive runs,
/// matching at path-component boundaries, and filename-only hits.
fn fuzzy_score(query: &str, candidate: &str) -> Option<i64> {
    let q: Vec<char> = query.chars().map(|c| c.to_ascii_lowercase()).collect();
    let c: Vec<char> = candidate.chars().map(|ch| ch.to_ascii_lowercase()).collect();
    if q.is_empty() {
        return Some(0);
    }
    if q.len() > c.len() {
        return None;
    }

    let mut score: i64 = 0;
    let mut qi = 0;
    let mut prev_match = false;
    let mut first_idx = None;
    let file_start = candidate.rfind('/').map(|i| i + 1).unwrap_or(0);

    for (ci, &ch) in c.iter().enumerate() {
        if qi >= q.len() {
            break;
        }
        if ch != q[qi] {
            prev_match = false;
            continue;
        }

        let mut bonus: i64 = 10;
        if prev_match {
            bonus += 25; // consecutive
        }
        if ci == 0 || matches!(c.get(ci.saturating_sub(1)), Some('/' | '_' | '-' | '.')) {
            bonus += 20; // path / word boundary
        }
        if ci >= file_start {
            bonus += 15; // filename region
            if ci == file_start {
                bonus += 20;
            }
        }

        score += bonus;
        if first_idx.is_none() {
            first_idx = Some(ci);
        }
        qi += 1;
        prev_match = true;
    }

    if qi != q.len() {
        return None;
    }

    // Prefer earlier first match and shorter paths
    if let Some(fi) = first_idx {
        score -= fi as i64;
    }
    score -= candidate.len() as i64 / 4;
    Some(score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subsequence_matches() {
        assert!(fuzzy_score("app", "src/app.rs").is_some());
        assert!(fuzzy_score("sar", "src/app.rs").is_some());
        assert!(fuzzy_score("zzz", "src/app.rs").is_none());
    }

    #[test]
    fn prefers_filename() {
        let a = fuzzy_score("app", "src/app.rs").unwrap();
        let b = fuzzy_score("app", "apple/src/other.rs").unwrap();
        assert!(a > b);
    }
}
