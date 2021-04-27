use cucumber_rust::{t, Steps};
use glv::commit::commits_for_range;
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
            world.entry = Some(HistoryEntry::new(commit, 0, None));
            world
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

    steps
}
