//! Embed version from git tag (release) or fall back to Cargo.toml

fn main() {
    if let Ok(output) = std::process::Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .output()
    {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !version.is_empty() {
                println!("cargo:rustc-env=RATLOG_VERSION={}", version);
            }
        }
    }
}
