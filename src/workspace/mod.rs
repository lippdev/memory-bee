//! Unified workspace contracts. Demo and native providers are never conflated.
pub mod adapter;
pub mod codex;
pub mod store;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Agent {
    Claude,
    Codex,
}
impl Agent {
    pub fn label(self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Codex => "Codex",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "claude" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    User { text: String },
    Text { text: String },
    Tool { name: String, detail: String },
    Change { path: String, diff: String },
    Approval { id: String, command: String },
    Decision { id: String, allow: bool },
    Completed,
    Interrupted,
    Error { message: String },
    Notice { message: String },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub id: u64,
    pub agent: Agent,
    pub profile: String,
    pub project: String,
    pub native_id: Option<String>,
    pub simulation: bool,
    pub running: bool,
    pub events: Vec<Event>,
}
impl Session {
    pub fn new(id: u64, agent: Agent, profile: String, project: String) -> Self {
        Self {
            id,
            agent,
            profile,
            project,
            native_id: None,
            simulation: true,
            running: false,
            events: Vec::new(),
        }
    }
    pub fn record(&mut self, event: Event) {
        match event {
            Event::User { .. } => self.running = true,
            Event::Completed | Event::Interrupted | Event::Error { .. } => self.running = false,
            _ => {}
        }
        self.events.push(event);
    }
}

/// Both providers must be validated with subscriptions before public live mode.
pub fn live_gate() -> Result<(), String> {
    Err("Programação real indisponível: Claude e Codex com assinaturas precisam ser validados juntos. Use --demo para a simulação; nenhuma API paga é usada como alternativa.".into())
}
