use crate::model::{Event, Session, Turn};
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

    pub fn latest_session(&self) -> Result<Session> {
        let mut latest: Option<Session> = None;
        for event in self.events()? {
            if let Event::SessionStarted { session } = event {
                latest = Some(session);
            }
        }
        latest.context("no session found; run `atrace start`")
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
    let verification = if t.verification.is_empty() {
        "- none\n".to_string()
    } else {
        t.verification
            .iter()
            .map(|v| format!("- `{v}`\n"))
            .collect()
    };
    format!(
        "# Turn {}\n\nSession: {}  \nModel: {}  \nSummary: {}  \nCreated: {}\n\n## Verification\n\n{}\n## VCS\n\n```json\n{}\n```\n",
        t.id,
        t.session,
        t.model.as_deref().unwrap_or("unknown"),
        t.summary.as_deref().unwrap_or(""),
        format_time(&t.created_at),
        verification,
        serde_json::to_string_pretty(&t.vcs).unwrap_or_default()
    )
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
    }
}
