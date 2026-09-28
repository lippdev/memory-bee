//! Explicit private local storage, single-writer lock, atomic snapshots.
use super::{Agent, Event, Session};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};
const MAX_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub version: u32,
    pub project: String,
    pub profiles: Vec<String>,
    pub sessions: Vec<Session>,
    pub selected: usize,
}
impl State {
    pub fn new(project: String, agent: Agent) -> Self {
        Self {
            version: 1,
            project: project.clone(),
            profiles: vec!["Pessoal (demo)".into(), "Trabalho (demo)".into()],
            sessions: vec![Session::new(1, agent, "Pessoal (demo)".into(), project)],
            selected: 0,
        }
    }
    pub fn current(&self) -> &Session {
        &self.sessions[self.selected]
    }
    pub fn current_mut(&mut self) -> &mut Session {
        &mut self.sessions[self.selected]
    }
    pub fn fork(&mut self, agent: Agent, profile: String, context: bool) -> Result<(), String> {
        if !self.profiles.contains(&profile) {
            return Err("Perfil desconhecido".into());
        }
        if self.current().running {
            return Err("Interrompa ou aguarde o turno antes de trocar".into());
        }
        if self.sessions.len() >= 128 {
            return Err("Limite de 128 sessões; use outra pasta de estado".into());
        }
        let id = self
            .sessions
            .iter()
            .map(|s| s.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("Limite de IDs")?;
        let mut session = Session::new(id, agent, profile, self.project.clone());
        if context {
            // A provenance reference, not a native continuation or a provider message.
            let source = self.current();
            session.record(Event::Notice { message: format!("SIMULAÇÃO: contexto selecionado da sessão {} ({} / {}). Histórico de origem preservado; nenhum harness recebeu conteúdo.", source.id, source.agent.label(), source.profile) });
            for event in &source.events {
                if let Event::User { text } | Event::Text { text } = event {
                    session.events.push(Event::Notice {
                        message: format!("Contexto importado da sessão {}: {text}", source.id),
                    });
                }
            }
        }
        self.sessions.push(session);
        self.selected = self.sessions.len() - 1;
        Ok(())
    }
    fn validate(&self, project: &str) -> Result<(), String> {
        if self.version != 1
            || self.project != project
            || self.selected >= self.sessions.len()
            || self.sessions.len() > 128
            || self.profiles.is_empty()
            || self.profiles.len() > 32
        {
            return Err("Estado incompatível, inválido ou pertencente a outro projeto".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        if self.sessions.iter().any(|s| {
            !s.simulation
                || s.native_id.is_some()
                || s.project != self.project
                || !self.profiles.contains(&s.profile)
                || !ids.insert(s.id)
                || s.id == u64::MAX
                || s.events.len() > 100_000
        }) {
            return Err("Sessão inválida no estado de simulação".into());
        }
        Ok(())
    }
}

pub struct Store {
    root: PathBuf,
    _lock: super::private::Lock,
}
impl Store {
    pub fn open(root: &Path, project: &str, agent: Agent) -> Result<(Self, State), String> {
        let root = super::private::directory(root)?;
        let lock = super::private::Lock::acquire(&root, "workspace.lock")?;
        let recovered = super::private::recover(&root, "workspace.new")?;
        let store = Self { root, _lock: lock };
        let path = store.root.join("workspace.json");
        let metadata = match fs::symlink_metadata(&path) {
            Ok(meta) => Some(meta),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
        let mut state = if let Some(meta) = metadata {
            if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_BYTES {
                return Err("Arquivo de estado inválido ou acima de 16 MiB".into());
            }
            let mut bytes = Vec::new();
            fs::File::open(&path)
                .map_err(|e| e.to_string())?
                .take(MAX_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() as u64 > MAX_BYTES {
                return Err("Estado acima do limite".into());
            }
            serde_json::from_slice::<State>(&bytes)
                .map_err(|e| format!("Estado inválido; original preservado: {e}"))?
        } else {
            State::new(project.into(), agent)
        };
        state.validate(project)?;
        if let Some(kept) = recovered {
            state.current_mut().record(Event::Notice {
                message: format!(
                    "Gravação anterior interrompida; último estado completo carregado e o arquivo parcial preservado em {}.",
                    kept.display()
                ),
            });
        }
        for session in &mut state.sessions {
            if session.running {
                session.record(Event::Interrupted);
                session.record(Event::Notice {
                    message: "Processo anterior interrompido; nenhum comando foi reenviado.".into(),
                });
            }
        }
        store.save(&state)?;
        Ok((store, state))
    }
    pub fn save(&self, state: &State) -> Result<(), String> {
        state.validate(&state.project)?;
        let bytes = serde_json::to_vec(state).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("Estado acima de 16 MiB; exporte o histórico antes de continuar".into());
        }
        super::private::write_atomic(&self.root, "workspace.new", "workspace.json", &bytes)
    }
}
