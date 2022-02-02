// Copyright (C) 2021  Bahtiar `kalkin-` Gadimov <bahtiar@gadimov.de>
//
// This file is part of git-log-viewer
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::sync::mpsc::{Receiver, Sender};

use git_stree::{SubtreeConfig, Subtrees};
use posix_errors::PosixError;
use subject_classifier::Subject;
use url::Url;

use crate::actors::fork_point::ForkPointThread;
use crate::actors::github::{GitHubRequest, GitHubThread};
use crate::actors::subtrees::{SubtreeChangesRequest, SubtreeThread};
use crate::commit::{child_history, commits_for_range, history_length, parse_remote_url, Commit};
use crate::history_entry::HistoryEntry;
use crate::ui::base::data::{DataAdapter, SearchProgress};
use crate::ui::base::search::{Direction, Needle, SearchResult};
use crate::ui::base::StyledLine;
use git_wrapper::Remote;
use git_wrapper::Repository;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;

pub struct HistoryAdapter {
    history: Vec<HistoryEntry>,
    length: usize,
    paths: Vec<String>,
    remotes: Vec<Remote>,
    range: String,
    repo: Repository,
    forge_url: Option<Url>,
    github_thread: GitHubThread,
    fork_point_thread: ForkPointThread,
    subtree_modules: Vec<SubtreeConfig>,
    subtree_thread: SubtreeThread,
    search_thread: Option<JoinHandle<()>>,
}

#[derive(Clone)]
struct RangePart {
    i: usize,
    id: String,
}

struct CommitRange {
    start: RangePart,
    end: RangePart,
    level: usize,
}
impl Debug for CommitRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut indent = "".to_string();
        for _ in 0..self.level {
            indent.push(' ');
        }

        let text = format!(
            "#{: >5}…{: <5} {}{}..{} ({})",
            self.start.i, self.end.i, indent, self.end.id, self.start.id, self.level
        );
        f.write_str(&text)
    }
}

pub struct AdapterState(Vec<CommitRange>);

impl Debug for AdapterState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(&self.0).finish()
    }
}

impl From<&HistoryAdapter> for AdapterState {
    fn from(adapter: &HistoryAdapter) -> Self {
        let mut result: Vec<CommitRange> = vec![];
        if !(adapter.history.is_empty()) {
            return Self(result);
        }
        let mut start = RangePart {
            i: 0,
            id: adapter.history[0].short_id().clone(),
        };
        let mut level = 0;
        for (i, e) in adapter.history.iter().enumerate() {
            if level != e.level() {
                let prev = &adapter.history[i - 1];
                let end = RangePart {
                    i: i - 1,
                    id: prev.short_id().clone(),
                };
                let range = CommitRange {
                    start: start.clone(),
                    end,
                    level: level as usize,
                };
                result.push(range);
                start = RangePart {
                    i,
                    id: e.short_id().clone(),
                };
                level = e.level();
            }
        }
        let end = RangePart {
            i: adapter.history.len() - 1,
            id: adapter.history.last().expect("smth").short_id().clone(),
        };
        let range = CommitRange {
            start,
            end,
            level: level as usize,
        };
        result.push(range);
        Self(result)
    }
}

fn find_forge_url(hash_map: &HashMap<String, Remote>) -> Option<Url> {
    if let Some(remote) = hash_map.get("origin") {
        if let Some(s) = &remote.fetch {
            if let Some(u) = parse_remote_url(s) {
                return Some(u);
            }
        }
        if let Some(s) = &remote.push {
            if let Some(u) = parse_remote_url(s) {
                return Some(u);
            }
        }
    }
    for r in hash_map.values() {
        if let Some(s) = &r.fetch {
            if let Some(u) = parse_remote_url(s) {
                return Some(u);
            }
        }
        if let Some(s) = &r.push {
            if let Some(u) = parse_remote_url(s) {
                return Some(u);
            }
        }
    }
    None
}

impl HistoryAdapter {
    ///
    /// # Errors
    ///
    /// Will return an error if git `working_dir` does not exist or git executable is missing
    pub fn new(repo: Repository, range: &str, paths: Vec<String>) -> Result<Self, PosixError> {
        let remotes: Vec<Remote>;
        let forge_url: Option<Url>;
        if let Some(hash_map) = repo.remotes() {
            forge_url = find_forge_url(&hash_map);
            remotes = hash_map
                .into_iter()
                .map(|(_k, v)| v)
                .collect::<Vec<Remote>>();
        } else {
            forge_url = None;
            remotes = vec![];
        }
        if forge_url.is_some() {
            log::debug!("Found forge url {}", forge_url.clone().unwrap());
        }

        let length = history_length(&repo, range, &paths)?;
        let subtrees = Subtrees::from_repo(repo.clone()).expect("Read subtree config");
        let subtree_modules = subtrees.all().unwrap();
        let subtree_thread = SubtreeThread::new(subtrees);
        let fork_point_thread = ForkPointThread::new(repo.clone());
        Ok(Self {
            history: vec![],
            length,
            paths,
            remotes,
            forge_url,
            range: range.to_string(),
            repo,
            github_thread: GitHubThread::new(),
            fork_point_thread,
            subtree_modules,
            subtree_thread,
            search_thread: None,
        })
    }

    pub fn unfold_up_to(&mut self, sr: &SearchResult) -> usize {
        assert!(!sr.0.is_empty(), "Unexpected empty SearchResult vector");
        let addresses = &sr.0;
        let mut result = 0;
        let last_level = addresses.len();
        for (level, addr) in addresses.iter().enumerate() {
            result = self.addr_to_index(result, level, *addr);
            let entry = self.get_data(result);
            if last_level - 1 != (entry.level() as usize) {
                assert_eq!(entry.level() as usize, level);
                result += 1;
                if entry.is_foldable() {
                    if entry.is_folded() {
                        self.toggle_folding(result);
                    }
                } else {
                    log::warn!("Error during converting a SearchResult to HistoryAdapter index");
                    log::warn!(
                        "Unfoldable entry {} on level {} from {:?}",
                        entry.short_id(),
                        level,
                        addresses
                    );
                    break;
                }
            }
        }

        result
    }
    fn addr_to_index(&mut self, start_index: usize, level: usize, addr: usize) -> usize {
        assert_eq!(self.get_data(start_index).level() as usize, level);
        let mut result: usize = 0;
        let mut stop: usize = 0;
        for i in start_index..self.length {
            let entry = self.get_data(i);
            if entry.level() as usize == level {
                result = i;
                if stop == addr {
                    break;
                }
                stop += 1;
            }
        }

        result
    }

    // TODO return nothing
    fn fill_up(&mut self, max: usize) -> bool {
        let skip = self.history.len();
        let range = self.range.as_str();
        let tmp = commits_for_range(
            &self.repo,
            range,
            self.paths.as_ref(),
            Some(skip),
            Some(max),
        );
        if tmp.is_empty() {
            return false;
        }
        let mut above_commit = if self.history.is_empty() {
            None
        } else {
            Some(self.history.last().expect("a commit").commit())
        };
        let mut tmp2 = Vec::with_capacity(tmp.len());
        for commit in tmp {
            if !self.subtree_modules.is_empty() {
                self.subtree_thread.send(SubtreeChangesRequest {
                    oid: commit.id().clone(),
                });
            }
            let fork_point = self
                .fork_point_thread
                .request_calculation(&commit, above_commit);
            let entry =
                HistoryEntry::new(commit, 0, self.forge_url.clone(), fork_point, &self.remotes);
            if let Some(url) = entry.url() {
                if let Subject::PullRequest { id, .. } = entry.special() {
                    self.github_thread.send(GitHubRequest {
                        oid: entry.id().clone(),
                        url,
                        pr_id: id.to_string(),
                    });
                }
            }
            tmp2.push(entry);
            above_commit = Some(tmp2.last().expect("a commit").commit());
        }
        self.history.append(tmp2.as_mut());
        true
    }

    fn is_fill_up_needed(&self, i: usize) -> bool {
        i >= self.history.len()
    }

    pub fn default_action(&mut self, i: usize) {
        if self.history[i].is_foldable() {
            self.toggle_folding(i);
        }
    }

    fn toggle_folding(&mut self, i: usize) {
        let pos = i + 1;
        let mut tmp: Vec<HistoryEntry> = vec![];
        let selected = &self.history[i];
        if selected.is_folded() {
            let children: Vec<Commit> = child_history(&self.repo, selected.commit(), &self.paths);
            let mut above_commit = Some(selected.commit());
            for t in children {
                if !self.subtree_modules.is_empty() {
                    self.subtree_thread.send(SubtreeChangesRequest {
                        oid: t.id().clone(),
                    });
                }
                let fork_point_calc = self.fork_point_thread.request_calculation(&t, above_commit);
                let level = selected.level() + 1;
                let entry: HistoryEntry =
                    HistoryEntry::new(t, level, selected.url(), fork_point_calc, &self.remotes);
                if let Some(url) = entry.url() {
                    if let Subject::PullRequest { id, .. } = entry.special() {
                        self.github_thread.send(GitHubRequest {
                            oid: entry.id().clone(),
                            url,
                            pr_id: id.to_string(),
                        });
                    }
                }
                tmp.push(entry);
                above_commit = Some(tmp.last().expect("a commit").commit());
            }
        } else {
            let level = selected.level();
            while let Some(e) = self.history.get(pos) {
                if e.level() > level {
                    self.history.remove(pos);
                    self.length -= 1;
                } else {
                    break;
                }
            }
        }
        let unfolding = tmp.is_empty();
        self.history[i].folded(unfolding);
        if !unfolding {
            for (i, entry) in tmp.into_iter().enumerate() {
                self.history.insert(pos + i, entry);
                self.length += 1;
            }
        }
    }

    /// Run this function before accessing data, to update data calculated by other threads
    pub fn update(&mut self) {
        while let Ok(v) = self.fork_point_thread.try_recv() {
            for e in &mut self.history {
                if e.id() == &v.oid {
                    e.set_fork_point(v.value);
                    break;
                }
            }
        }
        while let Ok(v) = self.subtree_thread.try_recv() {
            for e in &mut self.history {
                if e.id() == &v.oid {
                    e.set_subtrees(v.subtrees);
                    break;
                }
            }
        }
        while let Ok(v) = self.github_thread.try_recv() {
            for e in &mut self.history {
                if e.id() == &v.oid {
                    e.set_subject(v.subject);
                    break;
                }
            }
        }
    }
}

impl Debug for HistoryAdapter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HistoryAdapter")
            .field("range", &self.range)
            .field("length", &self.length)
            .field("loaded", &self.history.len())
            .field("paths", &self.paths)
            .field("history", &AdapterState::from(self))
            .finish()
    }
}

impl DataAdapter<HistoryEntry> for HistoryAdapter {
    fn get_line(&mut self, i: usize, selected: bool) -> StyledLine<String> {
        if self.is_fill_up_needed(i) {
            assert!(self.fill_up(i + 50));
        }
        let entry = self.history.get_mut(i).unwrap();
        entry.render(selected)
    }

    fn get_data(&mut self, i: usize) -> &HistoryEntry {
        assert!(i < self.length);
        if self.is_fill_up_needed(i) {
            assert!(self.fill_up(self.history.len() - i + 50));
        }
        self.history.get(i).unwrap()
    }

    fn is_empty(&self) -> bool {
        self.length == 0
    }

    fn len(&self) -> usize {
        self.length
    }

    fn search(&mut self, needle: Needle, start: usize) -> Receiver<SearchProgress> {
        let range = self.range.clone();
        let paths = self.paths.clone();
        let repo = self.repo.clone();

        let (rx, tx) = mpsc::channel::<SearchProgress>();
        let thread = thread::spawn(move || {
            let commits = commits_for_range(&repo, &range, &paths, None, None);

            if !commits.is_empty() {
                Self::search_recursive(&needle, start, &rx, &commits, &[], &repo, &paths);
            }

            #[allow(unused_must_use)]
            {
                rx.send(SearchProgress::Finished);
            }
        });
        self.search_thread = Some(thread);
        tx
    }
}

#[derive(Eq, PartialEq)]
enum KeepGoing {
    Canceled,
    Chuckaway,
}

impl HistoryAdapter {
    fn search_recursive(
        needle: &Needle,
        start: usize,
        rx: &Sender<SearchProgress>,
        commits: &[Commit],
        search_path: &[usize],
        repo: &Repository,
        paths: &[String],
    ) -> KeepGoing {
        let mut seen = 0;
        let range = {
            let mut part1 = (start..commits.len()).collect::<Vec<usize>>();
            let part2 = (0..start).collect::<Vec<usize>>();
            part1.extend(part2);
            if *needle.direction() == Direction::Backward {
                part1 = part1.into_iter().rev().collect::<Vec<_>>();
            }
            part1
        };
        for i in range {
            let c = &commits[i];
            seen += 1;
            let mut r = search_path.to_vec();
            r.push(i);
            if c.matches(needle)
                && rx
                    .send(SearchProgress::Found(SearchResult(r.clone())))
                    .is_err()
            {
                return KeepGoing::Canceled;
            }
            if c.is_merge() {
                let tmp = child_history(repo, c, paths);
                let result = Self::search_recursive(needle, 0, rx, &tmp, &r, repo, paths);
                if result == KeepGoing::Canceled {
                    return result;
                }
            }
            if seen % 100 == 0 {
                // This should be fixed in the next clippy version (0.1.59?).
                // https://github.com/rust-lang/rust-clippy/issues/8269
                #[allow(clippy::question_mark)]
                if rx.send(SearchProgress::Searched(seen)).is_err() {
                    return KeepGoing::Canceled;
                }
                seen = 0;
            }
        }
        if 0 < seen && rx.send(SearchProgress::Searched(seen)).is_err() {
            return KeepGoing::Canceled;
        }
        KeepGoing::Chuckaway
    }
}

#[cfg(test)]
mod test {
    use crate::history_adapter::HistoryAdapter;
    use git_wrapper::Repository;

    #[test]
    #[should_panic]
    fn not_loaded_default_action() {
        let range = "6be11cb7f9e..df622aa0149";
        let repo = Repository::default().unwrap();
        let mut adapter = HistoryAdapter::new(repo, range, vec![]).unwrap();
        assert_eq!(adapter.history.len(), 0);
        adapter.default_action(8);
    }

    #[test]
    fn folding() {
        let range = "6be11cb7f9e..df622aa0149";
        let repo = Repository::default().unwrap();
        let mut adapter = HistoryAdapter::new(repo, range, vec![]).unwrap();
        assert_eq!(adapter.length, 9);
        adapter.fill_up(50);
        assert_eq!(adapter.history.len(), 9);
        adapter.default_action(8);
        assert_eq!(adapter.history.len(), 15);
        adapter.default_action(8);
        assert_eq!(adapter.history.len(), 9);
    }
}
