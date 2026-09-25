//! Provider-independent commands and a deterministic, model-free simulator.
use super::{Agent, Event};
use std::collections::VecDeque;

pub trait Adapter {
    fn send(&mut self, message: &str) -> Result<(), String>;
    fn poll(&mut self) -> Option<Event>;
    fn decide(&mut self, id: &str, allow: bool) -> Result<(), String>;
    fn interrupt(&mut self);
}

pub struct Demo {
    agent: Agent,
    queue: VecDeque<Event>,
    pending: Option<String>,
    turn: u64,
    active: bool,
}
impl Demo {
    pub fn new(agent: Agent) -> Self {
        Self {
            agent,
            queue: VecDeque::new(),
            pending: None,
            turn: 0,
            active: false,
        }
    }
}
impl Adapter for Demo {
    fn send(&mut self, message: &str) -> Result<(), String> {
        if self.active {
            return Err("Aguarde ou interrompa o turno atual".into());
        }
        self.turn += 1;
        self.active = true;
        self.queue.push_back(Event::Text {
            text: format!("[SIMULAÇÃO {}] Vou mostrar o fluxo ", self.agent.label()),
        });
        self.queue.push_back(Event::Text {
            text: "de leitura, alteração e permissão. Nenhum arquivo do projeto será alterado."
                .into(),
        });
        if message.contains("[erro]") {
            self.queue.push_back(Event::Error {
                message: "Falha sintética do provedor; histórico preservado.".into(),
            });
        } else {
            self.queue.push_back(Event::Tool {
                name: "Ler arquivo (simulado)".into(),
                detail: "src/example.rs".into(),
            });
            self.queue.push_back(Event::Change {
                path: "src/example.rs".into(),
                diff:
                    "--- exemplo sintético\n+++ exemplo sintético\n- todo!()\n+ buscar_por_nome()"
                        .into(),
            });
            self.queue.push_back(Event::Approval {
                id: format!("demo-{}", self.turn),
                command: "cargo test (simulado; não será executado)".into(),
            });
        }
        Ok(())
    }
    fn poll(&mut self) -> Option<Event> {
        if self.pending.is_some() {
            return None;
        }
        let event = self.queue.pop_front()?;
        if let Event::Approval { id, .. } = &event {
            self.pending = Some(id.clone());
        }
        if matches!(
            event,
            Event::Completed | Event::Interrupted | Event::Error { .. }
        ) {
            self.active = false;
        }
        Some(event)
    }
    fn decide(&mut self, id: &str, allow: bool) -> Result<(), String> {
        if self.pending.as_deref() != Some(id) {
            return Err("Permissão expirada ou desconhecida".into());
        }
        self.pending = None;
        self.queue.push_back(Event::Decision {
            id: id.into(),
            allow,
        });
        self.queue.push_back(Event::Text {
            text: if allow {
                "Permissão aceita. Teste sintético concluído."
            } else {
                "Permissão negada. Nenhum comando foi executado."
            }
            .into(),
        });
        self.queue.push_back(Event::Completed);
        Ok(())
    }
    fn interrupt(&mut self) {
        self.queue.clear();
        self.pending = None;
        if self.active {
            self.queue.push_back(Event::Interrupted);
        }
    }
}
