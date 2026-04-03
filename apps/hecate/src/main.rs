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
    /// Remove a linked worktree (by registered name or path) and drop metadata.
    Rm {
        /// Registered worktree name (from `hecate list`), unless `--path` is set.
        #[arg(required_unless_present = "path")]
        name: Option<String>,
        /// Remove by checkout path (must match a row in metadata for this clone).
        #[arg(long, value_name = "PATH", required_unless_present = "name")]
        path: Option<PathBuf>,
        /// Pass `--force` to `git worktree remove` (e.g. dirty tree or locked).
        #[arg(long, short)]
        force: bool,
        #[arg(long, value_name = "DIR")]
        cwd: Option<PathBuf>,
    },
    /// Show repository, branch, hecate_root, metadata path, and tracked worktree count.
    State {
        /// Print machine-readable JSON.
        #[arg(long)]
        json: bool,
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
        Command::Rm {
            name,
            path,
            force,
            cwd,
        } => {
            let base = cwd
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| PathBuf::from("."));
            if let Err(e) = hecate::rm::run(&base, name, path, force) {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        Command::State { json, cwd } => {
            let base = cwd
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| PathBuf::from("."));
            if let Err(e) = hecate::state::run(&base, json) {
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
