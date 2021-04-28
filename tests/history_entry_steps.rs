use cucumber_rust::{t, Steps};
use glv::commit::{commits_for_range, Commit};
use glv::fork_point::{ForkPointCalculation, ForkPointThread};
use glv::history_entry::HistoryEntry;

pub fn steps() -> Steps<crate::MyWorld> {
    let mut steps: Steps<crate::MyWorld> = Steps::new();

    steps.given_regex_async(
        r#"^history entry for commit (.+)$"#,
        t!(|mut world, ctx| {
            assert_eq!(ctx.matches.len(), 2);
            let id = ctx.matches[1].clone();
            let working_dir = world.working_dir.path().to_str().unwrap();
            let range = format!("{}~1..{}", id, id);
            let paths: Vec<String> = vec![];
            let commits = commits_for_range(working_dir, &range, &paths, None, None).unwrap();
            let commit = commits.into_iter().next().unwrap();
            world.entry = Some(HistoryEntry::new(
                commit,
                0,
                None,
                ForkPointCalculation::Done(false),
            ));
            world
        }),
    );

    steps.given_regex_async(
        r#"^history entries for range (.+)$"#,
        t!(|mut world, ctx| {
            let range = ctx.matches[1].clone();
            let working_dir = world.working_dir.path().to_str().unwrap();
            let paths: Vec<String> = vec![];
            let commits = commits_for_range(working_dir, &range, &paths, None, None).unwrap();
            let mut result: Vec<HistoryEntry> = vec![];
            for c in commits.into_iter() {
                result.push(HistoryEntry::new(
                    c,
                    0,
                    None,
                    ForkPointCalculation::Done(false),
                ))
            }
            assert!(!result.is_empty());
            world.entries = Some(result);
            world
        }),
    );

    steps.when_regex_async(
        r#"^fork point calculation done$"#,
        t!(|mut world, ctx| {
            assert!(world.entries.is_some());
            let working_dir = world.working_dir.path().to_str().unwrap();
            if let Some(entries) = world.entries.as_mut() {
                let mut above_commit: Option<Commit> = None;
                for e in entries.into_iter() {
                    let t = if above_commit.is_none() {
                        false
                    } else {
                        let above = above_commit.unwrap();
                        let second = above.children().first().unwrap();
                        ForkPointThread::is_fork_point(working_dir, &e.id(), &second)
                    };
                    above_commit = Some(e.commit().clone());
                    e.set_fork_point(t);
                }
            }
            return world;
        }),
    );

    steps.then_regex_async(
        r#"^entry is not a merge$"#,
        t!(|world, ctx| {
            match &world.entry {
                Some(e) => {
                    assert_eq!(e.is_merge(), false, "Not a merge");
                }
                None => {
                    panic!("No history entry found");
                }
            }
            world
        }),
    );
    steps.then_regex_async(
        r#"^entry with index (\d+) is a fork point$"#,
        t!(|world, ctx| {
            assert_eq!(ctx.matches.len(), 2);
            let index: usize = ctx.matches[1].clone().parse::<usize>().unwrap();
            match &world.entries {
                Some(entries) => {
                    let e = entries.get(index).unwrap();
                    assert!(e.is_fork_point(), "Expected fork point");
                }
                None => {
                    panic!("Expected history entries");
                }
            }
            world
        }),
    );

    steps
}
