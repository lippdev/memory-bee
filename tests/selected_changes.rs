use memory_pier::{
    bundle::{Options, Prepared, prepare},
    claude::{Limits, inspect},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
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
            "memory-pier-code-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let repo = Self(path);
        repo.git(&["init", "-b", "fixture"]);
        repo.git(&["config", "user.name", "Synthetic"]);
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
            "git {:?}: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().into()
    }
    fn write(&self, path: &str, bytes: impl AsRef<[u8]>) {
        fs::write(self.0.join(path), bytes).unwrap();
    }
    fn commit(&self) {
        self.git(&["add", "."]);
        self.git(&["commit", "--allow-empty", "-m", "synthetic"]);
    }
    fn prepare(&self, paths: &[&str]) -> Result<Prepared, String> {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/claude/basic.jsonl");
        prepare(
            inspect(&source, Limits::default()).unwrap(),
            &Options {
                project: Some(self.0.clone()),
                include_paths: paths.iter().map(|p| (*p).into()).collect(),
                ..Options::default()
            },
        )
    }
}
impl Drop for Repo {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn value(prepared: &Prepared) -> Value {
    serde_json::to_value(prepared).unwrap()
}

#[test]
fn selected_patch_and_new_files_reconstruct_bytes_without_touching_source() {
    let repo = Repo::new();
    for (path, content) in [
        ("tracked.txt", "base\n"),
        ("deleted.txt", "removed\n"),
        ("empty.txt", ""),
        ("space name.txt", "old without newline"),
        ("unselected.txt", "original\n"),
    ] {
        repo.write(path, content);
    }
    repo.commit();
    repo.write("tracked.txt", "staged\n");
    repo.git(&["add", "tracked.txt"]);
    repo.write("tracked.txt", "current\n");
    repo.write("space name.txt", "new\r\nnext");
    repo.write("empty.txt", "first\n");
    repo.write("new.txt", "new file\n");
    repo.write("new-empty.txt", "");
    repo.write("unselected.txt", "not exported\n");
    fs::remove_file(repo.0.join("deleted.txt")).unwrap();
    let index = fs::read(repo.0.join(".git/index")).unwrap();
    let prepared = repo
        .prepare(&[
            "tracked.txt",
            "deleted.txt",
            "empty.txt",
            "space name.txt",
            "new.txt",
            "new-empty.txt",
        ])
        .unwrap();
    assert!(!prepared.is_partial());
    assert!(prepared.findings().is_empty());
    let v = value(&prepared);
    assert_eq!(v["manifest"]["format_version"], 2);
    assert_eq!(v["manifest"]["code_state"], "changes-included");
    assert!(
        !v["code"]["patch"]
            .as_str()
            .unwrap()
            .contains("unselected.txt")
    );
    assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), index);
    assert_eq!(
        fs::read_to_string(repo.0.join("tracked.txt")).unwrap(),
        "current\n"
    );
    let out = repo.0.join("bundle");
    prepared.write(&out).unwrap();
    let target = repo.0.join("target");
    repo.git(&[
        "clone",
        "--no-hardlinks",
        repo.0.to_str().unwrap(),
        target.to_str().unwrap(),
    ]);
    fs::write(target.join("tracked.txt"), "conflicting destination\n").unwrap();
    let conflict = Command::new("git")
        .arg("-C")
        .arg(&target)
        .args(["apply", "--check"])
        .arg(out.join("changes.patch"))
        .output()
        .unwrap();
    assert!(!conflict.status.success());
    assert_eq!(
        fs::read_to_string(target.join("tracked.txt")).unwrap(),
        "conflicting destination\n"
    );
    fs::write(target.join("tracked.txt"), "base\n").unwrap();
    for args in [vec!["apply", "--check"], vec!["apply"]] {
        let result = Command::new("git")
            .arg("-C")
            .arg(&target)
            .args(args)
            .arg(out.join("changes.patch"))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    for change in v["manifest"]["changes"].as_array().unwrap() {
        let path = change["path"].as_str().unwrap();
        if change["kind"] == "add" {
            fs::copy(
                out.join(change["payload"].as_str().unwrap()),
                target.join(path),
            )
            .unwrap();
        }
        if change["kind"] == "delete" {
            assert!(!target.join(path).exists());
        } else {
            let actual = fs::read(target.join(path)).unwrap();
            assert_eq!(actual, fs::read(repo.0.join(path)).unwrap());
            assert_eq!(
                change["result_sha256"],
                format!("{:x}", Sha256::digest(actual))
            );
        }
    }
    for payload in v["manifest"]["files"].as_array().unwrap() {
        assert_eq!(
            payload["sha256"],
            format!(
                "{:x}",
                Sha256::digest(fs::read(out.join(payload["path"].as_str().unwrap())).unwrap())
            )
        );
    }
    assert_eq!(
        fs::read_to_string(target.join("unselected.txt")).unwrap(),
        "original\n"
    );
    assert!(prepared.write(&out).is_err());
}

#[test]
fn binary_non_utf8_oversize_and_unchanged_are_explicit_omissions() {
    let repo = Repo::new();
    repo.write("unchanged", "same\n");
    repo.write("binary", [0, 1]);
    repo.commit();
    repo.write("binary", [0, 2]);
    repo.write("invalid", [255]);
    repo.write("large", vec![b'a'; 1024 * 1024 + 1]);
    let prepared = repo
        .prepare(&["binary", "invalid", "large", "unchanged"])
        .unwrap();
    let v = value(&prepared);
    assert!(prepared.is_partial());
    assert_eq!(v["manifest"]["code_state"], "base-reference");
    assert!(v["manifest"]["changes"].as_array().unwrap().is_empty());
    for reason in [
        "binary_unsupported",
        "non_utf8_unsupported",
        "file_limit",
        "sem diferença",
    ] {
        assert!(v["manifest"]["omissions"].to_string().contains(reason));
    }
    let out = repo.0.join("bundle");
    prepared.write(&out).unwrap();
    assert!(!out.join("files").exists());
    assert!(!out.join("changes.patch").exists());
}

#[test]
fn invalid_paths_missing_paths_and_unborn_base_fail_without_export() {
    let repo = Repo::new();
    repo.write("new", "text");
    assert!(repo.prepare(&["new"]).is_err());
    repo.commit();
    for path in [
        "../escape",
        "/absolute",
        ".git/config",
        "nested/../file",
        ":(glob)*",
        "a\\b",
        "a//b",
        "./new",
        "a/",
        "missing",
        "unicodé",
    ] {
        assert!(repo.prepare(&[path]).is_err(), "{path}");
    }
}

#[cfg(unix)]
#[test]
fn symlinks_directories_and_nested_repositories_are_not_followed() {
    use std::os::unix::fs::symlink;
    let repo = Repo::new();
    repo.commit();
    repo.write("real", "safe");
    symlink(repo.0.join("real"), repo.0.join("link")).unwrap();
    fs::create_dir(repo.0.join("directory")).unwrap();
    symlink(repo.0.join("directory"), repo.0.join("parent")).unwrap();
    fs::create_dir(repo.0.join("nested")).unwrap();
    fs::create_dir(repo.0.join("nested/.git")).unwrap();
    repo.write("nested/file", "private");
    let p = repo
        .prepare(&["link", "parent/file", "directory", "nested/file"])
        .unwrap();
    assert!(p.is_partial());
    assert!(
        value(&p)["manifest"]["changes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn removed_secrets_and_new_secrets_block_writes_without_echo_in_cli() {
    let repo = Repo::new();
    let token = format!("ghp_{}", "syntheticfixture".repeat(3));
    repo.write("old", &token);
    repo.commit();
    repo.write("old", "sanitized\n");
    repo.write("new", &token);
    let p = repo.prepare(&["old", "new"]).unwrap();
    assert!(!p.findings().is_empty());
    assert!(p.findings().iter().all(|f| f.selection.is_some()));
    assert!(p.write(&repo.0.join("blocked")).is_err());
    let result = Command::new(env!("CARGO_BIN_EXE_memory-pier"))
        .args(["export", "testdata/claude/basic.jsonl", "--project"])
        .arg(&repo.0)
        .args(["--include-path", "old", "--output"])
        .arg(repo.0.join("blocked"))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(3));
    assert!(!String::from_utf8_lossy(&result.stdout).contains(&token));
    assert!(!repo.0.join("blocked").exists());
}

#[cfg(unix)]
#[test]
fn mode_only_and_empty_deletion_patches_apply_and_payloads_stay_private() {
    use std::os::unix::fs::PermissionsExt;
    let repo = Repo::new();
    repo.write("mode", "same\n");
    repo.write("empty", "");
    repo.commit();
    fs::set_permissions(repo.0.join("mode"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::remove_file(repo.0.join("empty")).unwrap();
    repo.write("new", "new\n");
    let p = repo.prepare(&["mode", "empty", "new"]).unwrap();
    let out = repo.0.join("bundle");
    p.write(&out).unwrap();
    let target = repo.0.join("target");
    repo.git(&["clone", repo.0.to_str().unwrap(), target.to_str().unwrap()]);
    let result = Command::new("git")
        .arg("-C")
        .arg(&target)
        .arg("apply")
        .arg(out.join("changes.patch"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!target.join("empty").exists());
    assert_ne!(
        fs::metadata(target.join("mode"))
            .unwrap()
            .permissions()
            .mode()
            & 0o111,
        0
    );
    assert_eq!(
        fs::metadata(out.join("files"))
            .unwrap()
            .permissions()
            .mode()
            & 0o077,
        0
    );
    for f in fs::read_dir(out.join("files")).unwrap() {
        assert_eq!(
            f.unwrap().metadata().unwrap().permissions().mode() & 0o077,
            0
        );
    }
}

#[test]
fn staged_new_files_and_literal_names_are_selected_by_working_tree() {
    let repo = Repo::new();
    repo.commit();
    repo.write("-name.txt", "current\n");
    repo.git(&["add", "--", "-name.txt"]);
    let v = value(&repo.prepare(&["-name.txt"]).unwrap());
    assert_eq!(v["manifest"]["changes"][0]["kind"], "add");
    assert_eq!(v["code"]["new_files"][0]["content"], "current\n");
}

#[cfg(unix)]
#[test]
fn configured_clean_filters_are_not_executed_by_capture_or_status() {
    let repo = Repo::new();
    repo.write("tracked", "base\n");
    repo.commit();
    repo.write(".gitattributes", "tracked filter=synthetic\n");
    repo.write("tracked", "current\n");
    repo.git(&[
        "config",
        "filter.synthetic.clean",
        "touch FILTER_EXECUTED; cat",
    ]);
    repo.git(&["config", "filter.synthetic.required", "true"]);
    let p = repo.prepare(&["tracked"]).unwrap();
    assert!(!repo.0.join("FILTER_EXECUTED").exists());
    assert!(
        value(&p)["code"]["patch"]
            .as_str()
            .unwrap()
            .contains("+current")
    );
}

#[test]
fn conflicts_are_omitted_and_cli_requires_explicit_project() {
    let repo = Repo::new();
    repo.write("conflict", "base\n");
    repo.commit();
    repo.write("conflict", "one\n");
    repo.commit();
    repo.git(&["checkout", "-b", "other", "HEAD~1"]);
    repo.write("conflict", "other\n");
    repo.commit();
    assert!(
        !Command::new("git")
            .arg("-C")
            .arg(&repo.0)
            .args(["merge", "fixture"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let p = repo.prepare(&["conflict"]).unwrap();
    assert!(p.is_partial());
    assert!(
        value(&p)["manifest"]["omissions"]
            .to_string()
            .contains("conflict_unsupported")
    );
    let result = Command::new(env!("CARGO_BIN_EXE_memory-pier"))
        .args([
            "export",
            "testdata/claude/basic.jsonl",
            "--include-path",
            "file",
            "--preview",
        ])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(64));
}

#[test]
fn count_and_total_byte_budgets_are_enforced() {
    let repo = Repo::new();
    repo.commit();
    let names: Vec<_> = (0..65).map(|i| format!("file-{i:02}")).collect();
    assert!(
        repo.prepare(&names.iter().map(String::as_str).collect::<Vec<_>>())
            .is_err()
    );
    for name in names.iter().take(9) {
        repo.write(name, vec![b'a'; 1024 * 1024]);
    }
    let p = repo
        .prepare(&names.iter().take(9).map(String::as_str).collect::<Vec<_>>())
        .unwrap();
    assert!(p.is_partial());
    let v = value(&p);
    assert_eq!(v["manifest"]["changes"].as_array().unwrap().len(), 8);
    assert!(
        v["manifest"]["omissions"]
            .to_string()
            .contains("total_limit")
    );
}

#[cfg(unix)]
#[test]
fn submodule_filters_are_not_executed_and_submodules_are_reported() {
    let child = Repo::new();
    child.write("tracked", "base\n");
    child.commit();
    let repo = Repo::new();
    repo.write("root", "base\n");
    repo.commit();
    repo.git(&[
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        child.0.to_str().unwrap(),
        "module",
    ]);
    repo.commit();
    repo.write("module/.gitattributes", "tracked filter=nested\n");
    repo.write("module/tracked", "changed\n");
    repo.git(&[
        "-C",
        "module",
        "config",
        "filter.nested.clean",
        "touch FILTER_EXECUTED; cat",
    ]);
    repo.write("root", "selected\n");
    let p = repo.prepare(&["root", "module"]).unwrap();
    assert!(p.is_partial());
    assert!(!repo.0.join("module/FILTER_EXECUTED").exists());
    assert!(
        value(&p)["manifest"]["omissions"]
            .to_string()
            .contains("base_type_unsupported")
    );
    assert!(
        value(&p)["manifest"]["warnings"]
            .to_string()
            .contains("git_submodules_unverified")
    );
}
