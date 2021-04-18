use cursive::theme::{BaseColor, Color, ColorType, Effect, Style};

use git_subtrees_improved::SubtreeConfig;
use git_wrapper::is_ancestor;

use crate::core::{child_history, merge_base, Commit};

#[derive(Clone)]
pub enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Clone)]
pub struct SearchState {
    pub active: bool,
    pub needle: String,
    style: Style,
    pub direction: SearchDirection,
}

impl SearchState {
    pub fn new(default_style: Style) -> Self {
        let mut style = default_style;
        style.color.back = ColorType::Color(Color::Dark(BaseColor::Red));
        style.color.front = ColorType::Color(Color::Dark(BaseColor::White));
        style.effects |= Effect::Bold;
        SearchState {
            active: false,
            needle: "".to_string(),
            style,
            direction: SearchDirection::Forward,
        }
    }

    pub fn style(&self) -> Style {
        self.style
    }
}

pub fn search_recursive(
    working_dir: &str,
    commit: &Commit,
    subtree_modules: &[SubtreeConfig],
    search_state: &SearchState,
) -> Option<(usize, Vec<Commit>)> {
    assert!(commit.is_merge(), "Expected a merge commit");

    let mut commits = child_history(working_dir, commit, subtree_modules);
    for (i, c) in commits.iter_mut().enumerate() {
        if c.search_matches(&search_state.needle, true) {
            return Some((i, commits));
        } else if c.is_merge() {
            if let Some((pos, mut children)) =
                search_recursive(working_dir, c, subtree_modules, search_state)
            {
                let needle_position = i + pos;
                let mut insert_position = i;
                for child in children.iter_mut() {
                    insert_position += 1;
                    commits.insert(insert_position, child.to_owned());
                }
                return Some((needle_position, commits));
            }
        }
    }

    None
}

pub fn search_link_recursive(
    working_dir: &str,
    commit: &Commit,
    subtree_modules: &[SubtreeConfig],
    link: &Commit,
) -> Option<(usize, Vec<Commit>)> {
    assert!(commit.is_merge(), "Expected a merge commit");

    let mut commits = child_history(working_dir, commit, subtree_modules);
    for (i, c) in commits.iter_mut().enumerate() {
        if !c.is_commit_link() && c.id() == link.id() {
            return Some((i, commits));
        } else if c.is_merge() {
            let bellow = &c.bellow().expect("Expected Merge").to_string();
            let link_id = &link.id().to_string();
            // Heuristic skip examining merge if link is ancestor of the first child
            if is_ancestor(working_dir, link_id, bellow).unwrap() {
                continue;
            }
            if let Some((pos, mut children)) =
                search_link_recursive(working_dir, c, subtree_modules, link)
            {
                let needle_position = i + pos;
                let mut insert_position = i;
                for child in children.iter_mut() {
                    insert_position += 1;
                    commits.insert(insert_position, child.to_owned());
                }
                return Some((needle_position, commits));
            }
        }
    }
    None
}
