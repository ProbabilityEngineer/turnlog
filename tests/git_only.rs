#![cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn git_default_and_override_never_execute_jj() {
    for mode in [None, Some("git"), Some("invalid")] {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        let bin = temp.path().join("bin");
        fs::create_dir_all(repo.join(".jj")).unwrap();
        fs::create_dir(&bin).unwrap();
        let marker = temp.path().join("jj-called");
        let stub = bin.join("jj");
        fs::write(&stub, "#!/bin/sh\ntouch \"$JJ_MARKER\"\nexit 99\n").unwrap();
        fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(
            Command::new("git")
                .args(["init", "-q"])
                .current_dir(&repo)
                .status()
                .unwrap()
                .success()
        );
        let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
        for args in [
            vec!["init"],
            vec!["start", "--goal", "git safety"],
            vec!["record", "--attach-diff", "--summary", "git only"],
            vec!["status"],
        ] {
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_turnlog"));
            cmd.current_dir(&repo)
                .args(args)
                .env("PATH", &path)
                .env("JJ_MARKER", &marker)
                .env_remove("TURNLOG_VCS");
            if let Some(value) = mode {
                cmd.env("TURNLOG_VCS", value);
            }
            let out = cmd.output().unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        assert!(!marker.exists(), "JJ executed for mode {mode:?}");
        let index = fs::read_to_string(repo.join(".turnlog/index.jsonl")).unwrap();
        assert!(index.contains("\"git\""), "{index}");
    }
}
