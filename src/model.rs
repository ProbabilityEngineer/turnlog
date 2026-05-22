use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub schema_version: u32,
    pub id: String,
    pub ticket: Option<String>,
    pub goal: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub repo_root: String,
    pub vcs_start: VcsInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Turn {
    pub schema_version: u32,
    pub id: String,
    pub session: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub model: Option<String>,
    pub summary: Option<String>,
    pub verification: Vec<String>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    pub vcs: VcsInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Attachment {
    pub kind: AttachmentKind,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentKind {
    Diff,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VcsInfo {
    Jj {
        jj_change: Option<String>,
        jj_commit: Option<String>,
        jj_operation: Option<String>,
        git_head: Option<String>,
        git_branch: Option<String>,
        dirty: bool,
        changed_files: Vec<String>,
    },
    Git {
        git_head: Option<String>,
        git_branch: Option<String>,
        dirty: bool,
        changed_files: Vec<String>,
    },
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    SessionStarted { session: Session },
    TurnRecorded { turn: Turn },
}

impl Event {
    pub fn id(&self) -> &str {
        match self {
            Event::SessionStarted { session } => &session.id,
            Event::TurnRecorded { turn } => &turn.id,
        }
    }
}
