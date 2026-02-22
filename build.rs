fn main() {
    println!("cargo:rerun-if-changed=.");
    let git_describe =
        get_git_descfribe().unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=GIT_DESCRIBE={}", git_describe);
}

fn get_git_descfribe() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["describe", "--tags", "--always", "--long", "--dirty"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}
