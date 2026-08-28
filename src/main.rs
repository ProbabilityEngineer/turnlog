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
        Command::Status => {
            print_status(&cwd)?;
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

fn print_status(cwd: &std::path::Path) -> Result<()> {
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
}
