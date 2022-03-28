//! Add commit id & dirty flag to `CARGO_PKG_VERSION`
use std::process::Command;

fn head_path() -> String {
    let output = Command::new("git")
        .args(&["rev-parse", "--git-dir"])
        .output()
        .expect("Got $GIT_DIR");
    let git_dir = String::from_utf8_lossy(&output.stdout);
    return format!("{}/HEAD", git_dir);
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed={}", head_path());

    if let Ok(status) = Command::new("git")
        .args(&["diff-index", "--quiet", "HEAD", "--"])
        .status()
    {
        let changed_since_release = {
            let id = {
                let out = Command::new("git")
                    .args(&["rev-list", "-1", "--", "CHANGELOG.md"])
                    .output()
                    .expect("A comitted CHANGELOG.md");
                String::from_utf8_lossy(&out.stdout).to_string()
            };
            let range = format!("{}..HEAD", id.trim());
            let out = Command::new("git")
                .args(&["rev-list", "--count", &range, "--", "."])
                .output()
                .expect("git rev-list successful");
            String::from_utf8_lossy(&out.stdout).to_string().trim() != "0"
        };

        let cargo_version = env!("CARGO_PKG_VERSION");
        let version = match (changed_since_release, status.code().unwrap_or(1) != 0) {
            (false, false) => cargo_version.to_owned(),
            (false, true) => format!("{}+dirty", cargo_version),
            (true, dirty) => {
                let id_out = Command::new("git")
                    .args(&["rev-parse", "--short", "HEAD"])
                    .output()
                    .expect("Executed git-rev-parse(1)");
                let id = String::from_utf8_lossy(&id_out.stdout).to_string();
                if dirty {
                    format!("{}+{}.dirty", cargo_version, id.trim())
                } else {
                    format!("{}+{}.", cargo_version, id.trim())
                }
            }
        };
        println!("cargo:rustc-env=CARGO_PKG_VERSION={}", version);
    }
}
