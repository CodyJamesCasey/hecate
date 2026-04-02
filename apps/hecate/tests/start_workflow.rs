//! End-to-end `hecate start` against a real git repo (requires `git` on PATH).

use std::fs;
use std::path::Path;
use std::process::Command;

fn init_repo(path: &Path) {
    assert!(
        Command::new("git")
            .current_dir(path)
            .args(["init", "-b", "main"])
            .status()
            .expect("git")
            .success(),
        "git init failed — is git installed?"
    );
    for (k, v) in [("user.email", "t@e.co"), ("user.name", "t")] {
        Command::new("git")
            .current_dir(path)
            .args(["config", k, v])
            .status()
            .unwrap();
    }
    fs::write(path.join("f.txt"), "x\n").unwrap();
    Command::new("git")
        .current_dir(path)
        .args(["add", "f.txt"])
        .status()
        .unwrap();
    Command::new("git")
        .current_dir(path)
        .args(["commit", "-m", "i"])
        .status()
        .unwrap();
}

#[test]
fn start_registers_worktree_and_metadata() {
    let parking = tempfile::tempdir().unwrap();
    let repo = tempfile::tempdir().unwrap();
    init_repo(repo.path());

    let hecate_dir = repo.path().join(".hecate");
    fs::create_dir_all(&hecate_dir).unwrap();
    let root_str = parking.path().to_string_lossy().replace('\\', "/");
    fs::write(
        hecate_dir.join("config.toml"),
        format!("hecate_root = \"{root_str}\"\n"),
    )
    .unwrap();

    hecate::start::run("77", repo.path()).expect("start");

    let seg = hecate_core::repo_default_segment(repo.path()).unwrap();
    let wt = parking.path().join(&seg).join("77");
    assert!(wt.join("f.txt").exists(), "worktree checkout missing");

    let meta = hecate_config::read_metadata(parking.path()).unwrap();
    let key = hecate_config::clone_identity_key(repo.path());
    let list = meta.repos.get(&key).expect("repo key");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].branch, "task/77");
    assert_eq!(list[0].name, "77");
    assert_eq!(list[0].task.as_deref(), Some("77"));
}
