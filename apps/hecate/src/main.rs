//! Hecate user CLI.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hecate", version, about = "Task-aware Git worktrees")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List worktrees registered for this clone (from metadata).
    List {
        /// Print machine-readable JSON.
        #[arg(long)]
        json: bool,
        /// Run as if started in this directory (must be inside a Git repo).
        #[arg(long, value_name = "DIR")]
        cwd: Option<PathBuf>,
    },
    /// Create a new branch and worktree for a task, and register it in metadata.
    Start {
        /// Task label (e.g. issue number or short slug); becomes directory name and `task/<slug>` branch.
        task: String,
        /// Run as if started in this directory (must be inside a Git repo).
        #[arg(long, value_name = "DIR")]
        cwd: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::List { json, cwd } => {
            let base = cwd
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| PathBuf::from("."));
            if let Err(e) = hecate::list::run(&base, json) {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        Command::Start { task, cwd } => {
            let base = cwd
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| PathBuf::from("."));
            if let Err(e) = hecate::start::run(&task, &base) {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }
}
