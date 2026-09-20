use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    /// Combined working tree vs HEAD (legacy Ctrl+D default).
    Head,
    /// Unstaged changes only (`git diff`).
    Unstaged,
    /// Staged changes only (`git diff --cached`).
    Staged,
}

impl DiffKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Head => "diff",
            Self::Unstaged => "unstaged",
            Self::Staged => "staged",
        }
    }

    pub fn cycle(self) -> Option<Self> {
        match self {
            Self::Head => Some(Self::Unstaged),
            Self::Unstaged => Some(Self::Staged),
            Self::Staged => None,
        }
    }

    pub fn first() -> Self {
        Self::Head
    }
}

#[derive(Debug, Clone)]
pub struct GitEntry {
    pub path: PathBuf,
    pub rel: String,
    /// Index (staged) status char, or `' '` if clean in index.
    pub index: char,
    /// Worktree status char, or `' '` if clean in worktree.
    pub worktree: char,
}

impl GitEntry {
    pub fn is_staged(&self) -> bool {
        self.index != ' ' && self.index != '?'
    }

    pub fn is_unstaged(&self) -> bool {
        self.worktree != ' ' || self.index == '?'
    }

    pub fn display_code(&self) -> String {
        format!("{}{}", self.index, self.worktree)
    }
}

#[derive(Debug, Clone)]
pub struct GitBranch {
    pub name: String,
    pub current: bool,
}

#[derive(Debug, Clone)]
pub struct BlameLine {
    pub short_sha: String,
    pub author: String,
    pub content: String,
}

#[derive(Debug, Default)]
pub struct GitStatus {
    pub map: std::collections::HashMap<PathBuf, char>,
    pub branch: String,
    pub entries: Vec<GitEntry>,
}

impl GitStatus {
    pub fn load(root: &Path) -> Self {
        let mut status = Self {
            map: std::collections::HashMap::new(),
            branch: String::new(),
            entries: Vec::new(),
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
            let bytes = line.as_bytes();
            let index = bytes[0] as char;
            let worktree = bytes[1] as char;
            let path_part = line[3..].trim();
            // Handle renames: "old -> new"
            let path_part = path_part
                .split(" -> ")
                .last()
                .unwrap_or(path_part)
                .trim_matches('"');
            let full = root.join(path_part);
            let code = if index != ' ' && index != '?' {
                index
            } else {
                worktree
            };
            status.map.insert(full.clone(), code);
            status.entries.push(GitEntry {
                path: full.clone(),
                rel: path_part.to_string(),
                index,
                worktree,
            });
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
            .entries
            .sort_by(|a, b| a.rel.to_lowercase().cmp(&b.rel.to_lowercase()));
        status
    }

    pub fn diff_kind(root: &Path, file: &Path, kind: DiffKind) -> Option<String> {
        let rel = file.strip_prefix(root).ok()?;
        let rel_s = rel.display().to_string();
        let root_s = root.display().to_string();

        let mut args: Vec<&str> = vec!["-C", &root_s, "diff"];
        match kind {
            DiffKind::Head => {
                args.push("HEAD");
            }
            DiffKind::Unstaged => {}
            DiffKind::Staged => {
                args.push("--cached");
            }
        }
        args.push("--");
        args.push(&rel_s);

        let out = Command::new("git").args(&args).output().ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        if text.is_empty() {
            let empty = match kind {
                DiffKind::Head => "(no diff vs HEAD — untracked or unchanged)",
                DiffKind::Unstaged => "(no unstaged changes)",
                DiffKind::Staged => "(no staged changes)",
            };
            Some(empty.to_string())
        } else {
            Some(text)
        }
    }

    pub fn stage(root: &Path, file: &Path) -> Result<(), String> {
        let rel = file
            .strip_prefix(root)
            .map_err(|_| "path outside repo".to_string())?;
        let out = Command::new("git")
            .args([
                "-C",
                &root.display().to_string(),
                "add",
                "--",
                &rel.display().to_string(),
            ])
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }

    pub fn unstage(root: &Path, file: &Path) -> Result<(), String> {
        let rel = file
            .strip_prefix(root)
            .map_err(|_| "path outside repo".to_string())?;
        let out = Command::new("git")
            .args([
                "-C",
                &root.display().to_string(),
                "restore",
                "--staged",
                "--",
                &rel.display().to_string(),
            ])
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            // Older git: fall back to reset HEAD
            let out2 = Command::new("git")
                .args([
                    "-C",
                    &root.display().to_string(),
                    "reset",
                    "HEAD",
                    "--",
                    &rel.display().to_string(),
                ])
                .output()
                .map_err(|e| e.to_string())?;
            if out2.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
            }
        }
    }

    pub fn branches(root: &Path) -> Vec<GitBranch> {
        let Ok(out) = Command::new("git")
            .args([
                "-C",
                &root.display().to_string(),
                "branch",
                "--list",
                "--format=%(refname:short)%09%(HEAD)",
            ])
            .output()
        else {
            return Vec::new();
        };
        if !out.status.success() {
            return Vec::new();
        }
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(2, '\t');
                let name = parts.next()?.trim();
                if name.is_empty() {
                    return None;
                }
                let head = parts.next().unwrap_or("").trim();
                Some(GitBranch {
                    name: name.to_string(),
                    current: head == "*",
                })
            })
            .collect()
    }

    pub fn checkout(root: &Path, branch: &str) -> Result<(), String> {
        let out = Command::new("git")
            .args(["-C", &root.display().to_string(), "checkout", branch])
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            Err(if err.is_empty() {
                "checkout failed".into()
            } else {
                err
            })
        }
    }

    pub fn blame(root: &Path, file: &Path) -> Option<Vec<BlameLine>> {
        let rel = file.strip_prefix(root).ok()?;
        let out = Command::new("git")
            .args([
                "-C",
                &root.display().to_string(),
                "blame",
                "--line-porcelain",
                "--",
                &rel.display().to_string(),
            ])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        Some(parse_blame_porcelain(&String::from_utf8_lossy(&out.stdout)))
    }
}

fn parse_blame_porcelain(text: &str) -> Vec<BlameLine> {
    let mut lines = Vec::new();
    let mut short_sha = String::new();
    let mut author = String::new();

    for raw in text.lines() {
        if raw.starts_with('\t') {
            lines.push(BlameLine {
                short_sha: short_sha.clone(),
                author: author.clone(),
                content: raw[1..].to_string(),
            });
            continue;
        }
        if let Some((sha, _)) = raw.split_once(' ') {
            if sha.len() >= 7 && sha.chars().all(|c| c.is_ascii_hexdigit()) {
                short_sha = sha.chars().take(7).collect();
                author.clear();
                continue;
            }
        }
        if let Some(rest) = raw.strip_prefix("author ") {
            author = rest.to_string();
        }
    }
    lines
}
