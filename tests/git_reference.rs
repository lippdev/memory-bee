use memory_pier::{
    bundle::{Options, prepare},
    claude::{Limits, inspect},
    git,
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Repo(PathBuf);
impl Repo {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "memory-pier-git-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let repo = Self(path);
        repo.git(&["init", "-b", "fixture"]);
        repo.git(&["config", "user.name", "Synthetic Test"]);
        repo.git(&["config", "user.email", "synthetic@example.invalid"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo.git(&["config", "core.hooksPath", "/dev/null"]);
        repo
    }
    fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.0)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git test command failed: {:?}",
            args
        );
        String::from_utf8(output.stdout).unwrap().trim().into()
    }
    fn commit(&self) -> String {
        fs::write(self.0.join("tracked.txt"), "synthetic base\n").unwrap();
        self.git(&["add", "."]);
        self.git(&["commit", "-m", "synthetic base"]);
        self.git(&["rev-parse", "HEAD"])
    }
}
impl Drop for Repo {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn bundle(path: &Path) -> Value {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/claude/basic.jsonl");
    let prepared = prepare(
        inspect(&source, Limits::default()).unwrap(),
        &Options {
            project: Some(path.into()),
            ..Options::default()
        },
    )
    .unwrap();
    serde_json::to_value(prepared).unwrap()
}
#[test]
fn clean_reference_from_subdirectory_preserves_index_and_source() {
    let repo = Repo::new();
    let commit = repo.commit();
    repo.git(&[
        "remote",
        "add",
        "origin",
        "https://example.invalid/team/project.git",
    ]);
    fs::create_dir(repo.0.join("nested")).unwrap();
    let index = fs::read(repo.0.join(".git/index")).unwrap();
    let value = bundle(&repo.0.join("nested"));
    assert_eq!(value["manifest"]["code_state"], "base-reference");
    assert_eq!(value["manifest"]["project"]["base_commit"], commit);
    assert_eq!(value["manifest"]["project"]["branch"], "fixture");
    assert_eq!(value["manifest"]["project"]["dirty"], false);
    assert_eq!(
        value["manifest"]["project"]["remote"],
        "https://example.invalid/team/project.git"
    );
    assert!(value["handoff"].as_str().unwrap().contains(&commit));
    assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), index);
    assert_eq!(
        fs::read_to_string(repo.0.join("tracked.txt")).unwrap(),
        "synthetic base\n"
    );
    assert!(!value.to_string().contains(repo.0.to_str().unwrap()));
}
#[test]
fn dirty_detects_untracked_staged_unstaged_and_conflicts() {
    let repo = Repo::new();
    repo.commit();
    fs::write(repo.0.join("new.txt"), "synthetic\n").unwrap();
    assert_eq!(git::inspect(&repo.0).project.dirty, Some(true));
    fs::remove_file(repo.0.join("new.txt")).unwrap();
    fs::write(repo.0.join("tracked.txt"), "change\n").unwrap();
    assert_eq!(git::inspect(&repo.0).project.dirty, Some(true));
    repo.git(&["add", "tracked.txt"]);
    assert_eq!(git::inspect(&repo.0).project.dirty, Some(true));
    let value = bundle(&repo.0);
    assert!(value["handoff"].as_str().unwrap().contains("não incluídas"));
    assert_eq!(value["manifest"]["files"].as_array().unwrap().len(), 2);
    repo.git(&["commit", "-m", "one"]);
    repo.git(&["checkout", "-b", "other", "HEAD~1"]);
    fs::write(repo.0.join("tracked.txt"), "other\n").unwrap();
    repo.git(&["commit", "-am", "other"]);
    let result = Command::new("git")
        .arg("-C")
        .arg(&repo.0)
        .args(["merge", "fixture"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert_eq!(git::inspect(&repo.0).project.dirty, Some(true));
}
#[test]
fn detached_and_unborn_heads_have_distinct_base_states() {
    let repo = Repo::new();
    let unborn = bundle(&repo.0);
    assert_eq!(unborn["manifest"]["code_state"], "unknown");
    assert_eq!(unborn["manifest"]["project"]["branch"], "fixture");
    assert_eq!(unborn["partial"], true);
    let commit = repo.commit();
    repo.git(&["checkout", "--detach"]);
    let detached = bundle(&repo.0);
    assert_eq!(detached["manifest"]["project"]["base_commit"], commit);
    assert!(detached["manifest"]["project"]["branch"].is_null());
    assert_eq!(detached["partial"], false);
    assert!(
        detached["handoff"]
            .as_str()
            .unwrap()
            .contains("git_detached_head")
    );
}
#[test]
fn missing_non_repository_and_bare_return_unknown_without_paths() {
    let repo = Repo::new();
    for path in [repo.0.join("missing"), repo.0.join(".git")] {
        let value = bundle(&path);
        assert_eq!(value["manifest"]["code_state"], "unknown");
        assert_eq!(value["partial"], true);
        assert!(!value.to_string().contains(path.to_str().unwrap()));
    }
    fs::remove_dir_all(repo.0.join(".git")).unwrap();
    assert!(git::inspect(&repo.0).partial);
}
#[test]
fn remote_credentials_local_paths_and_queries_are_omitted() {
    let repo = Repo::new();
    repo.commit();
    for remote in [
        "https://user:synthetic@example.invalid/repo",
        "https://example.invalid/repo?token=synthetic",
        "/private/synthetic/repo",
        "file:///private/synthetic/repo",
        "ssh://user@example.invalid/repo",
    ] {
        repo.git(&["config", "remote.origin.url", remote]);
        let value = bundle(&repo.0);
        assert!(value["manifest"]["project"]["remote"].is_null());
        assert!(!value.to_string().contains(remote));
    }
    repo.git(&[
        "config",
        "remote.origin.url",
        "git@example.invalid:team/repo.git",
    ]);
    assert_eq!(
        git::inspect(&repo.0).project.remote.as_deref(),
        Some("ssh://example.invalid/team/repo.git")
    );
}
#[test]
fn git_metadata_is_subject_to_secret_detection() {
    let repo = Repo::new();
    repo.commit();
    let branch = format!("ghp_{}", "syntheticfixture".repeat(3));
    repo.git(&["checkout", "-b", &branch]);
    let value = bundle(&repo.0);
    assert!(
        value["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["field"] == "project.branch")
    );
    let output = Command::new(env!("CARGO_BIN_EXE_memory-pier"))
        .args(["export", "testdata/claude/basic.jsonl", "--project"])
        .arg(&repo.0)
        .arg("--output")
        .arg(repo.0.join("blocked"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(!repo.0.join("blocked").exists());
    assert!(!String::from_utf8_lossy(&output.stdout).contains(&branch));
}
#[test]
fn cli_partial_export_and_explicit_project_override_git_environment() {
    let repo = Repo::new();
    let commit = repo.commit();
    let result = Command::new(env!("CARGO_BIN_EXE_memory-pier"))
        .args([
            "export",
            "testdata/claude/basic.jsonl",
            "--preview",
            "--project",
        ])
        .arg(&repo.0)
        .env("GIT_DIR", "/synthetic/missing")
        .env("GIT_WORK_TREE", "/synthetic/missing")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["manifest"]["project"]["base_commit"], commit);
    let destination = repo.0.join("bundle");
    let result = Command::new(env!("CARGO_BIN_EXE_memory-pier"))
        .args(["export", "testdata/claude/basic.jsonl", "--project"])
        .arg(repo.0.join("missing"))
        .arg("--output")
        .arg(&destination)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(destination.join("manifest.json").exists());
}

#[test]
fn corrupt_index_and_missing_git_are_partial_not_clean() {
    let repo = Repo::new();
    repo.commit();
    fs::write(repo.0.join(".git/index"), "synthetic corrupt index").unwrap();
    let observation = git::inspect(&repo.0);
    assert!(observation.partial);
    assert!(observation.project.dirty.is_none());
    assert!(observation.project.base_commit.is_some());
    let result = Command::new(env!("CARGO_BIN_EXE_memory-pier"))
        .args([
            "export",
            "testdata/claude/basic.jsonl",
            "--preview",
            "--project",
        ])
        .arg(&repo.0)
        .env("PATH", repo.0.join("no-executables"))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    let value: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["manifest"]["code_state"], "unknown");
}

#[test]
fn linked_worktree_uses_its_own_head_and_status() {
    let repo = Repo::new();
    let commit = repo.commit();
    let linked = repo.0.join("linked");
    repo.git(&["worktree", "add", "-b", "linked", linked.to_str().unwrap()]);
    let observation = git::inspect(&linked);
    assert_eq!(
        observation.project.base_commit.as_deref(),
        Some(commit.as_str())
    );
    assert_eq!(observation.project.branch.as_deref(), Some("linked"));
    assert_eq!(observation.project.dirty, Some(false));
    fs::write(linked.join("new.txt"), "synthetic").unwrap();
    assert_eq!(git::inspect(&linked).project.dirty, Some(true));
}
