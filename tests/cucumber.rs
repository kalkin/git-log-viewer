use std::convert::Infallible;

use cucumber_rust::{async_trait, Context, Cucumber, World};

use glv::commit::Commit;
use glv::history_entry::HistoryEntry;

mod history_entry_steps;
mod steps;

#[derive(derive_more::Display, derive_more::FromStr)]
#[display(fmt = "{}", self.0)]
pub struct Url(String);

pub struct MyWorld {
    url: Url,
    working_dir: tempfile::TempDir,
    commit: Option<Commit>,
    entry: Option<HistoryEntry>,
    range: Option<Vec<Commit>>,
}

#[async_trait(? Send)]
impl World for MyWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        return Ok(MyWorld {
            url: Url(String::new()),
            working_dir: tempfile::tempdir().unwrap(),
            commit: None,
            entry: None,
            range: None,
        });
    }
}

#[tokio::main]
async fn main() {
    Cucumber::<MyWorld>::new()
        // Specifies where our feature files exist
        .features(&["features"])
        // Adds the implementation of our steps to the runner
        .steps(steps::steps())
        .steps(history_entry_steps::steps())
        .context(Context::new())
        // Parses the command line arguments if passed
        .cli()
        // Runs the Cucumber tests and then exists
        .run_and_exit()
        .await
}
