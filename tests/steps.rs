use cucumber_rust::{t, Steps};

use glv::commit::{commits_for_range, Commit, GitRef};

use crate::Url;

pub fn steps() -> Steps<crate::MyWorld> {
    let mut steps: Steps<crate::MyWorld> = Steps::new();
    steps.given_regex_async(
        r#"repo url (.+)$"#,
        t!(|mut world, ctx| {
            assert_eq!(ctx.matches.len(), 2);
            let url = ctx.matches[1].clone();
            world.url = Url(url);
            let working_dir = world.working_dir.path().to_str().unwrap();
            assert!(git_wrapper::clone(&world.url.to_string(), working_dir).unwrap());
            return world;
        }),
    );

    steps.given_regex_async(
        r#"^commit (.+)$"#,
        t!(|mut world, ctx| {
            assert_eq!(ctx.matches.len(), 2);
            let id = ctx.matches[1].clone();
            let working_dir = world.working_dir.path().to_str().unwrap();
            let range = format!("{}~1..{}", id, id);
            let paths: Vec<String> = vec![];
            let commits = commits_for_range(working_dir, &range, &paths, None, None).unwrap();
            world.commit = commits.into_iter().next();
            world
        }),
    );

    steps.given_regex(r#"^range (.+)$"#, |mut world, ctx| {
        assert_eq!(ctx.matches.len(), 2);
        let range = ctx.matches[1].clone();
        let working_dir = world.working_dir.path().to_str().unwrap();
        let paths: Vec<String> = vec![];
        world.range = Some(commits_for_range(working_dir, &range, &paths, None, None).unwrap());
        world
    });

    steps.then_regex_async(
        r#"^commit has reference “(.+)”$"#,
        t!(|world, ctx| {
            let commit: &Commit = world.commit.as_ref().unwrap();
            let expected = GitRef(ctx.matches[1].clone());
            if !commit.references().iter().any(|r| r == &expected) {
                assert!(false, "Failed to find reference {}", expected)
            }
            return world;
        }),
    );

    steps.then_regex_async(
        r#"^commit has branch “(.+)”$"#,
        t!(|world, ctx| {
            let commit: &Commit = world.commit.as_ref().unwrap();
            let expected = GitRef(ctx.matches[1].clone());
            if !commit.branches().iter().any(|r| r == &expected) {
                assert!(false, "Failed to find branch {}", expected)
            }
            return world;
        }),
    );

    steps.then_regex_async(
        r#"^commit is head$"#,
        t!(|world, _ctx| {
            let commit: &Commit = world.commit.as_ref().unwrap();
            assert!(commit.is_head(), "Commit should be HEAD");
            return world;
        }),
    );

    steps.when_regex_async(
        r#"^(bellow|above) commit is (.+)$"#,
        t!(|world, ctx| {
            let commit: &Commit = world.commit.as_ref().unwrap();
            let direction = &ctx.matches[1].clone();
            let expected = &ctx.matches[2].clone();
            match direction.as_str() {
                "bellow" => {
                    assert!(
                        commit.bellow().is_some(),
                        "Expected commit {} to have bellow commit",
                        commit.short_id()
                    );
                    assert!(commit.bellow().unwrap().to_string().starts_with(expected));
                }
                _ => {
                    panic!("Unexpected pattern {}", direction)
                }
            }
            world
        }),
    );

    steps.then_regex_async(
        r#"^commit (author|committer) (name|email|date) is “(.+)”$"#,
        t!(|world, ctx| {
            let commit: &Commit = world.commit.as_ref().unwrap();
            let user = &ctx.matches[1];
            let field = &ctx.matches[2];
            let expected = &ctx.matches[3];
            let actual;
            match (user.as_str(), field.as_str()) {
                ("author", "name") => actual = commit.author_name(),
                ("author", "email") => actual = commit.author_email(),
                ("author", "date") => actual = commit.author_date(),
                ("committer", "name") => actual = commit.committer_name(),
                ("committer", "email") => actual = commit.committer_email(),
                ("committer", "date") => actual = commit.committer_date(),
                (_, _) => panic!("Unexpected {} / {} combination", user, field),
            }
            assert_eq!(expected, actual);
            world
        }),
    );

    steps.then_regex_async(
        r#"^commit subject is “(.+)”$"#,
        t!(|world, ctx| {
            let commit: &Commit = world.commit.as_ref().unwrap();
            let expected = ctx.matches[1].as_str();
            let actual = commit.subject();
            assert_eq!(expected, actual);
            world
        }),
    );

    steps.then_regex_async(
        r#"^commit has (\d+) child commit$"#,
        t!(|world, ctx| {
            let commit: &Commit = world.commit.as_ref().unwrap();
            let digits = ctx.matches[1].clone();
            let expected: usize = digits.parse().unwrap();
            let actual = commit.children().len();
            assert_eq!(expected, actual);
            world
        }),
    );

    steps.then_regex(r#"^I should have (\d+) commits$"#, |world, ctx| {
        assert!(world.range.is_some());
        let digits = &ctx.matches[1];
        let expected: usize = digits.parse().unwrap();
        let actual = world.range.as_ref().unwrap().len();
        assert_eq!(expected, actual);
        world
    });

    return steps;
}
