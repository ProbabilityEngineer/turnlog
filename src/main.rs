mod cli;
mod ids;
mod model;
mod store;
mod vcs;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use model::{Attachment, AttachmentKind, Event, SCHEMA_VERSION, Session, Turn};
use store::Store;
use time::OffsetDateTime;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cwd = match cli.cwd {
        Some(path) if path.is_absolute() => path,
        Some(path) => std::env::current_dir()?.join(path),
        None => std::env::current_dir()?,
    };
    match cli.command {
        Command::Init => {
            let root = vcs::repo_root(&cwd);
            let store = Store::at_repo_root(&root);
            store.init()?;
            println!("initialized {}", root.join(".turnlog").display());
        }
        Command::Start { ticket, goal } => {
            let root = vcs::repo_root(&cwd);
            let store = Store::at_repo_root(&root);
            store.init()?;
            let session = Session {
                schema_version: SCHEMA_VERSION,
                id: ids::session_id(),
                ticket,
                goal,
                created_at: OffsetDateTime::now_utc(),
                repo_root: root.display().to_string(),
                vcs_start: vcs::detect(&cwd),
            };
            store.write_session(&session)?;
            store.set_current_session(&session.id)?;
            println!("started {}", session.id);
        }
        Command::Record {
            session,
            model,
            summary,
            verification,
            attach_diff,
        } => {
            let store = Store::discover(&cwd)?;
            let session = match session {
                Some(id) => store
                    .session_by_id(&id)?
                    .ok_or_else(|| anyhow::anyhow!("session not found: {id}"))?,
                None => store.current_session()?,
            };
            let turn_id = ids::turn_id();
            let mut attachments = Vec::new();
            if attach_diff {
                match vcs::diff(&cwd) {
                    Some(diff) if !diff.is_empty() => {
                        let path = format!(".turnlog/attachments/{turn_id}.diff");
                        store.write_attachment(&path, &diff)?;
                        attachments.push(Attachment {
                            kind: AttachmentKind::Diff,
                            path,
                        });
                    }
                    Some(_) => eprintln!("no diff to attach"),
                    None => eprintln!("no VCS diff available; skipping diff attachment"),
                }
            }
            let turn = Turn {
                schema_version: SCHEMA_VERSION,
                id: turn_id,
                session: session.id,
                created_at: OffsetDateTime::now_utc(),
                model,
                summary,
                verification,
                attachments,
                vcs: vcs::detect(&cwd),
            };
            store.write_turn(&turn)?;
            println!("recorded {}", turn.id);
        }
        Command::Current => {
            let store = Store::discover(&cwd)?;
            let session = store.current_session()?;
            println!(
                "current session: {} ticket={} goal={}",
                session.id,
                session.ticket.as_deref().unwrap_or("none"),
                session.goal
            );
        }
        Command::Use { id } => {
            let store = Store::discover(&cwd)?;
            let session = store
                .session_by_id(&id)?
                .ok_or_else(|| anyhow::anyhow!("session not found: {id}"))?;
            store.set_current_session(&session.id)?;
            println!("current session: {} {}", session.id, session.goal);
        }
        Command::Status { full } => {
            print_status(&cwd, full)?;
        }
        Command::Log {
            session,
            ticket,
            grep,
            changed,
        } => {
            let store = Store::discover(&cwd)?;
            let events = store.events()?;
            let ticket_sessions: std::collections::HashSet<String> = ticket
                .as_deref()
                .map(|ticket| {
                    events
                        .iter()
                        .filter_map(|event| match event {
                            Event::SessionStarted { session }
                                if session.ticket.as_deref() == Some(ticket) =>
                            {
                                Some(session.id.clone())
                            }
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default();
            for event in events {
                if !event_matches(
                    &event,
                    session.as_deref(),
                    ticket.as_deref(),
                    &ticket_sessions,
                    grep.as_deref(),
                    changed.as_deref(),
                )? {
                    continue;
                }
                print_event_summary(&event);
            }
        }
        Command::Show { id, json } => {
            let store = Store::discover(&cwd)?;
            match store.find(&id)? {
                Some(event) if json => println!("{}", serde_json::to_string_pretty(&event)?),
                Some(Event::SessionStarted { session }) => {
                    println!("{}", store.render_session_rollup(&session)?);
                }
                Some(Event::TurnRecorded { turn }) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&Event::TurnRecorded { turn })?
                    );
                }
                None => anyhow::bail!("not found: {id}"),
            }
        }
        Command::Grep { pattern } => {
            let store = Store::discover(&cwd)?;
            let needle = pattern.to_lowercase();
            for event in store.events()? {
                let json = serde_json::to_string(&event)?;
                if json.to_lowercase().contains(&needle) {
                    print_event_summary(&event);
                }
            }
        }
        Command::Repair => {
            let store = Store::discover(&cwd)?;
            let (rebuilt, skipped) = store.repair_index()?;
            println!("repaired index: {rebuilt} events");
            for path in skipped {
                eprintln!("skipped invalid canonical file: {}", path.display());
            }
        }
        Command::Report { id, stdout } => {
            let store = Store::discover(&cwd)?;
            let session = store
                .session_by_id(&id)?
                .ok_or_else(|| anyhow::anyhow!("session not found: {id}"))?;
            if stdout {
                println!("{}", store.render_session_rollup(&session)?);
            } else {
                let path = store.write_session_report(&session)?;
                println!("wrote {}", path.display());
            }
        }
    }
    Ok(())
}

fn vcs_kind(v: &model::VcsInfo) -> &'static str {
    match v {
        model::VcsInfo::Jj { .. } => "jj",
        model::VcsInfo::Git { .. } => "git",
        model::VcsInfo::None => "none",
    }
}

const STATUS_MAX_BYTES: usize = 12 * 1024;
const STATUS_SAMPLE_LIMIT: usize = 20;

fn truncate_utf8(value: &str, max_bytes: usize) -> (&str, bool) {
    if value.len() <= max_bytes {
        return (value, false);
    }
    let mut end = max_bytes.saturating_sub("…".len());
    while !value.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    (&value[..end], true)
}

fn bounded_field(value: &str, max_bytes: usize) -> String {
    let (prefix, truncated) = truncate_utf8(value, max_bytes);
    if truncated {
        format!("{prefix}…")
    } else {
        prefix.to_string()
    }
}

fn print_status(cwd: &std::path::Path, full: bool) -> Result<()> {
    if full {
        return print_status_full(cwd);
    }

    let mut lines = Vec::new();
    let mut push = |label: &str, value: String| {
        lines.push(format!("{label}{value}"));
    };
    let store = Store::discover(cwd).ok();
    push(
        "turnlog: ",
        if store.is_some() {
            "initialized".into()
        } else {
            "not initialized".into()
        },
    );

    if let Some(store) = &store {
        match store.current_session() {
            Ok(session) => push(
                "current session: ",
                format!(
                    "{} ticket={} goal={}",
                    session.id,
                    session.ticket.as_deref().unwrap_or("none"),
                    bounded_field(&session.goal, 256)
                ),
            ),
            Err(_) => push("current session: ", "none".into()),
        }
        match store.latest_turn()? {
            Some(turn) => push(
                "last turn: ",
                format!(
                    "{} session={} summary={}",
                    turn.id,
                    turn.session,
                    bounded_field(turn.summary.as_deref().unwrap_or(""), 256)
                ),
            ),
            None => push("last turn: ", "none".into()),
        }
    }

    let vcs = vcs::detect(cwd);
    match &vcs {
        model::VcsInfo::Jj {
            jj_change,
            jj_commit,
            git_head,
            git_branch,
            dirty,
            changed_files,
            ..
        } => {
            push("vcs: ", "jj".into());
            push(
                "jj change: ",
                jj_change.as_deref().unwrap_or("unknown").into(),
            );
            push(
                "jj commit: ",
                jj_commit.as_deref().unwrap_or("unknown").into(),
            );
            push(
                "git branch: ",
                git_branch.as_deref().unwrap_or("none").into(),
            );
            push("git head: ", git_head.as_deref().unwrap_or("none").into());
            push("dirty: ", dirty.to_string());
            append_changed_summary(&mut lines, changed_files);
        }
        model::VcsInfo::Git {
            git_head,
            git_branch,
            dirty,
            changed_files,
        } => {
            push("vcs: ", "git".into());
            push(
                "git branch: ",
                git_branch.as_deref().unwrap_or("none").into(),
            );
            push("git head: ", git_head.as_deref().unwrap_or("none").into());
            push("dirty: ", dirty.to_string());
            append_changed_summary(&mut lines, changed_files);
        }
        model::VcsInfo::None => push("vcs: ", "none".into()),
    }

    let mut output = String::new();
    let mut truncated = false;
    for line in lines {
        truncated |= append_bounded_line(&mut output, &line);
    }
    if truncated {
        let notice = "\nstatus output truncated; use `turnlog status --full` for full output\n";
        let keep = STATUS_MAX_BYTES.saturating_sub(notice.len());
        output.truncate(keep);
        while !output.is_char_boundary(output.len()) {
            output.pop();
        }
        output.push_str(notice);
    }
    print!("{output}");
    Ok(())
}

fn append_changed_summary(lines: &mut Vec<String>, changed_files: &[String]) {
    lines.push(format!("changed files: {}", changed_files.len()));
    for file in changed_files.iter().take(STATUS_SAMPLE_LIMIT) {
        lines.push(format!("  - {}", bounded_field(file, 512)));
    }
    if changed_files.len() > STATUS_SAMPLE_LIMIT {
        lines.push(format!(
            "omitted paths: {} (use `turnlog status --full` for full output)",
            changed_files.len() - STATUS_SAMPLE_LIMIT
        ));
    }
}

fn append_bounded_line(output: &mut String, line: &str) -> bool {
    let remaining = STATUS_MAX_BYTES.saturating_sub(output.len());
    if remaining == 0 {
        return true;
    }
    let (line, truncated) = truncate_utf8(line, remaining.saturating_sub(1));
    output.push_str(line);
    if truncated {
        output.push('…');
    }
    output.push('\n');
    truncated
}

fn print_status_full(cwd: &std::path::Path) -> Result<()> {
    let store = Store::discover(cwd).ok();
    println!(
        "turnlog: {}",
        if store.is_some() {
            "initialized"
        } else {
            "not initialized"
        }
    );

    if let Some(store) = &store {
        match store.current_session() {
            Ok(session) => println!(
                "current session: {} ticket={} goal={}",
                session.id,
                session.ticket.as_deref().unwrap_or("none"),
                session.goal
            ),
            Err(_) => println!("current session: none"),
        }
        if let Ok(latest) = store.latest_session() {
            let current_id = store.current_session_id()?.unwrap_or_default();
            if latest.id != current_id {
                println!(
                    "latest session: {} ticket={} goal={}",
                    latest.id,
                    latest.ticket.as_deref().unwrap_or("none"),
                    latest.goal
                );
            }
        }
        let orphaned = store.orphaned_records()?;
        if !orphaned.is_empty() {
            eprintln!(
                "warning: {} canonical records are missing from index; run `turnlog repair`",
                orphaned.len()
            );
        }
        match store.latest_turn()? {
            Some(turn) => println!(
                "last turn: {} session={} summary={}",
                turn.id,
                turn.session,
                turn.summary.as_deref().unwrap_or("")
            ),
            None => println!("last turn: none"),
        }
    }

    let vcs = vcs::detect(cwd);
    print_vcs_status(&vcs);
    Ok(())
}

fn print_vcs_status(vcs: &model::VcsInfo) {
    println!("vcs: {}", vcs_kind(vcs));
    match vcs {
        model::VcsInfo::Jj {
            jj_change,
            jj_commit,
            jj_operation,
            git_head,
            git_branch,
            dirty,
            changed_files,
        } => {
            println!("jj change: {}", jj_change.as_deref().unwrap_or("unknown"));
            println!("jj commit: {}", jj_commit.as_deref().unwrap_or("unknown"));
            println!(
                "jj operation: {}",
                jj_operation.as_deref().unwrap_or("unknown")
            );
            print_git_bits(git_head.as_deref(), git_branch.as_deref());
            print_dirty(*dirty, changed_files);
        }
        model::VcsInfo::Git {
            git_head,
            git_branch,
            dirty,
            changed_files,
        } => {
            print_git_bits(git_head.as_deref(), git_branch.as_deref());
            print_dirty(*dirty, changed_files);
        }
        model::VcsInfo::None => {}
    }
}

fn print_git_bits(git_head: Option<&str>, git_branch: Option<&str>) {
    println!("git branch: {}", git_branch.unwrap_or("none"));
    println!("git head: {}", git_head.unwrap_or("none"));
}

fn print_dirty(dirty: bool, changed_files: &[String]) {
    println!("dirty: {}", dirty);
    if changed_files.is_empty() {
        println!("changed files: none");
    } else {
        println!("changed files:");
        for file in changed_files {
            println!("  - {file}");
        }
    }
}

fn print_event_summary(event: &Event) {
    match event {
        Event::SessionStarted { session } => {
            println!(
                "session {} ticket={} goal={}",
                session.id,
                session.ticket.as_deref().unwrap_or("none"),
                session.goal
            );
        }
        Event::TurnRecorded { turn } => println!(
            "turn {} session={} model={} summary={}",
            turn.id,
            turn.session,
            turn.model.as_deref().unwrap_or("unknown"),
            turn.summary.as_deref().unwrap_or("")
        ),
    }
}

fn event_matches(
    event: &Event,
    session_filter: Option<&str>,
    ticket_filter: Option<&str>,
    ticket_sessions: &std::collections::HashSet<String>,
    grep_filter: Option<&str>,
    changed_filter: Option<&str>,
) -> Result<bool> {
    if let Some(session_id) = session_filter {
        match event {
            Event::SessionStarted { session } if session.id != session_id => return Ok(false),
            Event::TurnRecorded { turn } if turn.session != session_id => return Ok(false),
            _ => {}
        }
    }

    if let Some(ticket) = ticket_filter {
        match event {
            Event::SessionStarted { session } if session.ticket.as_deref() != Some(ticket) => {
                return Ok(false);
            }
            Event::TurnRecorded { turn } if !ticket_sessions.contains(&turn.session) => {
                return Ok(false);
            }
            _ => {}
        }
    }

    if let Some(changed) = changed_filter {
        match event {
            Event::TurnRecorded { turn } => {
                if !store::changed_files(&turn.vcs)
                    .iter()
                    .any(|file| file.contains(changed))
                {
                    return Ok(false);
                }
            }
            Event::SessionStarted { .. } => return Ok(false),
        }
    }

    if let Some(grep) = grep_filter {
        let json = serde_json::to_string(event)?;
        if !json.to_lowercase().contains(&grep.to_lowercase()) {
            return Ok(false);
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn turn() -> Event {
        Event::TurnRecorded {
            turn: Turn {
                schema_version: SCHEMA_VERSION,
                id: "turn_test".to_string(),
                session: "sess_test".to_string(),
                created_at: OffsetDateTime::UNIX_EPOCH,
                model: Some("model".to_string()),
                summary: Some("fixed timestamp rendering".to_string()),
                verification: vec!["cargo test".to_string()],
                attachments: vec![],
                vcs: model::VcsInfo::Git {
                    git_head: Some("abc".to_string()),
                    git_branch: Some("main".to_string()),
                    dirty: true,
                    changed_files: vec!["src/store.rs".to_string()],
                },
            },
        }
    }

    #[test]
    fn filters_turn_by_session_grep_and_changed_file() {
        let event = turn();
        assert!(
            event_matches(
                &event,
                Some("sess_test"),
                None,
                &HashSet::new(),
                Some("timestamp"),
                Some("store.rs")
            )
            .unwrap()
        );
        assert!(!event_matches(&event, Some("other"), None, &HashSet::new(), None, None).unwrap());
        assert!(
            !event_matches(
                &event,
                None,
                None,
                &HashSet::new(),
                None,
                Some("src/main.rs")
            )
            .unwrap()
        );
    }

    #[test]
    fn filters_turn_by_ticket_session_membership() {
        let event = turn();
        let mut ticket_sessions = HashSet::new();
        ticket_sessions.insert("sess_test".to_string());
        assert!(event_matches(&event, None, Some("TEST-1"), &ticket_sessions, None, None).unwrap());
        assert!(!event_matches(&event, None, Some("TEST-1"), &HashSet::new(), None, None).unwrap());
    }

    #[test]
    fn bounded_status_summarizes_large_unicode_path_sets() {
        let paths: Vec<String> = (0..30_523)
            .map(|index| format!("研究/非常に長いファイル名-{index}.txt"))
            .collect();
        let mut lines = Vec::new();
        append_changed_summary(&mut lines, &paths);
        let mut output = String::new();
        for line in lines {
            append_bounded_line(&mut output, &line);
        }
        assert!(output.len() <= STATUS_MAX_BYTES);
        assert!(output.is_char_boundary(output.len()));
        assert!(output.contains("changed files: 30523"));
        assert!(output.contains("omitted paths: 30503"));
    }

    #[test]
    fn bounded_status_fields_are_utf8_safe() {
        let value = "é".repeat(20_000);
        let bounded = bounded_field(&value, 256);
        assert!(bounded.len() <= 256);
        assert!(bounded.is_char_boundary(bounded.len()));
        assert!(bounded.ends_with('…'));
        assert_eq!(truncate_utf8("hello", 20), ("hello", false));
    }
}
