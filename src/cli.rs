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
        model: Option<String>,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long = "verification")]
        verification: Vec<String>,
    },
    /// Show current atrace and VCS status
    Status,
    /// List trace events
    Log,
    /// Show one session or turn by ID
    Show { id: String },
}
