mod cli;
mod ids;
mod model;
mod store;
mod vcs;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use model::{Event, SCHEMA_VERSION, Session, Turn};
use store::Store;
use time::OffsetDateTime;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cwd = std::env::current_dir()?;
    match cli.command {
        Command::Init => {
            let root = vcs::repo_root(&cwd);
            let store = Store::at_repo_root(&root);
            store.init()?;
            println!("initialized {}", root.join(".atrace").display());
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
            println!("started {}", session.id);
        }
        Command::Record {
            model,
            summary,
            verification,
        } => {
            let store = Store::discover(&cwd)?;
            let session = store.latest_session()?;
            let turn = Turn {
                schema_version: SCHEMA_VERSION,
                id: ids::turn_id(),
                session: session.id,
                created_at: OffsetDateTime::now_utc(),
                model,
                summary,
                verification,
                vcs: vcs::detect(&cwd),
            };
            store.write_turn(&turn)?;
            println!("recorded {}", turn.id);
        }
        Command::Status => {
            let vcs = vcs::detect(&cwd);
            println!("vcs: {}", vcs_kind(&vcs));
            match Store::discover(&cwd) {
                Ok(store) => match store.latest_session() {
                    Ok(session) => println!("latest session: {} {}", session.id, session.goal),
                    Err(_) => println!("latest session: none"),
                },
                Err(_) => println!("atrace: not initialized"),
            }
        }
        Command::Log => {
            let store = Store::discover(&cwd)?;
            for event in store.events()? {
                match event {
                    Event::SessionStarted { session } => {
                        println!("session {} {}", session.id, session.goal)
                    }
                    Event::TurnRecorded { turn } => println!(
                        "turn {} session={} model={}",
                        turn.id,
                        turn.session,
                        turn.model.as_deref().unwrap_or("unknown")
                    ),
                }
            }
        }
        Command::Show { id } => {
            let store = Store::discover(&cwd)?;
            match store.find(&id)? {
                Some(event) => println!("{}", serde_json::to_string_pretty(&event)?),
                None => anyhow::bail!("not found: {id}"),
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
