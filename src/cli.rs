use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "atrace",
    version,
    about = "Lightweight agent provenance recorder"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialize .atrace storage
    Init,
    /// Start a new agent session
    Start {
        #[arg(long)]
        ticket: Option<String>,
        #[arg(long)]
        goal: String,
    },
    /// Record an agent turn on the latest session
    Record {
        #[arg(long)]
        session: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long = "verification")]
        verification: Vec<String>,
        #[arg(long)]
        attach_diff: bool,
    },
    /// Show the active session
    Current,
    /// Set the active session
    Use { id: String },
    /// Show current atrace and VCS status
    Status,
    /// List trace events
    Log {
        #[arg(long)]
        session: Option<String>,
        #[arg(long)]
        ticket: Option<String>,
        #[arg(long)]
        grep: Option<String>,
        #[arg(long)]
        changed: Option<String>,
    },
    /// Show one session or turn by ID
    Show {
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// Search trace events for text
    Grep { pattern: String },
    /// Write a Markdown report for a session
    Report {
        id: String,
        #[arg(long)]
        stdout: bool,
    },
}
