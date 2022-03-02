//! Add commit id & dirty flag to `CARGO_PKG_VERSION`
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    if let Ok(status) = Command::new("git")
        .args(&["diff-index", "--quiet", "HEAD", "--"])
        .status()
    {
        let id_out = Command::new("git")
            .args(&["rev-parse", "--short", "HEAD"])
            .output()
            .unwrap();
        let id = String::from_utf8_lossy(&id_out.stdout).to_string();
        let cargo_version = env!("CARGO_PKG_VERSION");
        let version = if status.code().unwrap() == 0 {
            format!("{}+{}", cargo_version, id.trim())
        } else {
            format!("{}+{}.dirty", cargo_version, id.trim())
        };
        println!("cargo:rustc-env=CARGO_PKG_VERSION={}", version);
    }
}
