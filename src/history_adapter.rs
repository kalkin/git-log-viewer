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

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

use git_stree::{SubtreeConfig, Subtrees};
use posix_errors::PosixError;
use subject_classifier::Subject;
use url::Url;

use crate::actors::bitbucket::{BitbucketRequest, BitbucketThread};
use crate::actors::fork_point::ForkPointThread;
use crate::actors::github::{GitHubRequest, GitHubThread};
use crate::actors::subtrees::{SubtreeChangesRequest, SubtreeThread};
use crate::commit::{child_history, commits_for_range, history_length, Commit};
use crate::history_entry::{EntryKind, HistoryEntry};
use crate::ui::base::data::SearchProgress;
use crate::ui::base::search::{Direction, Needle, SearchResult};
use crate::ui::base::StyledLine;
use crate::utils::find_forge_url;
use git_wrapper::Remote;
use git_wrapper::Repository;
use std::fmt::{Debug, Formatter};
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;

pub struct HistoryAdapter {
    history: Vec<HistoryEntry>,
    length: usize,
    paths: Vec<PathBuf>,
    remotes: Vec<Remote>,
    range: Vec<OsString>,
    repo: Repository,
    forge_url: Option<Url>,
    github_thread: GitHubThread,
    bb_server_thread: BitbucketThread,
    fork_point_thread: ForkPointThread,
    subtree_modules: Vec<SubtreeConfig>,
    subtree_thread: SubtreeThread,
    search_thread: Option<JoinHandle<()>>,
    debug: bool,
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
#[cfg(not(tarpaulin_include))]
impl Debug for CommitRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut indent = "".to_owned();
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

#[cfg(not(tarpaulin_include))]
impl Debug for HistoryAdapter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result: Vec<CommitRange> = vec![];
        if !self.history.is_empty() {
            let mut start = RangePart {
                i: 0,
                id: self.history[0].short_id().clone(),
            };
            let mut level = 0;
            for (i, e) in self.history.iter().enumerate() {
                if level != e.level() {
                    let prev = &self.history[i - 1];
                    let end = RangePart {
                        i: i - 1,
                        id: prev.short_id().clone(),
                    };
                    let range = CommitRange {
                        start: start.clone(),
                        end,
                        level: level.try_into().expect("usize"),
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
                i: self.history.len() - 1,
                id: self.history.last().expect("smth").short_id().clone(),
            };
            let range = CommitRange {
                start,
                end,
                level: level.try_into().expect("usize"),
            };
            result.push(range);
        }
        f.debug_struct("HistoryAdapter")
            .field("range", &self.range)
            .field("length", &self.length)
            .field("loaded", &self.history.len())
            .field("paths", &self.paths)
            .field("history", &result)
            .finish()
    }
}

impl HistoryAdapter {
    ///
    /// # Errors
    ///
    /// Will return an error if git `working_dir` does not exist or git executable is missing
    pub fn new(
        repo: Repository,
        range: Vec<OsString>,
        paths: Vec<PathBuf>,
        debug: bool,
    ) -> Result<Self, PosixError> {
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
        log::debug!("Forge url {:?}", forge_url);

        let length = history_length(&repo, &range, &paths)?;
        if length == 0 {
            return Err(PosixError::new(1, "No commits found".to_owned()));
        }
        let subtrees = Subtrees::from_repo(repo.clone()).expect("Read subtree config");
        let subtree_modules = subtrees.all()?;
        let subtree_thread = SubtreeThread::new(subtrees);
        let bb_server_thread = BitbucketThread::new();
        let fork_point_thread = ForkPointThread::new(repo.clone());
        Ok(Self {
            history: vec![],
            length,
            paths,
            remotes,
            forge_url,
            bb_server_thread,
            range,
            repo,
            github_thread: GitHubThread::new(),
            fork_point_thread,
            subtree_modules,
            subtree_thread,
            search_thread: None,
            debug,
        })
    }

    pub fn unfold_up_to(&mut self, sr: &SearchResult) -> usize {
        debug_assert!(!sr.0.is_empty(), "Unexpected empty SearchResult vector");
        let addresses = &sr.0;
        let mut result = 0;
        let last_level = addresses.len();
        for (level, addr) in addresses.iter().enumerate() {
            result = self.addr_to_index(result, level, *addr);
            let entry = self.get_data(result);
            let entry_level: usize = entry.level().try_into().expect("usize");
            if last_level - 1 != (entry_level) {
                if entry_level != level {
                    log::error!("Failed unfold_up_to");
                    return 0;
                }
                assert_eq!(entry_level, level);
                result += 1;
                if entry.is_foldable() {
                    if entry.is_folded() {
                        self.toggle_folding(result - 1);
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
        let self_level: usize = self
            .get_data(start_index)
            .level()
            .try_into()
            .expect("usize");
        assert_eq!(self_level, level);
        let mut result: usize = 0;
        let mut stop: usize = 0;
        #[allow(clippy::arithmetic)]
        // arithmetic: `stop` is always <= `i` <= `usize::MAX`
        for i in start_index..self.length {
            let entry = self.get_data(i);
            let cur_level: usize = entry.level().try_into().expect("usize");
            if cur_level == level {
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
        let tmp = commits_for_range(
            &self.repo,
            &self.range,
            self.paths.as_ref(),
            Some(skip),
            Some(max),
        );
        if tmp.is_empty() {
            return false;
        }
        let mut above_entry = self.history.last();
        let mut tmp2 = Vec::with_capacity(tmp.len());
        let level = 0;
        for commit in tmp {
            let entry = self.to_entry(commit, above_entry, level, false);
            tmp2.push(entry);
            above_entry = tmp2.last();
        }
        self.history.append(tmp2.as_mut());
        true
    }

    fn to_entry(
        &self,
        commit: Commit,
        above_entry: Option<&HistoryEntry>,
        level: u8,
        link: bool,
    ) -> HistoryEntry {
        let above_commit = above_entry.map(HistoryEntry::commit);
        let kind = EntryKind::new(&commit, above_commit.is_some(), link);

        if !self.subtree_modules.is_empty() {
            self.subtree_thread
                .send(SubtreeChangesRequest {
                    oid: commit.id().clone(),
                })
                .unwrap();
        }
        let fork_point = self
            .fork_point_thread
            .request_calculation(&commit, above_commit);

        let mut entry = HistoryEntry::new(
            commit,
            level,
            self.forge_url.clone(),
            fork_point,
            &self.remotes,
            kind,
            self.debug,
        );

        if let Some(url) = entry.url() {
            if let Subject::PullRequest { id, .. } = entry.special() {
                if GitHubThread::can_handle(&url) {
                    if let Some(title) = GitHubThread::from_cache(&url, id) {
                        log::debug!("PR #{} (CACHE) ⇒ «{}»", id, title);
                        entry.set_subject(&title);
                    } else {
                        let req = GitHubRequest {
                            oid: entry.id().clone(),
                            url,
                            pr_id: id.to_string(),
                        };
                        if let Err(err) = self.github_thread.send(req) {
                            log::error!("{}", err);
                        }
                    }
                } else if BitbucketThread::can_handle(&url) {
                    if let Some(title) = BitbucketThread::from_cache(&url, id) {
                        log::debug!("PR #{} (CACHE) ⇒ «{}»", id, title);
                        entry.set_subject(&title);
                    } else {
                        let req = BitbucketRequest {
                            oid: entry.id().clone(),
                            url,
                            pr_id: id.to_string(),
                        };

                        if let Err(err) = self.bb_server_thread.send(req) {
                            log::error!("{}", err);
                        }
                    }
                } else {
                    log::info!("Unrecognized url {}", url);
                }
            }
        }
        entry
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
        let selected = &self.history[i];
        if selected.is_folded() {
            let children: Vec<Commit> =
                child_history(&self.repo, selected.commit(), self.paths.as_ref());
            log::debug!("Unfolding entry {}, with #{} children", i, children.len());

            // Check if we need to add a Link commit
            let link_commit = if let (Some(Some(oid)), Some(bellow_selected)) = (
                children.last().map(|c| c.parents().first()),
                selected.commit().parents().first(),
            ) {
                if oid == bellow_selected {
                    None
                } else {
                    Commit::from_repo(&self.repo, oid)
                }
            } else {
                None
            };

            let mut tmp: Vec<HistoryEntry> = vec![];
            {
                let level = selected.level() + 1;
                let mut above_entry = Some(selected);
                for t in children {
                    let entry = self.to_entry(t, above_entry, level, false);
                    tmp.push(entry);
                    above_entry = tmp.last();
                }
                if let Some(link) = link_commit {
                    tmp.push(self.to_entry(link, above_entry, level, true));
                }
            }

            self.history[i].set_visible_children(tmp.len());
            for (j, entry) in tmp.into_iter().enumerate() {
                log::trace!(
                    "Inserting index {}, entry {:?}",
                    j,
                    self.history[j].commit().subject()
                );
                self.history.insert(pos + j, entry);
                self.length += 1;
            }
        } else {
            let f = selected.visible_children();
            log::debug!("Folding entry {}, with #{} children", i, f);
            for j in (pos..(pos + f)).rev() {
                log::trace!(
                    "Removing index {}: {:?}",
                    j,
                    self.history[j].commit().subject()
                );
                if !self.history[j].is_folded() {
                    self.toggle_folding(j);
                }
                self.history.remove(j);
            }
            self.history[i].set_visible_children(0);
        }
    }

    /// Run this function before accessing data, to update data calculated by other threads
    pub fn update(&mut self) {
        while let Ok(v) = self.fork_point_thread.try_recv() {
            for e in &mut self.history {
                if e.id() == &v.first {
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
                    e.set_subject(&v.subject);
                    break;
                }
            }
        }

        while let Ok(v) = self.bb_server_thread.try_recv() {
            for e in &mut self.history {
                if e.id() == &v.oid {
                    e.set_subject(&v.subject);
                    break;
                }
            }
        }
    }
    pub fn get_line(&mut self, i: usize, selected: bool) -> StyledLine<String> {
        if self.is_fill_up_needed(i) {
            assert!(self.fill_up(i + 50));
        }
        self.history[i].render(selected)
    }

    pub fn get_data(&mut self, i: usize) -> &HistoryEntry {
        debug_assert!(i < self.length);
        if self.is_fill_up_needed(i) {
            assert!(self.fill_up(self.history.len() - i + 50));
        }
        &self.history[i]
    }

    pub const fn len(&self) -> usize {
        self.length
    }

    pub fn search(&mut self, needle: Needle, start: usize) -> Receiver<SearchProgress> {
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
        paths: &[PathBuf],
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
            #[allow(clippy::arithmetic)]
            {
                // arithmetic: `seen` can never exceed `usize::MAX`, because `seen <= range.len()`
                seen += 1;
            }
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
            // std::ops::Rem is safe
            #[allow(clippy::arithmetic)]
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
    use std::ffi::OsString;

    use crate::history_adapter::HistoryAdapter;
    use git_wrapper::Repository;
    use pretty_assertions::assert_eq;

    #[test]
    #[should_panic]
    fn not_loaded_default_action() {
        let range = vec![OsString::from("6be11cb7f9e..df622aa0149")];
        let repo = Repository::default().unwrap();
        let mut adapter = HistoryAdapter::new(repo, range, vec![], false).unwrap();
        assert_eq!(adapter.history.len(), 0);
        adapter.default_action(8);
    }

    #[test]
    fn folding() {
        let range = vec![OsString::from("6be11cb7f9e..df622aa0149")];
        let repo = Repository::default().unwrap();
        let mut adapter = HistoryAdapter::new(repo, range, vec![], false).unwrap();
        assert_eq!(adapter.length, 9);
        adapter.fill_up(50);
        assert_eq!(adapter.history.len(), 9);
        adapter.default_action(8);
        assert_eq!(adapter.history.len(), 15);
        adapter.default_action(8);
        assert_eq!(adapter.history.len(), 9);
    }
}
