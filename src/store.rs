use crate::model::{Attachment, Event, Session, Turn};
use anyhow::{Context, Result, bail};
use fs2::FileExt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};
use time::format_description::well_known::Rfc3339;

#[derive(Clone)]
pub struct Store {
    root: PathBuf,
}

struct QueriedEvents {
    events: Vec<Event>,
    warnings: Vec<String>,
}

impl Store {
    pub fn discover(cwd: &Path) -> Result<Self> {
        let mut cur = cwd.to_path_buf();
        loop {
            let candidate = cur.join(".turnlog");
            if candidate.is_dir() {
                return Ok(Self { root: candidate });
            }
            if !cur.pop() {
                break;
            }
        }
        bail!("not in an turnlog repo; run `turnlog init`")
    }

    pub fn at_repo_root(repo_root: &Path) -> Self {
        Self {
            root: repo_root.join(".turnlog"),
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
        self.ensure_gitignored()?;
        Ok(())
    }

    fn ensure_gitignored(&self) -> Result<()> {
        let repo_root = self.root.parent().unwrap_or(&self.root);
        let gitignore = repo_root.join(".gitignore");
        let existing = match fs::read_to_string(&gitignore) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => {
                return Err(error).with_context(|| format!("read {}", gitignore.display()));
            }
        };
        if existing
            .lines()
            .map(str::trim)
            .any(|line| line == ".turnlog" || line == ".turnlog/")
        {
            return Ok(());
        }
        let separator = if existing.is_empty() || existing.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        fs::write(&gitignore, format!("{existing}{separator}.turnlog/\n"))
            .with_context(|| format!("write {}", gitignore.display()))?;
        Ok(())
    }

    fn lock(&self) -> Result<StoreLock> {
        StoreLock::acquire(self.root.join(".lock"))
    }

    pub fn write_session(&self, session: &Session) -> Result<()> {
        let json = serde_json::to_string_pretty(session)?;
        let _lock = self.lock()?;
        self.validate_index()?;
        atomic_write(
            &self
                .root
                .join("sessions")
                .join(format!("{}.json", session.id)),
            json.as_bytes(),
        )?;
        atomic_write(
            &self
                .root
                .join("sessions")
                .join(format!("{}.md", session.id)),
            render_session(session).as_bytes(),
        )?;
        let mut bytes = serde_json::to_vec(&Event::SessionStarted {
            session: session.clone(),
        })?;
        bytes.push(b'\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("index.jsonl"))?;
        file.write_all(&bytes)?;
        file.sync_data()?;
        Ok(())
    }

    pub fn write_turn(&self, turn: &Turn) -> Result<()> {
        let json = serde_json::to_string_pretty(turn)?;
        let _lock = self.lock()?;
        self.validate_index()?;
        atomic_write(
            &self.root.join("turns").join(format!("{}.json", turn.id)),
            json.as_bytes(),
        )?;
        atomic_write(
            &self.root.join("turns").join(format!("{}.md", turn.id)),
            render_turn(turn).as_bytes(),
        )?;
        let mut bytes = serde_json::to_vec(&Event::TurnRecorded { turn: turn.clone() })?;
        bytes.push(b'\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("index.jsonl"))?;
        file.write_all(&bytes)?;
        file.sync_data()?;
        Ok(())
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
        latest.context("no session found; run `turnlog start`")
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
            .context("no active session; run `turnlog start` or `turnlog use <session-id>`")?;
        self.session_by_id(&id)?.with_context(|| {
            format!("active session {id} was not found; run `turnlog use <session-id>`")
        })
    }

    pub fn session_by_id(&self, id: &str) -> Result<Option<Session>> {
        Ok(self.events()?.into_iter().find_map(|event| match event {
            Event::SessionStarted { session } if session.id == id => Some(session),
            _ => None,
        }))
    }

    pub fn events(&self) -> Result<Vec<Event>> {
        let queried = self.query_events()?;
        for warning in queried.warnings {
            eprintln!("warning: {warning}; results are incomplete; run `turnlog repair`");
        }
        Ok(queried.events)
    }

    fn validate_index(&self) -> Result<()> {
        self.read_index(true)?;
        let queried = self.query_events()?;
        if let Some(warning) = queried.warnings.first() {
            bail!("{warning}; refusing to append; run `turnlog repair`");
        }
        Ok(())
    }

    fn read_index(&self, strict: bool) -> Result<(Vec<Event>, Vec<String>)> {
        let path = self.root.join("index.jsonl");
        let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let mut events = Vec::new();
        let mut warnings = Vec::new();
        for (number, line) in raw
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.trim().is_empty())
        {
            match serde_json::from_str(line) {
                Ok(event) => events.push(event),
                Err(error) => {
                    let excerpt: String = line.chars().take(80).collect();
                    let warning = format!(
                        "{}:{}: invalid JSON at column {}; excerpt: {:?}",
                        path.display(),
                        number + 1,
                        error.column(),
                        excerpt
                    );
                    if strict {
                        bail!(warning);
                    }
                    warnings.push(warning);
                }
            }
        }
        Ok((events, warnings))
    }

    fn canonical_events(&self) -> Result<(Vec<Event>, Vec<String>)> {
        let mut events = Vec::new();
        let mut warnings = Vec::new();
        for (dir, kind) in [("sessions", "session"), ("turns", "turn")] {
            for entry in fs::read_dir(self.root.join(dir))? {
                let path = entry?.path();
                if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                    continue;
                }
                let parsed = fs::read_to_string(&path).ok().and_then(|raw| match kind {
                    "session" => serde_json::from_str::<Session>(&raw)
                        .ok()
                        .map(|session| Event::SessionStarted { session }),
                    _ => serde_json::from_str::<Turn>(&raw)
                        .ok()
                        .map(|turn| Event::TurnRecorded { turn }),
                });
                match parsed {
                    Some(event) => events.push(event),
                    None => warnings.push(format!("invalid canonical record {}", path.display())),
                }
            }
        }
        Ok((events, warnings))
    }

    fn query_events(&self) -> Result<QueriedEvents> {
        let (indexed, mut warnings) = self.read_index(false)?;
        let (canonical, canonical_warnings) = self.canonical_events()?;
        warnings.extend(canonical_warnings);
        let indexed_by_id: std::collections::HashMap<_, _> = indexed
            .into_iter()
            .map(|event| (event.id().to_owned(), event))
            .collect();
        let canonical_ids: std::collections::HashSet<_> = canonical
            .iter()
            .map(|event| event.id().to_owned())
            .collect();
        let mut events = Vec::new();
        for event in canonical {
            match indexed_by_id.get(event.id()) {
                None => warnings.push(format!(
                    "canonical record {} is missing from index",
                    event.id()
                )),
                Some(indexed) if indexed != &event => warnings.push(format!(
                    "index record {} disagrees with canonical record",
                    event.id()
                )),
                _ => {}
            }
            events.push(event);
        }
        for id in indexed_by_id
            .keys()
            .filter(|id| !canonical_ids.contains(*id))
        {
            warnings.push(format!(
                "index record {id} has no canonical record and was ignored"
            ));
        }
        events.sort_by(|left, right| {
            left.created_at()
                .cmp(&right.created_at())
                .then_with(|| left.id().cmp(right.id()))
        });
        Ok(QueriedEvents { events, warnings })
    }

    pub fn orphaned_records(&self) -> Result<Vec<PathBuf>> {
        let indexed: std::collections::HashSet<String> = self
            .read_index(false)?
            .0
            .into_iter()
            .map(|event| event.id().to_owned())
            .collect();
        let mut orphaned = Vec::new();
        for dir in ["sessions", "turns"] {
            for entry in fs::read_dir(self.root.join(dir))? {
                let path = entry?.path();
                if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                    continue;
                }
                let Some(id) = path.file_stem().and_then(|stem| stem.to_str()) else {
                    continue;
                };
                if !indexed.contains(id) {
                    orphaned.push(path);
                }
            }
        }
        orphaned.sort();
        Ok(orphaned)
    }

    pub fn repair_index(&self) -> Result<(usize, Vec<PathBuf>)> {
        let _lock = self.lock()?;
        let index = self.root.join("index.jsonl");
        if index.exists() {
            fs::copy(&index, index.with_extension("jsonl.bak"))?;
        }
        let mut events = Vec::new();
        let mut skipped = Vec::new();
        for (dir, kind) in [("sessions", "session"), ("turns", "turn")] {
            for entry in fs::read_dir(self.root.join(dir))? {
                let path = entry?.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let parsed = fs::read_to_string(&path).ok().and_then(|raw| match kind {
                    "session" => serde_json::from_str::<crate::model::Session>(&raw)
                        .ok()
                        .map(|s| Event::SessionStarted { session: s }),
                    _ => serde_json::from_str::<crate::model::Turn>(&raw)
                        .ok()
                        .map(|t| Event::TurnRecorded { turn: t }),
                });
                match parsed {
                    Some(event) => events.push(event),
                    None => skipped.push(path),
                }
            }
        }
        events.sort_by(|left, right| {
            left.created_at()
                .cmp(&right.created_at())
                .then_with(|| left.id().cmp(right.id()))
        });
        let mut bytes = Vec::new();
        for event in &events {
            bytes.extend(serde_json::to_vec(event)?);
            bytes.push(b'\n');
        }
        let temp = index.with_extension("jsonl.tmp");
        atomic_write(&temp, &bytes)?;
        fs::rename(temp, index)?;
        Ok((events.len(), skipped))
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

fn atomic_temp_path(path: &Path) -> PathBuf {
    path.with_extension(format!("{}.tmp", std::process::id()))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp = atomic_temp_path(path);
    let mut file = fs::File::create(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(temp, path)?;
    Ok(())
}

struct StoreLock {
    file: fs::File,
}

impl StoreLock {
    fn acquire(path: PathBuf) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match file.try_lock_exclusive() {
                Ok(()) => return Ok(Self { file }),
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    bail!("timed out waiting for active turnlog writer; retry after it finishes")
                }
                Err(error) => return Err(error.into()),
            }
        }
    }
}

impl Drop for StoreLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
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
        assert!(dir.path().join(".turnlog/index.jsonl").exists());
        assert!(dir.path().join(".turnlog/sessions").is_dir());
        assert!(dir.path().join(".turnlog/turns").is_dir());
        assert!(dir.path().join(".turnlog/reports").is_dir());
        assert!(
            fs::read_to_string(dir.path().join(".gitignore"))
                .unwrap()
                .lines()
                .any(|line| line == ".turnlog/")
        );
    }

    #[test]
    fn init_does_not_duplicate_gitignore_entry() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".gitignore"), "target/\n.turnlog\n").unwrap();
        let store = Store::at_repo_root(dir.path());
        store.init().unwrap();
        store.init().unwrap();
        let gitignore = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        let count = gitignore
            .lines()
            .filter(|line| matches!(line.trim(), ".turnlog" | ".turnlog/"))
            .count();
        assert_eq!(count, 1);
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

    fn session(id: impl Into<String>) -> Session {
        Session {
            schema_version: crate::model::SCHEMA_VERSION,
            id: id.into(),
            ticket: None,
            goal: "test".to_string(),
            created_at: time::OffsetDateTime::UNIX_EPOCH,
            repo_root: "test".to_string(),
            vcs_start: crate::model::VcsInfo::None,
        }
    }

    #[test]
    fn write_session_report_creates_markdown() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at_repo_root(dir.path());
        store.init().unwrap();
        let mut session = session("sess_test");
        session.ticket = Some("T-1".to_string());
        session.goal = "Test reports".to_string();
        session.repo_root = dir.path().display().to_string();
        store.write_session(&session).unwrap();
        let path = store.write_session_report(&session).unwrap();
        let report = fs::read_to_string(path).unwrap();
        assert!(report.contains("# Session sess_test"));
        assert!(report.contains("## Turns"));
        assert!(report.contains("No turns recorded"));
    }

    #[test]
    fn malformed_index_is_skipped_for_reads_and_refuses_writes() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at_repo_root(dir.path());
        store.init().unwrap();
        store.write_session(&session("sess_valid")).unwrap();
        fs::write(dir.path().join(".turnlog/index.jsonl"), "not json\n").unwrap();
        assert_eq!(store.events().unwrap().len(), 1);
        assert!(store.write_session(&session("sess_rejected")).is_err());
    }

    #[test]
    fn canonical_records_win_over_disagreeing_index_entries() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at_repo_root(dir.path());
        store.init().unwrap();
        let original = session("sess_canonical");
        store.write_session(&original).unwrap();
        let mut canonical = original.clone();
        canonical.goal = "canonical correction".to_string();
        fs::write(
            store.root.join("sessions/sess_canonical.json"),
            serde_json::to_string_pretty(&canonical).unwrap(),
        )
        .unwrap();
        let events = store.events().unwrap();
        assert!(
            matches!(&events[0], Event::SessionStarted { session } if session.goal == "canonical correction")
        );
        assert!(store.write_session(&session("sess_rejected")).is_err());
    }

    #[test]
    fn index_only_events_are_ignored_and_require_repair_before_writes() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at_repo_root(dir.path());
        store.init().unwrap();
        let stale = Event::SessionStarted {
            session: session("sess_stale_index"),
        };
        fs::write(
            store.root.join("index.jsonl"),
            format!("{}\n", serde_json::to_string(&stale).unwrap()),
        )
        .unwrap();
        assert!(store.events().unwrap().is_empty());
        assert!(store.write_session(&session("sess_rejected")).is_err());
    }

    #[test]
    fn canonical_queries_are_deterministically_ordered_by_timestamp_then_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at_repo_root(dir.path());
        store.init().unwrap();
        let mut later = session("sess_a");
        later.created_at += time::Duration::seconds(1);
        let earlier = session("sess_z");
        for record in [&later, &earlier] {
            fs::write(
                store
                    .root
                    .join("sessions")
                    .join(format!("{}.json", record.id)),
                serde_json::to_string_pretty(record).unwrap(),
            )
            .unwrap();
        }
        assert_eq!(
            store
                .events()
                .unwrap()
                .iter()
                .map(|event| event.id())
                .collect::<Vec<_>>(),
            vec!["sess_z", "sess_a"]
        );
    }

    #[test]
    fn repair_detects_and_indexes_orphaned_canonical_records() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at_repo_root(dir.path());
        store.init().unwrap();
        store.write_session(&session("sess_orphan")).unwrap();
        fs::write(dir.path().join(".turnlog/index.jsonl"), "").unwrap();
        assert_eq!(store.orphaned_records().unwrap().len(), 1);
        assert_eq!(store.repair_index().unwrap().0, 1);
        assert!(store.orphaned_records().unwrap().is_empty());
        assert!(dir.path().join(".turnlog/index.jsonl.bak").exists());
    }

    #[test]
    fn leftover_lock_file_does_not_block_writes() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at_repo_root(dir.path());
        store.init().unwrap();
        fs::write(
            dir.path().join(".turnlog/.lock"),
            "left by a crashed process\n",
        )
        .unwrap();
        store.write_session(&session("sess_after_crash")).unwrap();
        assert!(store.root.join(".lock").exists());
    }

    #[test]
    fn failed_canonical_atomic_write_leaves_no_partial_record() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("record.json");
        fs::create_dir(atomic_temp_path(&path)).unwrap();
        assert!(atomic_write(&path, b"complete record").is_err());
        assert!(!path.exists());
    }

    #[test]
    fn interruption_after_canonical_write_leaves_an_orphan_repair_can_restore() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at_repo_root(dir.path());
        store.init().unwrap();
        let session = session("sess_orphan_after_interruption");
        atomic_write(
            &store
                .root
                .join("sessions/sess_orphan_after_interruption.json"),
            serde_json::to_string_pretty(&session).unwrap().as_bytes(),
        )
        .unwrap();
        fs::write(
            store
                .root
                .join("sessions/sess_orphan_after_interruption.md"),
            render_session(&session),
        )
        .unwrap();
        assert_eq!(store.orphaned_records().unwrap().len(), 1);
        assert_eq!(store.repair_index().unwrap().0, 1);
        assert!(store.orphaned_records().unwrap().is_empty());
    }

    #[test]
    fn failed_repair_before_replacement_preserves_existing_index() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at_repo_root(dir.path());
        store.init().unwrap();
        store.write_session(&session("sess_repair_safe")).unwrap();
        let index = store.root.join("index.jsonl");
        let original = fs::read_to_string(&index).unwrap();
        let replacement_temp = index.with_extension("jsonl.tmp");
        fs::create_dir(atomic_temp_path(&replacement_temp)).unwrap();
        assert!(store.repair_index().is_err());
        assert_eq!(fs::read_to_string(&index).unwrap(), original);
    }

    #[test]
    fn concurrent_writers_produce_complete_parseable_events() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at_repo_root(dir.path());
        store.init().unwrap();
        let mut workers = Vec::new();
        for index in 0..24 {
            let store = store.clone();
            workers.push(thread::spawn(move || {
                store.write_session(&session(format!("sess_{index:02}")))
            }));
        }
        for worker in workers {
            worker.join().unwrap().unwrap();
        }
        let events = store.events().unwrap();
        assert_eq!(events.len(), 24);
        let ids: std::collections::HashSet<_> = events.iter().map(Event::id).collect();
        assert_eq!(ids.len(), 24);
    }
}
