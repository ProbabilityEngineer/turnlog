use crate::model::{Attachment, Event, Session, Turn};
use anyhow::{Context, Result, bail};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use time::format_description::well_known::Rfc3339;

pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn discover(cwd: &Path) -> Result<Self> {
        let mut cur = cwd.to_path_buf();
        loop {
            let candidate = cur.join(".atrace");
            if candidate.is_dir() {
                return Ok(Self { root: candidate });
            }
            if !cur.pop() {
                break;
            }
        }
        bail!("not in an atrace repo; run `atrace init`")
    }

    pub fn at_repo_root(repo_root: &Path) -> Self {
        Self {
            root: repo_root.join(".atrace"),
        }
    }

    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(self.root.join("sessions"))?;
        fs::create_dir_all(self.root.join("turns"))?;
        fs::create_dir_all(self.root.join("attachments"))?;
        fs::create_dir_all(self.root.join("reports"))?;
        if !self.root.join("index.jsonl").exists() {
            fs::write(self.root.join("index.jsonl"), "")?;
        }
        Ok(())
    }

    pub fn append_event(&self, event: &Event) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("index.jsonl"))?;
        serde_json::to_writer(&mut file, event)?;
        writeln!(file)?;
        Ok(())
    }

    pub fn write_session(&self, session: &Session) -> Result<()> {
        let json = serde_json::to_string_pretty(session)?;
        fs::write(
            self.root
                .join("sessions")
                .join(format!("{}.json", session.id)),
            json,
        )?;
        fs::write(
            self.root
                .join("sessions")
                .join(format!("{}.md", session.id)),
            render_session(session),
        )?;
        self.append_event(&Event::SessionStarted {
            session: session.clone(),
        })
    }

    pub fn write_turn(&self, turn: &Turn) -> Result<()> {
        let json = serde_json::to_string_pretty(turn)?;
        fs::write(
            self.root.join("turns").join(format!("{}.json", turn.id)),
            json,
        )?;
        fs::write(
            self.root.join("turns").join(format!("{}.md", turn.id)),
            render_turn(turn),
        )?;
        self.append_event(&Event::TurnRecorded { turn: turn.clone() })
    }

    pub fn write_attachment(&self, path: &str, content: &str) -> Result<()> {
        let full_path = self.root.parent().unwrap_or(&self.root).join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full_path, content)?;
        Ok(())
    }

    pub fn latest_session(&self) -> Result<Session> {
        let mut latest: Option<Session> = None;
        for event in self.events()? {
            if let Event::SessionStarted { session } = event {
                latest = Some(session);
            }
        }
        latest.context("no session found; run `atrace start`")
    }

    pub fn set_current_session(&self, session_id: &str) -> Result<()> {
        fs::write(self.root.join("current-session"), format!("{session_id}\n"))?;
        Ok(())
    }

    pub fn current_session_id(&self) -> Result<Option<String>> {
        let path = self.root.join("current-session");
        if !path.exists() {
            return Ok(None);
        }
        let id = fs::read_to_string(path)?.trim().to_string();
        Ok((!id.is_empty()).then_some(id))
    }

    pub fn current_session(&self) -> Result<Session> {
        let id = self
            .current_session_id()?
            .context("no active session; run `atrace start` or `atrace use <session-id>`")?;
        self.session_by_id(&id)?.with_context(|| {
            format!("active session {id} was not found; run `atrace use <session-id>`")
        })
    }

    pub fn session_by_id(&self, id: &str) -> Result<Option<Session>> {
        Ok(self.events()?.into_iter().find_map(|event| match event {
            Event::SessionStarted { session } if session.id == id => Some(session),
            _ => None,
        }))
    }

    pub fn events(&self) -> Result<Vec<Event>> {
        let path = self.root.join("index.jsonl");
        let raw = fs::read_to_string(path)?;
        raw.lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| Ok(serde_json::from_str(line)?))
            .collect()
    }

    pub fn find(&self, id: &str) -> Result<Option<Event>> {
        Ok(self.events()?.into_iter().find(|e| e.id() == id))
    }

    pub fn turns_for_session(&self, session_id: &str) -> Result<Vec<Turn>> {
        Ok(self
            .events()?
            .into_iter()
            .filter_map(|event| match event {
                Event::TurnRecorded { turn } if turn.session == session_id => Some(turn),
                _ => None,
            })
            .collect())
    }

    pub fn latest_turn(&self) -> Result<Option<Turn>> {
        Ok(self
            .events()?
            .into_iter()
            .filter_map(|event| match event {
                Event::TurnRecorded { turn } => Some(turn),
                _ => None,
            })
            .last())
    }

    pub fn render_session_rollup(&self, session: &Session) -> Result<String> {
        let turns = self.turns_for_session(&session.id)?;
        Ok(render_session_rollup(session, &turns))
    }

    pub fn write_session_report(&self, session: &Session) -> Result<PathBuf> {
        let report = self.render_session_rollup(session)?;
        let reports_dir = self.root.join("reports");
        fs::create_dir_all(&reports_dir)?;
        let path = reports_dir.join(format!("{}.md", session.id));
        fs::write(&path, report)?;
        Ok(path)
    }
}

fn format_time(timestamp: &time::OffsetDateTime) -> String {
    timestamp
        .format(&Rfc3339)
        .unwrap_or_else(|_| timestamp.to_string())
}

fn render_session(s: &Session) -> String {
    format!(
        "# Session {}\n\nTicket: {}  \nGoal: {}  \nCreated: {}  \nRepo: {}\n\n## VCS at start\n\n```json\n{}\n```\n",
        s.id,
        s.ticket.as_deref().unwrap_or("none"),
        s.goal,
        format_time(&s.created_at),
        s.repo_root,
        serde_json::to_string_pretty(&s.vcs_start).unwrap_or_default()
    )
}

fn render_turn(t: &Turn) -> String {
    let verification = verification_markdown(&t.verification);
    format!(
        "# Turn {}\n\nSession: {}  \nModel: {}  \nSummary: {}  \nCreated: {}\n\n## Verification\n\n{}\n## Attachments\n\n{}\n## VCS\n\n```json\n{}\n```\n",
        t.id,
        t.session,
        t.model.as_deref().unwrap_or("unknown"),
        t.summary.as_deref().unwrap_or(""),
        format_time(&t.created_at),
        verification,
        attachments_markdown(&t.attachments),
        serde_json::to_string_pretty(&t.vcs).unwrap_or_default()
    )
}

fn render_session_rollup(session: &Session, turns: &[Turn]) -> String {
    let mut out = render_session(session);
    out.push_str("\n## Turns\n\n");
    if turns.is_empty() {
        out.push_str("No turns recorded.\n");
        return out;
    }
    for turn in turns {
        out.push_str(&format!(
            "### {}\n\nModel: {}  \nSummary: {}  \nCreated: {}\n\nVerification:\n{}",
            turn.id,
            turn.model.as_deref().unwrap_or("unknown"),
            turn.summary.as_deref().unwrap_or(""),
            format_time(&turn.created_at),
            verification_markdown(&turn.verification)
        ));
        let files = changed_files(&turn.vcs);
        out.push_str("\nAttachments:\n");
        out.push_str(&attachments_markdown(&turn.attachments));
        out.push_str("\nChanged files:\n");
        if files.is_empty() {
            out.push_str("- none\n\n");
        } else {
            for file in files {
                out.push_str(&format!("- `{file}`\n"));
            }
            out.push('\n');
        }
    }
    out
}

fn verification_markdown(verification: &[String]) -> String {
    if verification.is_empty() {
        "- none\n".to_string()
    } else {
        verification.iter().map(|v| format!("- `{v}`\n")).collect()
    }
}

fn attachments_markdown(attachments: &[Attachment]) -> String {
    if attachments.is_empty() {
        "- none\n".to_string()
    } else {
        attachments
            .iter()
            .map(|a| format!("- {:?}: `{}`\n", a.kind, a.path))
            .collect()
    }
}

pub fn changed_files(vcs: &crate::model::VcsInfo) -> &[String] {
    match vcs {
        crate::model::VcsInfo::Jj { changed_files, .. }
        | crate::model::VcsInfo::Git { changed_files, .. } => changed_files,
        crate::model::VcsInfo::None => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn init_creates_layout() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at_repo_root(dir.path());
        store.init().unwrap();
        assert!(dir.path().join(".atrace/index.jsonl").exists());
        assert!(dir.path().join(".atrace/sessions").is_dir());
        assert!(dir.path().join(".atrace/turns").is_dir());
        assert!(dir.path().join(".atrace/reports").is_dir());
    }

    #[test]
    fn current_session_marker_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at_repo_root(dir.path());
        store.init().unwrap();
        assert_eq!(store.current_session_id().unwrap(), None);
        store.set_current_session("sess_test").unwrap();
        assert_eq!(
            store.current_session_id().unwrap().as_deref(),
            Some("sess_test")
        );
    }

    #[test]
    fn write_session_report_creates_markdown() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at_repo_root(dir.path());
        store.init().unwrap();
        let session = Session {
            schema_version: crate::model::SCHEMA_VERSION,
            id: "sess_test".to_string(),
            ticket: Some("T-1".to_string()),
            goal: "Test reports".to_string(),
            created_at: time::OffsetDateTime::UNIX_EPOCH,
            repo_root: dir.path().display().to_string(),
            vcs_start: crate::model::VcsInfo::None,
        };
        store.write_session(&session).unwrap();
        let path = store.write_session_report(&session).unwrap();
        let report = fs::read_to_string(path).unwrap();
        assert!(report.contains("# Session sess_test"));
        assert!(report.contains("## Turns"));
        assert!(report.contains("No turns recorded"));
    }
}
