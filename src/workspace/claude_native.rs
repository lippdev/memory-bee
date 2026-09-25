//! Local references to Claude Code sessions started by Memory Bee.
//! Claude Code keeps the conversation and credentials; this file keeps only IDs.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

const MAX_BYTES: u64 = 64 * 1024;
const MAX_SESSIONS: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeSession {
    pub id: Uuid,
    pub exit_code: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeState {
    pub version: u32,
    pub project: String,
    pub sessions: Vec<ClaudeSession>,
}

impl ClaudeState {
    fn validate(&self, project: &str) -> Result<(), String> {
        if self.version != 1 || self.project != project || self.sessions.len() > MAX_SESSIONS {
            return Err("Estado Claude incompatível ou pertencente a outro projeto".into());
        }
        let mut ids = BTreeSet::new();
        if self.sessions.iter().any(|s| !ids.insert(s.id)) {
            return Err("IDs Claude repetidos no estado".into());
        }
        Ok(())
    }
    pub fn latest(&self) -> Option<Uuid> {
        self.sessions.last().map(|s| s.id)
    }
}

pub struct ClaudeStore {
    root: PathBuf,
}

fn private_new(path: &Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).map_err(|e| e.to_string())
}

impl ClaudeStore {
    pub fn open(root: &Path, project: &Path) -> Result<(Self, ClaudeState), String> {
        if !root.exists() {
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder.create(root).map_err(|e| e.to_string())?;
        }
        let metadata = fs::symlink_metadata(root).map_err(|e| e.to_string())?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("Estado exige pasta real, sem symlink".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err("Pasta de estado deve ter permissão 0700".into());
            }
        }
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let project = project.to_str().ok_or("Projeto deve ser UTF-8")?;
        let store = Self { root };
        let mut lock = private_new(&store.root.join("claude-native.lock"))
            .map_err(|_| "Estado Claude em uso ou trava antiga presente. Confirme que não há processo ativo antes de remover claude-native.lock.")?;
        writeln!(lock, "{}", std::process::id()).map_err(|e| e.to_string())?;
        let path = store.root.join("claude-native.json");
        let state = match fs::symlink_metadata(&path) {
            Ok(meta) => {
                if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_BYTES {
                    return Err("Arquivo de estado Claude inválido".into());
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if meta.permissions().mode() & 0o077 != 0 {
                        return Err("Arquivo de estado Claude deve ser privado (0600)".into());
                    }
                }
                let mut bytes = Vec::new();
                File::open(&path)
                    .map_err(|e| e.to_string())?
                    .take(MAX_BYTES + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|e| e.to_string())?;
                if bytes.len() as u64 > MAX_BYTES {
                    return Err("Arquivo de estado Claude excede 64 KiB".into());
                }
                serde_json::from_slice::<ClaudeState>(&bytes)
                    .map_err(|e| format!("Estado Claude inválido; original preservado: {e}"))?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => ClaudeState {
                version: 1,
                project: project.into(),
                sessions: Vec::new(),
            },
            Err(e) => return Err(e.to_string()),
        };
        state.validate(project)?;
        Ok((store, state))
    }
    pub fn start(&self, state: &mut ClaudeState, id: Uuid) -> Result<(), String> {
        if state.sessions.len() >= MAX_SESSIONS {
            return Err("Limite de 128 sessões Claude nesta pasta".into());
        }
        let mut next = state.clone();
        next.sessions.push(ClaudeSession {
            id,
            exit_code: None,
        });
        self.save(&next)?;
        *state = next;
        Ok(())
    }
    pub fn finish(&self, state: &mut ClaudeState, id: Uuid, code: u32) -> Result<(), String> {
        let mut next = state.clone();
        let session = next
            .sessions
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or("Sessão Claude não encontrada no estado")?;
        session.exit_code = Some(code);
        self.save(&next)?;
        *state = next;
        Ok(())
    }
    pub fn save(&self, state: &ClaudeState) -> Result<(), String> {
        state.validate(&state.project)?;
        let bytes = serde_json::to_vec(state).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("Estado Claude excede 64 KiB".into());
        }
        let temp = self.root.join("claude-native.new");
        let mut file =
            private_new(&temp).map_err(|_| "claude-native.new já existe; original preservado")?;
        let result = file
            .write_all(&bytes)
            .and_then(|()| file.sync_all())
            .and_then(|()| fs::rename(&temp, self.root.join("claude-native.json")));
        if let Err(e) = result {
            let _ = fs::remove_file(&temp);
            return Err(e.to_string());
        }
        Ok(())
    }
}
impl Drop for ClaudeStore {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.root.join("claude-native.lock"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_references_are_private_scoped_and_resumable() {
        let root = std::env::temp_dir().join(format!("bee-claude-{}", Uuid::new_v4()));
        let project = std::env::current_dir().unwrap().canonicalize().unwrap();
        let (store, mut state) = ClaudeStore::open(&root, &project).unwrap();
        let id = Uuid::new_v4();
        store.start(&mut state, id).unwrap();
        assert_eq!(state.latest(), Some(id));
        assert!(ClaudeStore::open(&root, &project).is_err());
        store.finish(&mut state, id, 7).unwrap();
        drop(store);
        let (store, loaded) = ClaudeStore::open(&root, &project).unwrap();
        assert_eq!(loaded.sessions[0].exit_code, Some(7));
        assert!(ClaudeStore::open(&root, Path::new("/different-project")).is_err());
        drop(store);
        fs::remove_dir_all(root).unwrap();
    }
}
