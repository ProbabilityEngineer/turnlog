use crate::model::VcsInfo;
use std::path::{Path, PathBuf};
use std::process::Command;

fn run(cwd: &Path, cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn repo_root(cwd: &Path) -> PathBuf {
    if let Some(root) = run(cwd, "jj", &["root"]).filter(|s| !s.is_empty()) {
        return PathBuf::from(root);
    }
    if let Some(root) = run(cwd, "git", &["rev-parse", "--show-toplevel"]).filter(|s| !s.is_empty())
    {
        return PathBuf::from(root);
    }
    cwd.to_path_buf()
}

pub fn detect(cwd: &Path) -> VcsInfo {
    if run(cwd, "jj", &["root"]).is_some() {
        return detect_jj(cwd);
    }
    if run(cwd, "git", &["rev-parse", "--show-toplevel"]).is_some() {
        return detect_git(cwd);
    }
    VcsInfo::None
}

pub fn diff(cwd: &Path) -> Option<String> {
    if run(cwd, "jj", &["root"]).is_some() {
        return run(cwd, "jj", &["diff", "--git"]);
    }
    if run(cwd, "git", &["rev-parse", "--show-toplevel"]).is_some() {
        return run(cwd, "git", &["diff"]);
    }
    None
}

fn detect_jj(cwd: &Path) -> VcsInfo {
    let jj_change = run(
        cwd,
        "jj",
        &["log", "-r", "@", "--no-graph", "-T", "change_id.short()"],
    );
    let jj_commit = run(
        cwd,
        "jj",
        &["log", "-r", "@", "--no-graph", "-T", "commit_id.short()"],
    );
    let jj_operation = run(
        cwd,
        "jj",
        &[
            "op",
            "log",
            "--limit",
            "1",
            "--no-graph",
            "-T",
            "id.short()",
        ],
    );
    let git_head = run(cwd, "git", &["rev-parse", "HEAD"]);
    let git_branch = run(cwd, "git", &["branch", "--show-current"]).filter(|s| !s.is_empty());
    let changed_files = jj_changed_files(cwd);
    let dirty = !changed_files.is_empty();
    VcsInfo::Jj {
        jj_change,
        jj_commit,
        jj_operation,
        git_head,
        git_branch,
        dirty,
        changed_files,
    }
}

fn detect_git(cwd: &Path) -> VcsInfo {
    let git_head = run(cwd, "git", &["rev-parse", "HEAD"]);
    let git_branch = run(cwd, "git", &["branch", "--show-current"]).filter(|s| !s.is_empty());
    let changed_files = git_changed_files(cwd);
    let dirty = !changed_files.is_empty();
    VcsInfo::Git {
        git_head,
        git_branch,
        dirty,
        changed_files,
    }
}

fn jj_changed_files(cwd: &Path) -> Vec<String> {
    let Some(raw) = run(cwd, "jj", &["status", "--no-pager"]) else {
        return vec![];
    };
    raw.lines()
        .filter(|line| {
            matches!(
                line.as_bytes().first(),
                Some(b'A' | b'M' | b'D' | b'R' | b'C')
            )
        })
        .filter_map(|line| line.get(2..).map(str::trim))
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn git_changed_files(cwd: &Path) -> Vec<String> {
    let Some(raw) = run(cwd, "git", &["status", "--porcelain"]) else {
        return vec![];
    };
    raw.lines()
        .filter_map(|line| line.get(3..).map(str::trim))
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
