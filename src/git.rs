use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Default)]
pub struct GitStatus {
    pub map: std::collections::HashMap<PathBuf, char>,
    pub branch: String,
}

impl GitStatus {
    pub fn load(root: &Path) -> Self {
        let mut status = Self {
            map: std::collections::HashMap::new(),
            branch: String::new(),
        };

        if let Ok(out) = Command::new("git")
            .args(["-C", &root.display().to_string(), "rev-parse", "--abbrev-ref", "HEAD"])
            .output()
        {
            if out.status.success() {
                status.branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
            }
        }

        let Ok(out) = Command::new("git")
            .args(["-C", &root.display().to_string(), "status", "--porcelain", "-u"])
            .output()
        else {
            return status;
        };
        if !out.status.success() {
            return status;
        }

        for line in String::from_utf8_lossy(&out.stdout).lines() {
            if line.len() < 4 {
                continue;
            }
            let code = line.as_bytes()[0] as char;
            let code = if code == ' ' {
                line.as_bytes()[1] as char
            } else {
                code
            };
            let path_part = line[3..].trim();
            // Handle renames: "old -> new"
            let path_part = path_part
                .split(" -> ")
                .last()
                .unwrap_or(path_part)
                .trim_matches('"');
            let full = root.join(path_part);
            status.map.insert(full.clone(), code);
            // Propagate dirty marker up to parents
            let mut parent = full.parent().map(|p| p.to_path_buf());
            while let Some(p) = parent {
                if p == root || !p.starts_with(root) {
                    status.map.entry(p.clone()).or_insert('•');
                    break;
                }
                status.map.entry(p.clone()).or_insert('•');
                parent = p.parent().map(|p| p.to_path_buf());
            }
        }

        status
    }

    pub fn diff(root: &Path, file: &Path) -> Option<String> {
        let rel = file.strip_prefix(root).ok()?;
        let out = Command::new("git")
            .args([
                "-C",
                &root.display().to_string(),
                "diff",
                "HEAD",
                "--",
                &rel.display().to_string(),
            ])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        if text.is_empty() {
            Some("(no diff vs HEAD — untracked or unchanged)".to_string())
        } else {
            Some(text)
        }
    }
}
