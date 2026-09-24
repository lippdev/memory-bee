//! Selected raw working-tree text relative to an immutable Git commit.
//! No Git diff/filters, shell, staging, or destination application.
use crate::git;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, io::Read, path::Path};

const FILE_LIMIT: usize = 1024 * 1024;
const TOTAL_LIMIT: usize = 8 * 1024 * 1024;
const PATH_LIMIT: usize = 64;

#[derive(Debug, Serialize)]
pub struct Change {
    pub path: String,
    pub kind: &'static str,
    pub payload: String,
    pub base_sha256: Option<String>,
    pub result_sha256: Option<String>,
    pub base_mode: Option<String>,
    pub result_mode: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct File {
    pub path: String,
    pub content: String,
}
#[derive(Debug, Default, Serialize)]
pub struct PreparedChanges {
    pub entries: Vec<Change>,
    pub patch: String,
    pub new_files: Vec<File>,
    pub omissions: Vec<String>,
    /// Full raw text scanned before patch encoding, including removed lines.
    #[serde(skip)]
    pub(crate) scan_text: Vec<(usize, String)>,
}

fn hash(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

pub fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 1024
        && !path.starts_with('/')
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b" /._-".contains(&b))
        && path
            .split('/')
            .all(|s| !s.is_empty() && s != "." && s != ".." && !s.eq_ignore_ascii_case(".git"))
}

#[derive(Clone, PartialEq)]
pub(crate) struct Snapshot {
    pub(crate) content: String,
    pub(crate) mode: String,
}
pub(crate) fn working(root: &Path, path: &str) -> Result<Option<Snapshot>, &'static str> {
    let mut current = root.to_path_buf();
    let parts: Vec<_> = path.split('/').collect();
    for (index, component) in parts.iter().enumerate() {
        current.push(component);
        let metadata = match fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("unreadable_path"),
        };
        if metadata.file_type().is_symlink() {
            return Err("symlink_unsupported");
        }
        if index + 1 < parts.len() {
            if !metadata.is_dir() {
                return Err("non_directory_parent");
            }
            if current.join(".git").exists() {
                return Err("nested_repository_unsupported");
            }
        } else {
            if !metadata.is_file() {
                return Err("non_regular_file_unsupported");
            }
            if metadata.len() > FILE_LIMIT as u64 {
                return Err("file_limit");
            }
            let mut bytes = Vec::new();
            fs::File::open(&current)
                .map_err(|_| "unreadable_file")?
                .take(FILE_LIMIT as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| "unreadable_file")?;
            if bytes.len() > FILE_LIMIT {
                return Err("file_limit");
            }
            let content = text(bytes)?;
            #[cfg(unix)]
            let executable = {
                use std::os::unix::fs::PermissionsExt;
                metadata.permissions().mode() & 0o111 != 0
            };
            #[cfg(not(unix))]
            let executable = false;
            return Ok(Some(Snapshot {
                content,
                mode: if executable { "100755" } else { "100644" }.into(),
            }));
        }
    }
    Err("invalid_path")
}
fn text(bytes: Vec<u8>) -> Result<String, &'static str> {
    if bytes.contains(&0) {
        return Err("binary_unsupported");
    }
    String::from_utf8(bytes).map_err(|_| "non_utf8_unsupported")
}
pub(crate) fn base(
    root: &Path,
    commit: &str,
    path: &str,
) -> Result<Option<Snapshot>, &'static str> {
    let (status, bytes) =
        git::run(root, &["ls-tree", "-z", commit, "--", path]).map_err(|_| "base_unavailable")?;
    if status != 0 {
        return Err("base_unavailable");
    }
    if bytes.is_empty() {
        return Ok(None);
    }
    let record = std::str::from_utf8(&bytes).map_err(|_| "base_unavailable")?;
    let (meta, name) = record
        .trim_end_matches('\0')
        .split_once('\t')
        .ok_or("base_unavailable")?;
    if name != path {
        return Err("base_unavailable");
    }
    let fields: Vec<_> = meta.split(' ').collect();
    if fields.len() != 3 || fields[1] != "blob" || !["100644", "100755"].contains(&fields[0]) {
        return Err("base_type_unsupported");
    }
    let (status, bytes) = git::run(root, &["cat-file", "blob", fields[2]])
        .map_err(|_| "base_unavailable_or_limit")?;
    if status != 0 {
        return Err("base_unavailable");
    }
    Ok(Some(Snapshot {
        content: text(bytes)?,
        mode: fields[0].into(),
    }))
}

/// Paths are literal and root-relative. Invalid selectors fail; unsupported content is omitted.
pub fn prepare(
    root_path: &Path,
    commit: &str,
    paths: &BTreeSet<String>,
) -> Result<PreparedChanges, String> {
    if paths.len() > PATH_LIMIT {
        return Err("at most 64 selected paths are supported".into());
    }
    if paths.iter().any(|p| !valid_path(p)) {
        return Err("selected paths must be safe root-relative ASCII file paths (no traversal, Git metadata or pathspecs)".into());
    }
    let root = git::text(root_path, &["rev-parse", "--show-toplevel"])
        .map_err(|_| "cannot resolve project root")?;
    let root = Path::new(&root);
    if git::text(root, &["rev-parse", "--verify", "HEAD^{commit}"])
        .ok()
        .as_deref()
        != Some(commit)
    {
        return Err("Git base changed before capture".into());
    }
    let mut result = PreparedChanges::default();
    let mut snapshots = Vec::new();
    let mut total = 0;
    for (index, path) in paths.iter().enumerate() {
        // Selector strings themselves may contain secrets; scan even when omitted.
        result.scan_text.push((index + 1, path.clone()));
        let capture = (|| -> Result<_, &'static str> {
            let (status, unmerged) = git::run(root, &["ls-files", "--unmerged", "-z", "--", path])
                .map_err(|_| "index_unavailable")?;
            if status != 0 {
                return Err("index_unavailable");
            }
            if !unmerged.is_empty() {
                return Err("conflict_unsupported");
            }
            let before = base(root, commit, path)?;
            let after = working(root, path)?;
            Ok((before, after))
        })();
        let (before, after) = match capture {
            Ok(pair) => pair,
            Err(reason) => {
                result
                    .omissions
                    .push(format!("Seleção {} omitida: {reason}.", index + 1));
                continue;
            }
        };
        if before.is_none() && after.is_none() {
            return Err(format!(
                "selected path {} does not exist in base or working tree",
                index + 1
            ));
        }
        if before == after {
            result.omissions.push(format!(
                "Seleção {} sem diferença em relação à base; nenhum payload.",
                index + 1
            ));
            continue;
        }
        let size = before.as_ref().map_or(0, |s| s.content.len())
            + after.as_ref().map_or(0, |s| s.content.len());
        if size > TOTAL_LIMIT - total {
            result
                .omissions
                .push(format!("Seleção {} omitida: total_limit.", index + 1));
            continue;
        }
        total += size;
        for snapshot in [&before, &after].into_iter().flatten() {
            result.scan_text.push((index + 1, snapshot.content.clone()));
        }
        let kind = if before.is_none() {
            "add"
        } else if after.is_none() {
            "delete"
        } else {
            "modify"
        };
        let payload = if let Some(before) = &before {
            result.patch.push_str(&patch(path, before, after.as_ref()));
            "changes.patch".into()
        } else {
            let name = format!("files/{:04}.txt", index + 1);
            result.new_files.push(File {
                path: name.clone(),
                content: after.as_ref().expect("nonempty pair").content.clone(),
            });
            name
        };
        result.entries.push(Change {
            path: path.clone(),
            kind,
            payload,
            base_sha256: before.as_ref().map(|s| hash(&s.content)),
            result_sha256: after.as_ref().map(|s| hash(&s.content)),
            base_mode: before.as_ref().map(|s| s.mode.clone()),
            result_mode: after.as_ref().map(|s| s.mode.clone()),
        });
        snapshots.push((path, after));
    }
    for (path, snapshot) in snapshots {
        if working(root, path).ok().as_ref() != Some(&snapshot) {
            return Err("selected working-tree content changed during capture".into());
        }
    }
    if git::text(root, &["rev-parse", "--verify", "HEAD^{commit}"])
        .ok()
        .as_deref()
        != Some(commit)
    {
        return Err("Git base changed during capture".into());
    }
    Ok(result)
}

// Whole-file replacement hunks avoid diff heuristics and filters; payload can be larger.
pub(crate) fn patch(path: &str, before: &Snapshot, after: Option<&Snapshot>) -> String {
    let a = format!("\"a/{path}\"");
    let b = format!("\"b/{path}\"");
    let mut patch = format!("diff --git {a} {b}\n");
    if let Some(after) = after {
        if before.mode != after.mode {
            patch.push_str(&format!(
                "old mode {}\nnew mode {}\n",
                before.mode, after.mode
            ));
        }
    } else {
        patch.push_str(&format!("deleted file mode {}\n", before.mode));
    }
    let old = before.content.as_str();
    let new = after.map_or("", |s| s.content.as_str());
    if old != new {
        patch.push_str(&format!(
            "--- {a}\n+++ {}\n",
            if after.is_some() {
                b.as_str()
            } else {
                "/dev/null"
            }
        ));
        let n = old.split_inclusive('\n').count();
        let m = new.split_inclusive('\n').count();
        patch.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            usize::from(n > 0),
            n,
            usize::from(m > 0),
            m
        ));
        for (prefix, content) in [('-', old), ('+', new)] {
            for line in content.split_inclusive('\n') {
                patch.push(prefix);
                patch.push_str(line);
                if !line.ends_with('\n') {
                    patch.push_str("\n\\ No newline at end of file\n");
                }
            }
        }
    }
    patch
}
