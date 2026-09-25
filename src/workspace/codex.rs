//! Experimental Codex App Server protocol adapter. Not enabled in the public
//! workspace: release requires both subscription-backed providers to pass QA.
use super::Event;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver, TryRecvError},
};

const MAX_FRAME: u64 = 1024 * 1024;

pub struct Transport {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<Result<Value, String>>,
}
impl Transport {
    /// Caller supplies a private per-profile home. Provider environment variables
    /// and the default home are not inherited; platform credential isolation is
    /// still unverified. No shell interprets provider commands.
    pub fn spawn(executable: &Path, profile_home: &Path, project: &Path) -> Result<Self, String> {
        let metadata = std::fs::symlink_metadata(profile_home).map_err(|e| e.to_string())?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("Profile requires a real private directory".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err("Profile requires mode 0700".into());
            }
        }
        let home = profile_home.canonicalize().map_err(|e| e.to_string())?;
        if !home.is_dir() {
            return Err("Profile home must be a directory".into());
        }
        let mut cmd = Command::new(executable);
        cmd.args(["app-server", "--listen", "stdio://"])
            .current_dir(project)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", &home)
            .env("CODEX_HOME", &home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;
        let stdin = child.stdin.take().ok_or("No provider stdin")?;
        let stdout = child.stdout.take().ok_or("No provider stdout")?;
        let (tx, messages) = mpsc::sync_channel(128);
        std::thread::spawn(move || {
            use std::io::Read;
            let mut reader = BufReader::new(stdout);
            loop {
                let mut bytes = Vec::new();
                let read = reader
                    .by_ref()
                    .take(MAX_FRAME + 1)
                    .read_until(b'\n', &mut bytes);
                let frame = match read {
                    Ok(0) => Err("Provider disconnected; commands will not be replayed".into()),
                    Ok(_) if bytes.len() as u64 > MAX_FRAME => {
                        Err("Provider frame exceeds 1 MiB".into())
                    }
                    Ok(_) => {
                        serde_json::from_slice(&bytes).map_err(|_| "Malformed provider JSON".into())
                    }
                    Err(_) => Err("Provider transport read failed".into()),
                };
                let terminal = frame.is_err();
                if tx.send(frame).is_err() || terminal {
                    break;
                }
            }
        });
        Ok(Self {
            child,
            stdin,
            messages,
        })
    }
    pub fn send(&mut self, message: &Value) -> Result<(), String> {
        let mut bytes = serde_json::to_vec(message).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_FRAME {
            return Err("Request exceeds 1 MiB".into());
        }
        bytes.push(b'\n');
        self.stdin
            .write_all(&bytes)
            .and_then(|()| self.stdin.flush())
            .map_err(|e| e.to_string())
    }
    pub fn poll(&mut self) -> Result<Option<Value>, String> {
        match self.messages.try_recv() {
            Ok(result) => result.map(Some),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err("Provider disconnected".into()),
        }
    }
}
impl Drop for Transport {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Default)]
pub struct Protocol {
    initialized: bool,
    next: u64,
    pending: BTreeMap<u64, String>,
    approvals: BTreeMap<String, Value>,
    pub thread: Option<String>,
    pub turn: Option<String>,
}
pub struct Incoming {
    pub events: Vec<Event>,
    pub replies: Vec<Value>,
}
impl Protocol {
    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        self.pending.insert(self.next, method.into());
        json!({"id":self.next,"method":method,"params":params})
    }
    pub fn initialize(&mut self) -> Value {
        self.request("initialize", json!({"clientInfo":{"name":"memory_bee","title":"Memory Bee","version":env!("CARGO_PKG_VERSION")}}))
    }
    pub fn account(&mut self) -> Result<Value, String> {
        self.ready()?;
        Ok(self.request("account/read", json!({"refreshToken":false})))
    }
    pub fn login(&mut self) -> Result<Value, String> {
        self.ready()?;
        Ok(self.request("account/login/start", json!({"type":"chatgpt"})))
    }
    fn ready(&self) -> Result<(), String> {
        if self.initialized {
            Ok(())
        } else {
            Err("Initialize must succeed before requests".into())
        }
    }
    pub fn start(&mut self, project: &Path, resume: Option<&str>) -> Result<Value, String> {
        self.ready()?;
        if self.turn.is_some()
            || self
                .pending
                .values()
                .any(|m| matches!(m.as_str(), "turn/start" | "thread/start" | "thread/resume"))
        {
            return Err("Turn or thread request is active".into());
        }
        let cwd = project.to_str().ok_or("Project must be UTF-8")?;
        Ok(if let Some(id) = resume {
            self.request("thread/resume", json!({"threadId":id,"cwd":cwd}))
        } else {
            self.request(
                "thread/start",
                json!({"cwd":cwd,"approvalPolicy":"on-request","sandbox":"workspace-write"}),
            )
        })
    }
    pub fn send(&mut self, text: &str) -> Result<Value, String> {
        self.ready()?;
        if self.turn.is_some()
            || self
                .pending
                .values()
                .any(|m| matches!(m.as_str(), "turn/start" | "thread/start" | "thread/resume"))
        {
            return Err("Turn is active".into());
        }
        let thread = self.thread.clone().ok_or("No thread")?;
        Ok(self.request(
            "turn/start",
            json!({"threadId":thread,"input":[{"type":"text","text":text}]}),
        ))
    }
    pub fn interrupt(&mut self) -> Result<Value, String> {
        let thread = self.thread.clone().ok_or("No thread")?;
        let turn = self.turn.clone().ok_or("No turn")?;
        Ok(self.request("turn/interrupt", json!({"threadId":thread,"turnId":turn})))
    }
    pub fn decide(&mut self, key: &str, allow: bool) -> Result<Value, String> {
        let id = self
            .approvals
            .remove(key)
            .ok_or("Approval expired or unknown")?;
        Ok(json!({"id":id,"result":{"decision":if allow {"accept"} else {"decline"}}}))
    }
    pub fn receive(&mut self, value: Value) -> Incoming {
        let mut out = Incoming {
            events: Vec::new(),
            replies: Vec::new(),
        };
        if let Some(method) = value.get("method").and_then(Value::as_str) {
            let params = &value["params"];
            if let Some(id) = value.get("id") {
                let key = id.to_string();
                let supported = matches!(
                    method,
                    "item/commandExecution/requestApproval" | "item/fileChange/requestApproval"
                );
                // Approval context must match the active thread and turn.
                if supported
                    && params["threadId"].as_str() == self.thread.as_deref()
                    && params["turnId"].as_str() == self.turn.as_deref()
                    && self.turn.is_some()
                    && !self.approvals.contains_key(&key)
                    && params.get("availableDecisions").is_none_or(|choices| {
                        choices.is_null()
                            || choices.as_array().is_some_and(|choices| {
                                choices.contains(&json!("accept"))
                                    && choices.contains(&json!("decline"))
                            })
                    })
                {
                    self.approvals.insert(key.clone(), id.clone());
                    out.events.push(Event::Approval {
                        id: key,
                        command: params.to_string(),
                    });
                } else {
                    out.replies.push(json!({"id":id,"error":{"code":-32601,"message":"Unsupported or stale request; permission not granted"}}));
                    out.events.push(Event::Error {
                        message: format!("Unsupported or stale provider request: {method}"),
                    });
                }
                return out;
            }
            if params
                .get("threadId")
                .and_then(Value::as_str)
                .is_some_and(|t| Some(t) != self.thread.as_deref())
            {
                return out;
            }
            let scoped = matches!(
                method,
                "turn/started"
                    | "turn/completed"
                    | "item/agentMessage/delta"
                    | "item/started"
                    | "item/completed"
                    | "serverRequest/resolved"
            );
            if scoped
                && (self.thread.is_none() || params["threadId"].as_str() != self.thread.as_deref())
            {
                return out;
            }
            let event_turn = if method.starts_with("turn/") {
                params["turn"]["id"].as_str()
            } else {
                params["turnId"].as_str()
            };
            if scoped
                && method != "serverRequest/resolved"
                && method != "turn/started"
                && (self.turn.is_none() || event_turn != self.turn.as_deref())
            {
                return out;
            }
            match method {
                "serverRequest/resolved" => {
                    self.approvals.remove(&params["requestId"].to_string());
                }
                "turn/started" => {
                    if self.pending.values().any(|m| m == "turn/start")
                        || self.turn.as_deref() == event_turn
                    {
                        self.turn = event_turn.map(str::to_owned);
                    }
                }
                "item/agentMessage/delta" => {
                    if let Some(text) = params["delta"].as_str() {
                        out.events.push(Event::Text { text: text.into() });
                    }
                }
                "item/started" | "item/completed" => {
                    let item = &params["item"];
                    match item["type"].as_str() {
                        Some("commandExecution") => out.events.push(Event::Tool {
                            name: method.into(),
                            detail: item.to_string(),
                        }),
                        Some("fileChange") => out.events.push(Event::Change {
                            path: "provider changes".into(),
                            diff: item.to_string(),
                        }),
                        Some("userMessage" | "agentMessage") => {}
                        _ => out.events.push(Event::Notice {
                            message: format!("Provider item not rendered: {}", item["type"]),
                        }),
                    }
                }
                "turn/completed" => {
                    self.turn = None;
                    self.approvals.clear();
                    out.events.push(match params["turn"]["status"].as_str() {
                        Some("completed") => Event::Completed,
                        Some("interrupted") => Event::Interrupted,
                        _ => Event::Error {
                            message: "Provider turn failed; inspect native history".into(),
                        },
                    });
                }
                "error" => out.events.push(Event::Error {
                    message: "Provider error; history preserved".into(),
                }),
                _ => out.events.push(Event::Notice {
                    message: format!("Provider notification: {method}"),
                }),
            }
        } else if let Some(id) = value["id"].as_u64() {
            if let Some(method) = self.pending.remove(&id) {
                if value.get("error").is_some() {
                    out.events.push(Event::Error {
                        message: format!("Provider rejected {method}; no automatic retry"),
                    });
                } else if let Some(result) = value.get("result") {
                    match method.as_str() {
                        "initialize" => { self.initialized = true; out.replies.push(json!({"method":"initialized","params":{}})); }
                        "thread/start" | "thread/resume" => self.thread = result["thread"]["id"].as_str().map(str::to_owned),
                        "turn/start" => self.turn = result["turn"]["id"].as_str().map(str::to_owned),
                        // Authentication payloads (including URLs) stay out of history.
                        "account/read" | "account/login/start" => out.events.push(Event::Notice { message: format!("{method} completed; authentication data is not conversation history") }),
                        _ => {}
                    }
                } else {
                    out.events.push(Event::Error {
                        message: "Provider response missing result".into(),
                    });
                }
            }
        } else {
            out.events.push(Event::Error {
                message: "Unrecognized provider envelope".into(),
            });
        }
        out
    }
}
