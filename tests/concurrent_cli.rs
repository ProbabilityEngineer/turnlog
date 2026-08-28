use std::fs;
use std::process::{Command, Stdio};

fn turnlog() -> Command {
    Command::new(env!("CARGO_BIN_EXE_turnlog"))
}

fn run(cwd: &std::path::Path, args: &[&str]) {
    let output = turnlog().current_dir(cwd).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "turnlog {} failed:\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn concurrent_record_processes_leave_a_complete_unique_index() {
    let repo = tempfile::tempdir().unwrap();
    run(repo.path(), &["init"]);
    run(repo.path(), &["start", "--goal", "concurrency test"]);

    let mut children = Vec::new();
    for number in 0..24 {
        children.push(
            turnlog()
                .current_dir(repo.path())
                .args(["record", "--summary", &format!("record {number}")])
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
    }
    for child in children {
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "concurrent record failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let index = fs::read_to_string(repo.path().join(".turnlog/index.jsonl")).unwrap();
    let entries: Vec<serde_json::Value> = index
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(entries.len(), 25, "one session plus 24 turn records");

    let ids: std::collections::HashSet<_> = entries
        .iter()
        .filter_map(|event| match event["type"].as_str() {
            Some("session_started") => event["session"]["id"].as_str(),
            Some("turn_recorded") => event["turn"]["id"].as_str(),
            _ => None,
        })
        .collect();
    assert_eq!(ids.len(), 25);
}
