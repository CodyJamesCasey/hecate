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
