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

#[test]
fn list_worktrees_for_clone_after_start() {
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

    assert!(
        hecate::list::worktrees_for_cwd(repo.path())
            .unwrap()
            .is_empty()
    );

    hecate::start::run("99", repo.path()).unwrap();
    let records = hecate::list::worktrees_for_cwd(repo.path()).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].name, "99");
    assert_eq!(records[0].branch, "task/99");
    assert_eq!(records[0].task.as_deref(), Some("99"));
}

#[test]
fn rm_by_name_removes_checkout_and_metadata() {
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

    hecate::start::run("42", repo.path()).unwrap();
    let seg = hecate_core::repo_default_segment(repo.path()).unwrap();
    let wt = parking.path().join(&seg).join("42");
    assert!(wt.join("f.txt").exists());

    hecate::rm::run(repo.path(), Some("42".into()), None, false).unwrap();
    assert!(!wt.exists());
    assert!(
        hecate::list::worktrees_for_cwd(repo.path())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn state_without_hecate_root_shows_unconfigured() {
    let repo = tempfile::tempdir().unwrap();
    let isolated_home = tempfile::tempdir().unwrap();
    init_repo(repo.path());
    let s = hecate::state::gather_opts(
        repo.path(),
        hecate::state::StateOptions {
            config_home_override: Some(isolated_home.path().to_path_buf()),
            use_process_hecate_env: false,
        },
    )
    .unwrap();
    assert_eq!(s.current_branch.as_deref(), Some("main"));
    assert!(s.hecate_root_configured.is_none());
    assert!(s.hecate_root_resolved.is_none());
    assert!(s.metadata_path.is_none());
    assert_eq!(s.worktree_count, 0);
}

#[test]
fn state_counts_tracked_worktrees() {
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

    let isolated_home = tempfile::tempdir().unwrap();
    let opts = hecate::state::StateOptions {
        config_home_override: Some(isolated_home.path().to_path_buf()),
        use_process_hecate_env: false,
    };
    let before = hecate::state::gather_opts(repo.path(), opts.clone()).unwrap();
    assert_eq!(before.worktree_count, 0);
    assert_eq!(
        before.metadata_path,
        Some(hecate_config::metadata_path(parking.path()))
    );

    hecate::start::run("5", repo.path()).unwrap();
    let after = hecate::state::gather_opts(repo.path(), opts).unwrap();
    assert_eq!(after.worktree_count, 1);
}
