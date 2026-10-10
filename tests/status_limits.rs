use std::{fs, path::Path, process::Command};

fn cli(dir: &Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_turnlog"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn status_empty_ordinary_large_and_full_preserves_records() {
    let repo = tempfile::tempdir().unwrap();
    let dir = repo.path();
    assert!(cli(dir, &["status"]).contains("not initialized"));
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(dir)
            .status()
            .unwrap()
            .success()
    );
    cli(dir, &["init"]);
    assert!(
        Command::new("git")
            .args(["add", ".gitignore"])
            .current_dir(dir)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-qm",
                "fixture"
            ])
            .current_dir(dir)
            .status()
            .unwrap()
            .success()
    );
    cli(dir, &["start", "--goal", &"研究\n".repeat(3000)]);
    cli(dir, &["record", "--summary", &"😀\n".repeat(5000)]);
    let index = fs::read(dir.join(".turnlog/index.jsonl")).unwrap();
    let turns: Vec<_> = fs::read_dir(dir.join(".turnlog/turns"))
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            (path.clone(), fs::read(path).unwrap())
        })
        .collect();
    let ordinary = cli(dir, &["status"]);
    assert!(ordinary.len() <= 12 * 1024);
    assert!(ordinary.lines().count() <= 40);
    fs::create_dir(dir.join("fixtures")).unwrap();
    for number in 0..30_001 {
        fs::write(dir.join(format!("path-{number:05}")), "").unwrap();
    }
    let output = cli(dir, &["status"]);
    assert!(output.len() <= 12 * 1024);
    assert!(output.lines().count() <= 40);
    assert!(output.contains("changed files: 30001"));
    assert!(output.contains("omitted paths: 29981"));
    assert_eq!(
        output
            .lines()
            .filter(|line| line.starts_with("  - "))
            .count(),
        20
    );
    let full = cli(dir, &["status", "--full"]);
    assert!(full.contains("path-30000"));
    assert_eq!(
        full.lines().filter(|line| line.starts_with("  - ")).count(),
        30_001
    );
    assert_eq!(fs::read(dir.join(".turnlog/index.jsonl")).unwrap(), index);
    for (path, bytes) in turns {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}
