//! Bounded bundle verification and explicit application. Imported text is never executed.
use crate::{
    changes::{self, Snapshot},
    git,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

type Result<T> = std::result::Result<T, String>;
const MANIFEST_LIMIT: usize = 1024 * 1024;
const PAYLOAD_LIMIT: usize = 128 * 1024 * 1024;
const TOTAL_LIMIT: usize = 160 * 1024 * 1024;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format_version: u8,
    created_at: String,
    source: Origin,
    project: Project,
    code_state: String,
    files: Vec<Payload>,
    omissions: Vec<String>,
    warnings: Vec<String>,
    redaction: String,
    selected_paths: Option<Vec<String>>,
    changes: Option<Vec<Change>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Origin {
    agent: String,
    version: Option<String>,
    session_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Project {
    remote: Option<String>,
    branch: Option<String>,
    base_commit: Option<String>,
    dirty: Option<bool>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    path: String,
    sha256: String,
    purpose: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Change {
    path: String,
    kind: String,
    payload: String,
    base_sha256: Option<String>,
    result_sha256: Option<String>,
    base_mode: Option<String>,
    result_mode: Option<String>,
}
struct Decoded {
    path: String,
    before: Option<Snapshot>,
    after: Option<Snapshot>,
    mode_only: bool,
}
/// All bytes used for application are owned and verified, independent of later package changes.
pub struct Verified {
    manifest: Manifest,
    decoded: Vec<Decoded>,
}
#[derive(Serialize)]
pub struct Report {
    pub valid: bool,
    pub format_version: u8,
    pub payloads: usize,
    pub changes: usize,
    pub omissions: usize,
    pub warnings: usize,
    pub redaction: String,
}
#[derive(Debug, Serialize)]
pub struct Application {
    pub checked: bool,
    pub written: bool,
    pub changes: usize,
    pub omissions: usize,
    pub warnings: usize,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hex(value: &str, sizes: &[usize]) -> bool {
    sizes.contains(&value.len())
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn require(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
fn required(value: &Value, names: &[&str]) -> Result<()> {
    let obj = value.as_object().ok_or("expected manifest object")?;
    require(
        names.iter().all(|n| obj.contains_key(*n)),
        "missing required manifest field",
    )
}
fn read_file(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "bundle file unavailable")?;
    require(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "bundle requires regular files, no symlinks",
    )?;
    require(metadata.len() <= limit as u64, "bundle size limit exceeded")?;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| "bundle file unavailable")?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "bundle read failed")?;
    require(bytes.len() <= limit, "bundle size limit exceeded")?;
    Ok(bytes)
}
fn new_payload(path: &str) -> bool {
    path.len() == 14
        && path.starts_with("files/")
        && path.ends_with(".txt")
        && path.as_bytes()[6..10].iter().all(u8::is_ascii_digit)
}
fn collision_free(paths: &[String]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for path in paths {
        require(changes::valid_path(path), "unsafe or unsupported path")?;
        require(
            seen.insert(path.to_ascii_lowercase()),
            "duplicate or case-colliding path",
        )?;
    }
    for path in &seen {
        let mut parent = path.as_str();
        while let Some((p, _)) = parent.rsplit_once('/') {
            require(!seen.contains(p), "file/directory path collision")?;
            parent = p;
        }
    }
    Ok(())
}

pub fn verify(directory: &Path) -> Result<Verified> {
    let meta = fs::symlink_metadata(directory).map_err(|_| "bundle directory unavailable")?;
    require(
        meta.is_dir() && !meta.file_type().is_symlink(),
        "bundle root must be a directory, not a symlink",
    )?;
    let bytes = read_file(&directory.join("manifest.json"), MANIFEST_LIMIT)?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "invalid manifest JSON")?;
    required(
        &value,
        &[
            "format_version",
            "created_at",
            "source",
            "project",
            "code_state",
            "files",
            "omissions",
            "warnings",
            "redaction",
        ],
    )?;
    required(&value["source"], &["agent", "version", "session_id"])?;
    required(
        &value["project"],
        &["remote", "branch", "base_commit", "dirty"],
    )?;
    let version = value["format_version"]
        .as_u64()
        .ok_or("invalid format version")?;
    require(version == 1 || version == 2, "unsupported bundle version")?;
    if version == 1 {
        require(
            value.get("selected_paths").is_none() && value.get("changes").is_none(),
            "v1 cannot contain v2 fields",
        )?;
    } else {
        required(&value, &["selected_paths", "changes"])?;
    }
    let manifest: Manifest =
        serde_json::from_slice(&bytes).map_err(|_| "invalid manifest fields")?;
    require(
        chrono::DateTime::parse_from_rfc3339(&manifest.created_at).is_ok(),
        "invalid creation timestamp",
    )?;
    require(!manifest.source.agent.is_empty(), "empty source agent")?;
    require(
        ["pending-review", "reviewed"].contains(&manifest.redaction.as_str()),
        "invalid redaction state",
    )?;
    require(
        ["base-reference", "changes-included", "unknown"].contains(&manifest.code_state.as_str()),
        "invalid code state",
    )?;
    if let Some(commit) = &manifest.project.base_commit {
        require(hex(commit, &[40, 64]), "invalid base commit")?;
    }
    // Nullable metadata is parsed for type validation but is never used as authority.
    let _ = (
        &manifest.source.version,
        &manifest.source.session_id,
        &manifest.project.remote,
        &manifest.project.branch,
        manifest.project.dirty,
    );
    require(
        (2..=67).contains(&manifest.files.len()),
        "invalid payload count",
    )?;
    let mut payloads = BTreeMap::new();
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(directory).map_err(|_| "bundle enumeration failed")? {
        let entry = entry.map_err(|_| "bundle enumeration failed")?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "unsupported bundle filename")?;
        if name == "files" {
            require(
                entry
                    .file_type()
                    .map_err(|_| "bundle metadata failed")?
                    .is_dir(),
                "files must be a real directory",
            )?;
            for file in fs::read_dir(entry.path()).map_err(|_| "bundle enumeration failed")? {
                let file = file.map_err(|_| "bundle enumeration failed")?;
                let name = file
                    .file_name()
                    .into_string()
                    .map_err(|_| "unsupported payload filename")?;
                require(actual.insert(format!("files/{name}")), "duplicate payload")?;
                require(actual.len() <= 67, "payload count limit")?;
            }
        } else if name != "manifest.json" {
            actual.insert(name);
        }
        require(actual.len() <= 67, "payload count limit")?;
    }
    let mut total = 0;
    for file in &manifest.files {
        let allowed = match file.purpose.as_str() {
            "entrypoint" => file.path == "HANDOFF.md",
            "history" => file.path == "history.jsonl",
            "patch" => version == 2 && file.path == "changes.patch",
            "new-file" => version == 2 && new_payload(&file.path),
            _ => false,
        };
        require(
            allowed && hex(&file.sha256, &[64]),
            "invalid payload path, purpose or hash",
        )?;
        require(!payloads.contains_key(&file.path), "duplicate payload path")?;
        let limit = match file.purpose.as_str() {
            "new-file" => 1024 * 1024,
            "patch" => 17 * 1024 * 1024,
            _ => PAYLOAD_LIMIT,
        };
        let bytes = read_file(&directory.join(&file.path), limit.min(TOTAL_LIMIT - total))?;
        total += bytes.len();
        require(hash(&bytes) == file.sha256, "payload hash mismatch")?;
        let text = String::from_utf8(bytes).map_err(|_| "non UTF-8 payload")?;
        payloads.insert(file.path.clone(), text);
    }
    require(
        payloads.keys().cloned().collect::<BTreeSet<_>>() == actual,
        "unlisted or missing bundle payload",
    )?;
    require(
        payloads.contains_key("HANDOFF.md") && payloads.contains_key("history.jsonl"),
        "missing context payload",
    )?;
    let history = &payloads["history.jsonl"];
    for line in history.lines() {
        let event: Value = serde_json::from_str(line).map_err(|_| "invalid history JSONL")?;
        require(event.is_object(), "history event must be an object")?;
    }
    let mut decoded = Vec::new();
    if version == 2 {
        require(
            manifest.project.base_commit.is_some(),
            "v2 requires a base commit",
        )?;
        let selected = manifest
            .selected_paths
            .as_ref()
            .ok_or("v2 requires selected paths")?;
        require((1..=64).contains(&selected.len()), "selection count limit")?;
        collision_free(selected)?;
        let changes = manifest.changes.as_ref().ok_or("v2 requires changes")?;
        require(changes.len() <= 64, "change count limit")?;
        require(
            manifest.code_state
                == if changes.is_empty() {
                    "base-reference"
                } else {
                    "changes-included"
                },
            "code state inconsistent with changes",
        )?;
        collision_free(&changes.iter().map(|c| c.path.clone()).collect::<Vec<_>>())?;
        let mut used = BTreeSet::from(["HANDOFF.md".to_string(), "history.jsonl".to_string()]);
        let mut expected_patch = String::new();
        let patch = payloads
            .get("changes.patch")
            .map(String::as_str)
            .unwrap_or("");
        let mut remaining = patch;
        let mut code_bytes = 0;
        for (index, change) in changes.iter().enumerate() {
            required(
                &value["changes"][index],
                &[
                    "path",
                    "kind",
                    "payload",
                    "base_sha256",
                    "result_sha256",
                    "base_mode",
                    "result_mode",
                ],
            )?;
            require(selected.contains(&change.path), "change not selected")?;
            for h in [&change.base_sha256, &change.result_sha256]
                .into_iter()
                .flatten()
            {
                require(hex(h, &[64]), "invalid change hash")?;
            }
            for m in [&change.base_mode, &change.result_mode]
                .into_iter()
                .flatten()
            {
                require(
                    ["100644", "100755"].contains(&m.as_str()),
                    "invalid file mode",
                )?;
            }
            let has_base = change.base_sha256.is_some() && change.base_mode.is_some();
            let has_result = change.result_sha256.is_some() && change.result_mode.is_some();
            let mut mode_only = false;
            let (before, after) = match change.kind.as_str() {
                "add" => {
                    require(
                        change.base_sha256.is_none()
                            && change.base_mode.is_none()
                            && has_result
                            && new_payload(&change.payload),
                        "invalid addition mapping",
                    )?;
                    require(used.insert(change.payload.clone()), "new payload reused")?;
                    let content = payloads
                        .get(&change.payload)
                        .ok_or("missing new file")?
                        .clone();
                    (
                        None,
                        Some(Snapshot {
                            content,
                            mode: change.result_mode.clone().unwrap(),
                        }),
                    )
                }
                "modify" | "delete" => {
                    let delete = change.kind == "delete";
                    require(
                        has_base
                            && change.payload == "changes.patch"
                            && if delete {
                                change.result_sha256.is_none() && change.result_mode.is_none()
                            } else {
                                has_result
                            },
                        "invalid patch mapping",
                    )?;
                    used.insert("changes.patch".into());
                    let end = remaining
                        .get(1..)
                        .and_then(|s| s.find("\ndiff --git "))
                        .map(|n| n + 2)
                        .unwrap_or(remaining.len());
                    let fragment = &remaining[..end];
                    remaining = &remaining[end..];
                    let (old, new, no_hunk) = decode_hunk(fragment)?;
                    mode_only = no_hunk && !delete;
                    if mode_only {
                        require(
                            change.base_sha256 == change.result_sha256
                                && change.base_mode != change.result_mode,
                            "invalid mode-only change",
                        )?;
                    }
                    let before = Snapshot {
                        content: old,
                        mode: change.base_mode.clone().unwrap(),
                    };
                    let after = (!delete).then(|| Snapshot {
                        content: new,
                        mode: change.result_mode.clone().unwrap(),
                    });
                    let canonical = changes::patch(&change.path, &before, after.as_ref());
                    require(
                        canonical == fragment,
                        "unsupported or inconsistent patch structure",
                    )?;
                    expected_patch.push_str(&canonical);
                    (Some(before), after)
                }
                _ => return Err("unsupported change kind".into()),
            };
            for (snapshot, expected) in [
                (&before, &change.base_sha256),
                (&after, &change.result_sha256),
            ] {
                if let Some(snapshot) = snapshot {
                    require(
                        snapshot.content.len() <= 1024 * 1024 && !snapshot.content.contains('\0'),
                        "unsupported code payload or size",
                    )?;
                    code_bytes += snapshot.content.len();
                    if !mode_only {
                        require(
                            Some(hash(snapshot.content.as_bytes())).as_ref() == expected.as_ref(),
                            "change content hash mismatch",
                        )?;
                    }
                }
            }
            require(code_bytes <= 8 * 1024 * 1024, "code size limit")?;
            decoded.push(Decoded {
                path: change.path.clone(),
                before,
                after,
                mode_only,
            });
        }
        require(
            expected_patch == patch && remaining.is_empty(),
            "unmapped patch content",
        )?;
        require(
            used == payloads.keys().cloned().collect(),
            "unmapped payload",
        )?;
    } else {
        require(
            manifest.code_state != "changes-included",
            "v1 code application unsupported",
        )?;
    }
    Ok(Verified { manifest, decoded })
}

// Decode only the full replacement dialect emitted by Memory Pier. Canonical re-encoding
// checks every header, path, count and marker; no received patch is passed to Git.
fn decode_hunk(fragment: &str) -> Result<(String, String, bool)> {
    let Some((_, body)) = fragment.split_once("\n@@ ") else {
        return Ok((String::new(), String::new(), true));
    };
    let (_, body) = body.split_once('\n').ok_or("invalid patch hunk")?;
    let mut old = String::new();
    let mut new = String::new();
    let mut last = None;
    for line in body.split_inclusive('\n') {
        require(line.ends_with('\n'), "unterminated patch line")?;
        if line == "\\ No newline at end of file\n" {
            let target = match last.take() {
                Some('-') => &mut old,
                Some('+') => &mut new,
                _ => return Err("invalid newline marker".into()),
            };
            require(target.pop() == Some('\n'), "invalid newline marker")?;
        } else if let Some(text) = line.strip_prefix('-') {
            old.push_str(text);
            last = Some('-');
        } else if let Some(text) = line.strip_prefix('+') {
            new.push_str(text);
            last = Some('+');
        } else {
            return Err("unsupported patch line".into());
        }
    }
    Ok((old, new, false))
}

impl Verified {
    pub fn report(&self) -> Report {
        Report {
            valid: true,
            format_version: self.manifest.format_version,
            payloads: self.manifest.files.len(),
            changes: self.decoded.len(),
            omissions: self.manifest.omissions.len(),
            warnings: self.manifest.warnings.len(),
            redaction: self.manifest.redaction.clone(),
        }
    }
    pub fn check(&self, project: &Path) -> Result<Plan> {
        require(
            self.manifest.format_version == 2 && !self.decoded.is_empty(),
            "application requires v2 with included changes",
        )?;
        #[cfg(not(unix))]
        return Err("application currently requires Unix file modes".into());
        let root = git::text(project, &["rev-parse", "--show-toplevel"])
            .map_err(|_| "target Git root unavailable")?;
        let root = fs::canonicalize(root).map_err(|_| "target root unavailable")?;
        let base = self
            .manifest
            .project
            .base_commit
            .as_deref()
            .ok_or("missing base")?;
        clean_base(&root, base)?;
        let mut operations = Vec::new();
        let mut total = 0;
        for (index, decoded) in self.decoded.iter().enumerate() {
            safe_parents(&root, &decoded.path)?;
            let manifest = &self.manifest.changes.as_ref().unwrap()[index];
            let before = changes::base(&root, base, &decoded.path)
                .map_err(|_| "target base file unavailable or unsupported")?;
            let working = changes::working(&root, &decoded.path)
                .map_err(|_| "unsafe or unsupported destination path")?;
            require(
                before == working,
                "destination differs from commit base or addition collides",
            )?;
            match &before {
                Some(snapshot) => {
                    require(
                        manifest.base_sha256.as_deref()
                            == Some(hash(snapshot.content.as_bytes()).as_str())
                            && manifest.base_mode.as_deref() == Some(snapshot.mode.as_str()),
                        "base file hash or mode mismatch",
                    )?;
                    if !decoded.mode_only {
                        require(
                            decoded.before.as_ref() == Some(snapshot),
                            "patch base differs from repository",
                        )?;
                    }
                }
                None => require(decoded.before.is_none(), "patch target missing from base")?,
            }
            let mut after = decoded.after.clone();
            if decoded.mode_only {
                after.as_mut().ok_or("invalid mode-only result")?.content = before
                    .as_ref()
                    .ok_or("mode-only base absent")?
                    .content
                    .clone();
            }
            total += before.as_ref().map_or(0, |s| s.content.len())
                + after.as_ref().map_or(0, |s| s.content.len());
            require(total <= 8 * 1024 * 1024, "application code limit")?;
            if let Some(after) = &after {
                require(
                    manifest.result_sha256.as_deref()
                        == Some(hash(after.content.as_bytes()).as_str()),
                    "result hash mismatch",
                )?;
            }
            operations.push(Operation {
                path: decoded.path.clone(),
                before,
                after,
            });
        }
        clean_base(&root, base)?;
        Ok(Plan {
            root,
            base: base.into(),
            operations,
            omissions: self.manifest.omissions.len(),
            warnings: self.manifest.warnings.len(),
        })
    }
}
fn clean_base(root: &Path, base: &str) -> Result<()> {
    let observed = git::inspect(root);
    require(
        observed.project.base_commit.as_deref() == Some(base),
        "target commit differs from bundle base",
    )?;
    require(
        !observed.partial && observed.project.dirty == Some(false),
        "target must be clean and fully inspectable (no local changes or submodules)",
    )
}
// Reject case aliases, symlink parents and nested repositories, including for absent files.
fn safe_parents(root: &Path, path: &str) -> Result<()> {
    let mut parent = root.to_path_buf();
    let parts: Vec<_> = path.split('/').collect();
    for (index, part) in parts.iter().enumerate() {
        if parent.is_dir() {
            for entry in fs::read_dir(&parent).map_err(|_| "target directory unavailable")? {
                let entry = entry.map_err(|_| "target directory unavailable")?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                require(
                    !name.eq_ignore_ascii_case(part) || name == *part,
                    "case alias in target path",
                )?;
            }
        }
        parent.push(part);
        match fs::symlink_metadata(&parent) {
            Ok(meta) => {
                require(
                    !meta.file_type().is_symlink(),
                    "symlink destination rejected",
                )?;
                if index + 1 < parts.len() {
                    require(meta.is_dir(), "non-directory parent")?;
                    require(!parent.join(".git").exists(), "nested repository rejected")?;
                } else {
                    require(meta.is_file(), "destination is not a regular file")?;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("target metadata unavailable".into()),
        }
    }
    Ok(())
}
struct Operation {
    path: String,
    before: Option<Snapshot>,
    after: Option<Snapshot>,
}
pub struct Plan {
    root: PathBuf,
    base: String,
    operations: Vec<Operation>,
    omissions: usize,
    warnings: usize,
}
impl Plan {
    pub fn report(&self) -> Application {
        Application {
            checked: true,
            written: false,
            changes: self.operations.len(),
            omissions: self.omissions,
            warnings: self.warnings,
        }
    }
    /// Recheck the target. Stage privately, keep original files for rollback, never stage Git.
    pub fn write(self) -> Result<Application> {
        self.write_inner(None)
    }
    fn write_inner(self, fail_after: Option<usize>) -> Result<Application> {
        clean_base(&self.root, &self.base)?;
        for op in &self.operations {
            safe_parents(&self.root, &op.path)?;
            require(
                changes::working(&self.root, &op.path).map_err(|_| "destination unavailable")?
                    == op.before,
                "destination changed since check",
            )?;
        }
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let name = format!(
            ".memory-pier-apply-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let stage = self.root.join(&name);
        private_dir(&stage).map_err(|_| "cannot create private application staging directory")?;
        let mut backed_up = Vec::new();
        let mut installed = Vec::new();
        let mut directories = Vec::new();
        let result = (|| -> Result<()> {
            let recovery = serde_json::json!({"base_commit":self.base,"state":"application may be incomplete; original files are old-N, new files are new-N", "paths":self.operations.iter().enumerate().map(|(i,op)| serde_json::json!({"index":i,"path":op.path,"had_original":op.before.is_some(),"has_result":op.after.is_some()})).collect::<Vec<_>>()});
            write_new(
                &stage.join("recovery.json"),
                &serde_json::to_vec_pretty(&recovery)
                    .map_err(|_| "recovery serialization failed")?,
                false,
            )?;
            for (index, op) in self.operations.iter().enumerate() {
                if let Some(after) = &op.after {
                    write_new(
                        &stage.join(format!("new-{index}")),
                        after.content.as_bytes(),
                        after.mode == "100755",
                    )?;
                }
            }
            for (index, op) in self.operations.iter().enumerate() {
                if fail_after == Some(index) {
                    return Err("injected write failure".into());
                }
                safe_parents(&self.root, &op.path)?;
                require(
                    changes::working(&self.root, &op.path)
                        .map_err(|_| "destination unavailable")?
                        == op.before,
                    "destination changed during application",
                )?;
                let target = self.root.join(&op.path);
                if op.after.is_some() {
                    let mut parent = self.root.clone();
                    let parts: Vec<_> = op.path.split('/').collect();
                    for component in &parts[..parts.len() - 1] {
                        parent.push(component);
                        if !parent.exists() {
                            private_dir(&parent)
                                .map_err(|_| "cannot create destination directory")?;
                            directories.push(parent.clone());
                        }
                    }
                }
                if op.before.is_some() {
                    fs::rename(&target, stage.join(format!("old-{index}")))
                        .map_err(|_| "cannot preserve original file")?;
                    backed_up.push(index);
                }
                if op.after.is_some() {
                    // Atomic exclusive insertion: unlike rename, this never replaces a collision.
                    fs::hard_link(stage.join(format!("new-{index}")), &target)
                        .map_err(|_| "cannot install file without replacing destination")?;
                    installed.push(index);
                }
            }
            require(
                git::text(&self.root, &["rev-parse", "--verify", "HEAD^{commit}"])
                    .ok()
                    .as_deref()
                    == Some(self.base.as_str()),
                "HEAD changed during application",
            )?;
            for op in &self.operations {
                require(
                    changes::working(&self.root, &op.path).map_err(|_| "result unavailable")?
                        == op.after,
                    "result verification failed",
                )?;
            }
            Ok(())
        })();
        if result.is_err() {
            let mut restored = true;
            for index in installed.into_iter().rev() {
                let op = &self.operations[index];
                if changes::working(&self.root, &op.path).ok().as_ref() == Some(&op.after) {
                    restored &= fs::remove_file(self.root.join(&op.path)).is_ok();
                } else {
                    restored = false;
                }
            }
            for index in backed_up.into_iter().rev() {
                restored &= fs::hard_link(
                    stage.join(format!("old-{index}")),
                    self.root.join(&self.operations[index].path),
                )
                .is_ok();
            }
            for dir in directories.iter().rev() {
                let _ = fs::remove_dir(dir);
            }
            if restored {
                if fs::remove_dir_all(&stage).is_err() {
                    return Err(format!(
                        "application rolled back; cleanup required in {name}"
                    ));
                }
                return Err("application failed; original files restored".into());
            }
            return Err(format!(
                "application interrupted; recovery files retained in {name}; do not retry before reviewing"
            ));
        }
        fs::remove_dir_all(&stage)
            .map_err(|_| format!("changes applied and verified; cleanup required in {name}"))?;
        Ok(Application {
            checked: true,
            written: true,
            changes: self.operations.len(),
            omissions: self.omissions,
            warnings: self.warnings,
        })
    }
}
fn private_dir(path: &Path) -> std::io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}
fn write_new(path: &Path, bytes: &[u8], executable: bool) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(if executable { 0o700 } else { 0o600 });
    }
    let mut file = options
        .open(path)
        .map_err(|_| "cannot create staged file")?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "cannot persist staged file".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn ordinary_mid_apply_failure_restores_originals_and_removes_created_directories() {
        let root =
            std::env::temp_dir().join(format!("memory-pier-rollback-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let git = |args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(args)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            String::from_utf8(out.stdout).unwrap().trim().to_owned()
        };
        git(&["init", "-b", "synthetic"]);
        fs::write(root.join("original"), "original\n").unwrap();
        git(&["add", "."]);
        git(&[
            "-c",
            "user.name=Synthetic",
            "-c",
            "user.email=synthetic@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
            "commit",
            "-m",
            "synthetic",
        ]);
        let base = git(&["rev-parse", "HEAD"]);
        let index = fs::read(root.join(".git/index")).unwrap();
        let plan = Plan {
            root: root.clone(),
            base,
            omissions: 0,
            warnings: 0,
            operations: vec![
                Operation {
                    path: "original".into(),
                    before: Some(Snapshot {
                        content: "original\n".into(),
                        mode: "100644".into(),
                    }),
                    after: Some(Snapshot {
                        content: "modified\n".into(),
                        mode: "100644".into(),
                    }),
                },
                Operation {
                    path: "nested/new".into(),
                    before: None,
                    after: Some(Snapshot {
                        content: "new\n".into(),
                        mode: "100644".into(),
                    }),
                },
                Operation {
                    path: "another".into(),
                    before: None,
                    after: Some(Snapshot {
                        content: "never written".into(),
                        mode: "100644".into(),
                    }),
                },
            ],
        };
        let error = plan.write_inner(Some(2)).unwrap_err();
        assert!(error.contains("original files restored"));
        assert_eq!(
            fs::read_to_string(root.join("original")).unwrap(),
            "original\n"
        );
        assert!(!root.join("nested").exists());
        assert!(!root.join("another").exists());
        assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
        assert!(git(&["status", "--porcelain"]).is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
