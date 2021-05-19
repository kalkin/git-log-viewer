#![allow(clippy::module_name_repetitions)]

use crossterm::style::{Attribute, Color, ContentStyle, StyledContent};

use git_wrapper::is_ancestor;
use gsi::SubtreeConfig;

use crate::commit::{child_history, Commit, Oid};
use crate::ui::base::search::{Needle, SearchResult};
use crate::ui::base::StyledLine;

#[must_use]
#[allow(dead_code)]
/// # Panics
///
/// Panics when a non merge commit provided
pub fn search_link_recursive(
    working_dir: &str,
    commit: &Commit,
    subtree_modules: &[SubtreeConfig],
    link: &Oid,
) -> Option<(usize, Vec<Commit>)> {
    if !commit.is_merge() {
        panic!("Expected a merge commit");
    }

    let mut commits = child_history(working_dir, commit);
    for (i, c) in commits.iter_mut().enumerate() {
        if !c.is_commit_link() && c.id() == link {
            return Some((i, commits));
        } else if c.is_merge() {
            let bellow = &c.bellow().expect("Expected Merge").to_string();
            let link_id = &link.to_string();
            // Heuristic skip examining merge if link is ancestor of the first child
            if is_ancestor(working_dir, link_id, bellow).unwrap() {
                continue;
            }
            if let Some((pos, mut children)) =
                search_link_recursive(working_dir, c, subtree_modules, link)
            {
                let needle_position = i + pos;
                let mut insert_position = i;
                for child in &mut children {
                    insert_position += 1;
                    commits.insert(insert_position, child.to_owned());
                }
                return Some((needle_position, commits));
            }
        }
    }
    None
}

struct TextMatch {
    start: usize,
    end: usize,
}

#[must_use]
#[allow(clippy::ptr_arg)]
pub fn highlight_search_line(
    line: &StyledLine<String>,
    search_state: &Needle,
) -> StyledLine<String> {
    let mut result = vec![];
    for sc in line {
        result.append(&mut highlight_search(sc, &search_state));
    }
    result
}

fn highlight_search(
    sc: &StyledContent<String>,
    search_state: &Needle,
) -> Vec<StyledContent<String>> {
    let mut cur = 0;
    let mut tmp = vec![];
    let indices = search_styled_content(sc, search_state);
    let mut style = ContentStyle {
        background_color: Some(Color::DarkRed),
        foreground_color: Some(Color::DarkGrey),
        ..ContentStyle::default()
    };
    style.attributes.set(Attribute::Bold);
    for s in indices {
        assert!(s.start >= cur);
        if cur < s.start {
            tmp.push(StyledContent::new(
                *sc.style(),
                sc.content()[cur..s.start].to_string(),
            ));
        }
        cur = s.end;

        tmp.push(StyledContent::new(
            style,
            sc.content()[s.start..s.end].to_string(),
        ));
    }
    if cur < sc.content().len() {
        tmp.push(StyledContent::new(
            *sc.style(),
            sc.content()[cur..].to_string(),
        ));
    }

    tmp
}

fn search_styled_content(sc: &StyledContent<String>, state: &Needle) -> Vec<TextMatch> {
    let haystack = sc.content();
    let needle = state.text();
    let mut result = Vec::new();
    let indices = haystack.match_indices(needle);
    for (i, s) in indices {
        result.push(TextMatch {
            start: i,
            end: i + s.len(),
        })
    }

    result
}

#[allow(clippy::ptr_arg)]
pub fn search_line(line: &StyledLine<String>, state: &Needle) -> Vec<SearchResult> {
    let parts = line
        .iter()
        .map(|sc| sc.content().clone())
        .collect::<Vec<_>>();

    let mut result = vec![];
    let haystack = parts.join("");
    let needle = state.text();
    let indices = haystack.match_indices(needle);
    for (i, s) in indices {
        result.push(SearchResult(vec![i, i + s.len()]))
    }
    result
}
