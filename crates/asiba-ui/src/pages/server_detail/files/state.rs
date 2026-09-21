use std::collections::BTreeSet;

use asiba_core::ServerId;
use egui::{Context, Id};

const TREE_KEY: &str = "files-tree";
const EDITOR_KEY: &str = "files-editor";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameKind {
    NewFile,
    NewDirectory,
    Rename,
    Move,
    Copy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Draft {
    Permissions {
        path: String,
        mode: String,
        owner: String,
    },
    Name {
        path: String,
        kind: NameKind,
        value: String,
    },
}

impl Draft {
    pub fn anchor(&self) -> &str {
        match self {
            Self::Permissions { path, .. } | Self::Name { path, .. } => path,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TreeState {
    pub expanded: BTreeSet<String>,
    pub draft: Option<Draft>,
    pub pending_open: Option<String>,
    pub selected: Option<String>,
    pub search_text: String,
}

impl TreeState {
    pub fn is_expanded(&self, path: &str) -> bool {
        self.expanded.contains(path)
    }

    pub fn toggle(&mut self, path: &str) {
        if !self.expanded.remove(path) {
            self.expanded.insert(path.to_owned());
        }
    }

    pub fn toggle_draft(&mut self, draft: Draft) {
        let is_same = self.draft.as_ref() == Some(&draft);
        self.draft = if is_same { None } else { Some(draft) };
    }

    pub fn reveal(&mut self, path: &str) {
        let mut ancestor = String::new();
        for segment in path.trim_matches('/').split('/') {
            ancestor.push('/');
            ancestor.push_str(segment);
            if ancestor != path {
                self.expanded.insert(ancestor.clone());
            }
        }
        self.selected = Some(path.to_owned());
    }

    pub fn has_draft_for(&self, path: &str) -> bool {
        self.draft.as_ref().is_some_and(|d| d.anchor() == path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Editor {
    pub path: String,
    pub text: String,
    pub original: String,
}

impl Editor {
    pub fn new(path: &str, content: &str) -> Self {
        Self {
            path: path.to_owned(),
            text: content.to_owned(),
            original: content.to_owned(),
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.text != self.original
    }
}

pub fn load_tree(ctx: &Context, server: &ServerId) -> TreeState {
    ctx.data(|d| d.get_temp(Id::new((TREE_KEY, server.as_str()))))
        .unwrap_or_default()
}

pub fn store_tree(ctx: &Context, server: &ServerId, state: TreeState) {
    ctx.data_mut(|d| d.insert_temp(Id::new((TREE_KEY, server.as_str())), state));
}

pub fn load_editor(ctx: &Context, server: &ServerId) -> Option<Editor> {
    ctx.data(|d| d.get_temp(Id::new((EDITOR_KEY, server.as_str()))))
}

pub fn store_editor(ctx: &Context, server: &ServerId, editor: Option<Editor>) {
    let id = Id::new((EDITOR_KEY, server.as_str()));
    ctx.data_mut(|d| match editor {
        Some(editor) => {
            d.insert_temp(id, editor);
        }
        None => d.remove::<Editor>(id),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reveal_expands_ancestors_and_selects_target() {
        let mut state = TreeState::default();
        state.reveal("/etc/ssh/sshd_config");
        assert!(state.is_expanded("/etc"));
        assert!(state.is_expanded("/etc/ssh"));
        assert!(!state.is_expanded("/etc/ssh/sshd_config"));
        assert_eq!(state.selected.as_deref(), Some("/etc/ssh/sshd_config"));
    }
}
