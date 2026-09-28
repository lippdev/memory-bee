//! Private state directory shared by the workspace stores: a single-writer
//! lock the operating system releases when the process dies, recovery of an
//! interrupted write, and atomic snapshots flushed before they count.
use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io::{Read, Write},
    path::{Path, PathBuf},
};

/// Creates the directory (0700) or checks an existing one; returns it canonical.
pub fn directory(root: &Path) -> Result<PathBuf, String> {
    if !root.exists() {
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(root).map_err(|e| e.to_string())?;
    }
    let meta = fs::symlink_metadata(root).map_err(|e| e.to_string())?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err("Estado exige pasta real, sem symlink".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o077 != 0 {
            return Err("Pasta de estado deve ter permissão 0700".into());
        }
    }
    root.canonicalize().map_err(|e| e.to_string())
}

fn regular_or_missing(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => Ok(true),
        Ok(_) => Err(format!(
            "{} deve ser arquivo comum, sem symlink",
            path.file_name().unwrap_or_default().to_string_lossy()
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

/// Advisory lock held while the store is open. The file stays on disk; the
/// kernel drops the lock when the holder exits or is killed, so a crash never
/// requires manual cleanup and a live process is never displaced.
pub struct Lock {
    _file: File,
}

impl Lock {
    pub fn acquire(root: &Path, name: &str) -> Result<Self, String> {
        let path = root.join(name);
        regular_or_missing(&path)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&path).map_err(|e| e.to_string())?;
        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                let mut holder = String::new();
                let _ = Read::by_ref(&mut file).take(32).read_to_string(&mut holder);
                return Err(format!(
                    "Estado em uso por outro processo Memory Bee (pid {}); nada foi alterado.",
                    holder.trim()
                ));
            }
            Err(TryLockError::Error(e)) => {
                return Err(format!("Trava do estado indisponível: {e}"));
            }
        }
        file.set_len(0)
            .and_then(|()| writeln!(file, "{}", std::process::id()))
            .map_err(|e| e.to_string())?;
        Ok(Self { _file: file })
    }
}

/// A leftover temporary file means a previous writer stopped before its
/// rename, so the committed snapshot is still the last complete state. The
/// leftover is kept beside it under a new name, never loaded or deleted.
/// Call only while holding the lock.
pub fn recover(root: &Path, temp: &str) -> Result<Option<PathBuf>, String> {
    let path = root.join(temp);
    if !regular_or_missing(&path)? {
        return Ok(None);
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let kept = root.join(format!("{temp}.recovered-{stamp}"));
    fs::rename(&path, &kept).map_err(|e| e.to_string())?;
    Ok(Some(kept))
}

/// Writes `bytes` to a new private temporary file, flushes it, renames it
/// over `target` and flushes the directory. The old snapshot stays intact
/// until the rename; on error the temporary file is removed.
pub fn write_atomic(root: &Path, temp: &str, target: &str, bytes: &[u8]) -> Result<(), String> {
    let temp_path = root.join(temp);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temp_path)
        .map_err(|_| format!("{temp} já existe ou está indisponível; original preservado"))?;
    let result = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::rename(&temp_path, root.join(target)))
        .map_err(|e| e.to_string());
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
        return result;
    }
    // The rename itself is durable only once the directory is flushed.
    #[cfg(unix)]
    File::open(root)
        .and_then(|dir| dir.sync_all())
        .map_err(|e| format!("Estado gravado, mas a pasta não confirmou a gravação: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("bee-private-{}", uuid::Uuid::new_v4()));
        directory(&root).unwrap()
    }

    #[test]
    fn lock_excludes_a_live_holder_and_survives_a_stale_file() {
        let root = temp_root();
        let lock = Lock::acquire(&root, "x.lock").unwrap();
        let busy = Lock::acquire(&root, "x.lock").err().unwrap();
        assert!(busy.contains(&std::process::id().to_string()), "{busy}");
        drop(lock);
        assert!(
            root.join("x.lock").exists(),
            "the lock file is never deleted"
        );
        // A stale file (as after a kill) does not block the next open.
        fs::write(root.join("x.lock"), "99999\n").unwrap();
        Lock::acquire(&root, "x.lock").unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    /// Blocking time of one snapshot at each store's size limit; run with
    /// `cargo test --locked -- --ignored --nocapture snapshot_latency`.
    #[test]
    #[ignore]
    fn snapshot_latency() {
        let root = temp_root();
        for (label, size) in [
            ("claude-native 64 KiB", 64 * 1024),
            ("workspace 16 MiB", 16 << 20),
        ] {
            let bytes = vec![b'x'; size];
            let mut times = Vec::new();
            for _ in 0..5 {
                let start = std::time::Instant::now();
                write_atomic(&root, "m.new", "m.json", &bytes).unwrap();
                times.push(start.elapsed().as_secs_f64() * 1000.0);
            }
            times.sort_by(f64::total_cmp);
            println!(
                "{label}: mediana {:.1} ms, máx {:.1} ms",
                times[2], times[4]
            );
        }
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn interrupted_write_is_kept_aside_and_snapshots_replace_atomically() {
        let root = temp_root();
        write_atomic(&root, "s.new", "s.json", b"old").unwrap();
        fs::write(root.join("s.new"), b"half").unwrap();
        assert!(write_atomic(&root, "s.new", "s.json", b"new").is_err());
        assert_eq!(fs::read(root.join("s.json")).unwrap(), b"old");
        let kept = recover(&root, "s.new").unwrap().unwrap();
        assert_eq!(fs::read(&kept).unwrap(), b"half");
        assert_eq!(recover(&root, "s.new").unwrap(), None);
        write_atomic(&root, "s.new", "s.json", b"new").unwrap();
        assert_eq!(fs::read(root.join("s.json")).unwrap(), b"new");
        assert!(!root.join("s.new").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(root.join("s.json"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o077, 0);
            std::os::unix::fs::symlink(root.join("s.json"), root.join("l.lock")).unwrap();
            assert!(Lock::acquire(&root, "l.lock").is_err());
        }
        fs::remove_dir_all(root).unwrap();
    }
}
