use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub expanded: bool,
    pub children: Option<Vec<TreeNode>>,
    pub git: Option<char>,
}

#[derive(Debug, Clone)]
pub struct FlatEntry {
    pub depth: u16,
    pub node_path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub expanded: bool,
    pub git: Option<char>,
}

pub struct FileTree {
    pub root: TreeNode,
    pub flat: Vec<FlatEntry>,
    pub selected: usize,
    pub scroll: usize,
    git: HashMap<PathBuf, char>,
}

impl FileTree {
    pub fn new(root: PathBuf, git: HashMap<PathBuf, char>) -> Self {
        let name = root
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.display().to_string());
        let git_char = git.get(&root).copied();
        let mut tree = Self {
            root: TreeNode {
                name,
                path: root,
                is_dir: true,
                expanded: true,
                children: None,
                git: git_char,
            },
            flat: Vec::new(),
            selected: 0,
            scroll: 0,
            git,
        };
        tree.ensure_children(&tree.root.path.clone());
        tree.rebuild_flat();
        tree
    }

    pub fn set_git(&mut self, git: HashMap<PathBuf, char>) {
        self.git = git;
        Self::apply_git(&mut self.root, &self.git);
        self.rebuild_flat();
    }

    fn apply_git(node: &mut TreeNode, git: &HashMap<PathBuf, char>) {
        node.git = git.get(&node.path).copied();
        if let Some(children) = node.children.as_mut() {
            for child in children {
                Self::apply_git(child, git);
            }
        }
    }

    fn ensure_children(&mut self, path: &Path) {
        let git = self.git.clone();
        if let Some(node) = Self::find_mut(&mut self.root, path) {
            if node.children.is_some() {
                return;
            }
            node.children = Some(load_children(&node.path, &git));
        }
    }

    fn find_mut<'a>(node: &'a mut TreeNode, path: &Path) -> Option<&'a mut TreeNode> {
        if node.path == path {
            return Some(node);
        }
        if let Some(children) = node.children.as_mut() {
            for child in children {
                if path.starts_with(&child.path) {
                    if let Some(found) = Self::find_mut(child, path) {
                        return Some(found);
                    }
                }
            }
        }
        None
    }

    pub fn rebuild_flat(&mut self) {
        self.flat.clear();
        flatten(&self.root, 0, &mut self.flat);
        if self.selected >= self.flat.len() && !self.flat.is_empty() {
            self.selected = self.flat.len() - 1;
        }
    }

    pub fn toggle_selected(&mut self) -> Option<PathBuf> {
        let Some(entry) = self.flat.get(self.selected).cloned() else {
            return None;
        };
        if entry.is_dir {
            let path = entry.node_path.clone();
            self.ensure_children(&path);
            if let Some(node) = Self::find_mut(&mut self.root, &path) {
                node.expanded = !node.expanded;
            }
            self.rebuild_flat();
            None
        } else {
            Some(entry.node_path)
        }
    }

    pub fn open_at_flat_index(&mut self, index: usize) -> Option<PathBuf> {
        if index >= self.flat.len() {
            return None;
        }
        self.selected = index;
        self.toggle_selected()
    }

    pub fn move_sel(&mut self, delta: isize) {
        if self.flat.is_empty() {
            return;
        }
        let len = self.flat.len() as isize;
        let next = (self.selected as isize + delta).clamp(0, len - 1) as usize;
        self.selected = next;
    }

    /// Scroll the viewport without wrapping. Keeps selection in view.
    pub fn scroll_by(&mut self, delta: isize, height: usize) {
        if self.flat.is_empty() || height == 0 {
            return;
        }
        let max_scroll = self.flat.len().saturating_sub(height);
        let next = self.scroll as isize + delta;
        self.scroll = next.clamp(0, max_scroll as isize) as usize;

        // Keep selection inside the visible window
        if self.selected < self.scroll {
            self.selected = self.scroll;
        } else if self.selected >= self.scroll + height {
            self.selected = self.scroll + height - 1;
        }
    }

    pub fn ensure_visible(&mut self, height: usize) {
        if height == 0 || self.flat.is_empty() {
            return;
        }
        let max_scroll = self.flat.len().saturating_sub(height);
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + height {
            self.scroll = (self.selected + 1).saturating_sub(height);
        }
        self.scroll = self.scroll.min(max_scroll);
    }

    pub fn reveal(&mut self, path: &Path) {
        let mut cursor = self.root.path.clone();
        let Ok(rel) = path.strip_prefix(&self.root.path) else {
            return;
        };
        for component in rel.components() {
            cursor.push(component);
            if cursor.is_dir() {
                self.ensure_children(&cursor);
                if let Some(node) = Self::find_mut(&mut self.root, &cursor) {
                    node.expanded = true;
                }
            }
        }
        self.rebuild_flat();
        if let Some(idx) = self.flat.iter().position(|e| e.node_path == path) {
            self.selected = idx;
        }
    }
}

fn flatten(node: &TreeNode, depth: u16, out: &mut Vec<FlatEntry>) {
    out.push(FlatEntry {
        depth,
        node_path: node.path.clone(),
        name: node.name.clone(),
        is_dir: node.is_dir,
        expanded: node.expanded,
        git: node.git,
    });
    if node.is_dir && node.expanded {
        if let Some(children) = node.children.as_ref() {
            for child in children {
                flatten(child, depth + 1, out);
            }
        }
    }
}

fn load_children(dir: &Path, git: &HashMap<PathBuf, char>) -> Vec<TreeNode> {
    let mut children = Vec::new();
    let walker = WalkBuilder::new(dir)
        .max_depth(Some(1))
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != ".git" && name != "node_modules" && name != "target" && name != "dist"
        })
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if path == dir {
            continue;
        }
        let Ok(meta) = fs::metadata(path) else {
            continue;
        };
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        children.push(TreeNode {
            name,
            path: path.to_path_buf(),
            is_dir: meta.is_dir(),
            expanded: false,
            children: None,
            git: git.get(path).copied(),
        });
    }

    children.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    children
}
